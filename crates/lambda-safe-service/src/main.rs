use axum::body::{Body, Bytes};
use axum::extract::{DefaultBodyLimit, State};
use axum::http::{
    HeaderMap, Request, StatusCode,
    header::{AUTHORIZATION, CONTENT_TYPE, WWW_AUTHENTICATE},
};
use axum::middleware::{Next, from_fn_with_state};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use lambda_safe_compiler::{Compiler, CompilerConfig, SourceFormat};
use lambda_safe_emit::signing_key_from_hex;
use lambda_safe_model::{CompileStatus, HardManifest, LambdaCertificate};
use lambda_safe_solver::{DecisionProcedure, FiniteModelSolver, Z3ProcessSolver};
use lambda_safe_syntax::deserialize_json_strict;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

#[derive(Clone, Debug)]
struct AppState {
    compiler: Arc<Compiler>,
    work_permits: Arc<Semaphore>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ApiSourceFormat {
    Auto,
    Lambda,
    Json,
}

impl From<ApiSourceFormat> for SourceFormat {
    fn from(value: ApiSourceFormat) -> Self {
        match value {
            ApiSourceFormat::Auto => Self::Auto,
            ApiSourceFormat::Lambda => Self::Lambda,
            ApiSourceFormat::Json => Self::Json,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceEnvelope {
    format: ApiSourceFormat,
    content: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CompileRequest {
    source: SourceEnvelope,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VerifyRequest {
    manifest: HardManifest,
    certificate: LambdaCertificate,
}

#[derive(Clone, Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    service: &'static str,
    version: &'static str,
}

#[derive(Clone, Debug, Serialize)]
struct ErrorResponse {
    code: &'static str,
    message: String,
}

#[tokio::main]
async fn main() {
    if let Err(message) = run().await {
        error!(error = %message, "service terminated");
        eprintln!("lambda-safe-service: {message}");
        std::process::exit(70);
    }
}

async fn run() -> Result<(), String> {
    initialize_tracing();
    let settings = Settings::from_environment()?;
    let api_token = read_api_token(settings.api_token_file.as_deref())?;
    ensure_api_token_for_bind(settings.bind, api_token.is_some())?;
    let solver = settings.create_solver()?;
    let mut config = CompilerConfig {
        max_source_bytes: settings.max_source_bytes,
        max_solver_obligations: settings.max_solver_obligations,
        solver_timeout: Duration::from_millis(settings.timeout_ms),
        max_compile_time: Duration::from_millis(settings.max_compile_ms),
        ..CompilerConfig::default()
    };
    if let Some(path) = &settings.signing_key_file {
        let value = fs::read_to_string(path)
            .map_err(|error| format!("cannot read signing key {}: {error}", path.display()))?;
        config.signing_key = Some(
            signing_key_from_hex(value.trim())
                .map_err(|error| format!("invalid signing key: {error}"))?,
        );
    }
    let compiler = Arc::new(Compiler::new(solver, config));
    let app = application(
        compiler,
        settings.max_body_bytes,
        settings.max_concurrent_requests,
        api_token,
    );
    let listener = TcpListener::bind(settings.bind)
        .await
        .map_err(|error| format!("cannot bind {}: {error}", settings.bind))?;
    info!(address = %settings.bind, "λ-Safe service listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| format!("HTTP server failure: {error}"))
}

fn application(
    compiler: Arc<Compiler>,
    max_body_bytes: usize,
    max_concurrent_requests: usize,
    api_token: Option<Arc<[u8]>>,
) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .route("/readyz", get(health))
        .route("/v1/info", get(info_handler))
        .route("/v1/check", post(check_handler))
        .route("/v1/compile", post(compile_handler))
        .route("/v1/certificates/verify", post(verify_handler))
        .layer(DefaultBodyLimit::max(max_body_bytes))
        .layer(CatchPanicLayer::new())
        .layer(TraceLayer::new_for_http())
        .route_layer(from_fn_with_state(api_token, require_api_token))
        .with_state(AppState {
            compiler,
            work_permits: Arc::new(Semaphore::new(max_concurrent_requests)),
        })
}

async fn require_api_token(
    State(expected_token): State<Option<Arc<[u8]>>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if matches!(request.uri().path(), "/healthz" | "/readyz") {
        return next.run(request).await;
    }
    let Some(expected_token) = expected_token else {
        return next.run(request).await;
    };
    let supplied_token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            let (scheme, token) = value.split_once(' ')?;
            scheme.eq_ignore_ascii_case("Bearer").then_some(token)
        });
    let is_valid = supplied_token
        .is_some_and(|value| bool::from(value.as_bytes().ct_eq(expected_token.as_ref())));
    if is_valid {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(WWW_AUTHENTICATE, "Bearer")],
            Json(ErrorResponse {
                code: "LSC-I004",
                message: "a valid bearer token is required".to_owned(),
            }),
        )
            .into_response()
    }
}

fn read_api_token(path: Option<&std::path::Path>) -> Result<Option<Arc<[u8]>>, String> {
    let Some(path) = path else {
        return Ok(None);
    };
    let mut token = fs::read(path)
        .map_err(|error| format!("cannot read API token file {}: {error}", path.display()))?;
    while matches!(token.last(), Some(b'\n' | b'\r')) {
        token.pop();
    }
    if token.len() < 32 || !token.iter().all(u8::is_ascii_graphic) {
        return Err(format!(
            "API token file {} must contain at least 32 visible ASCII characters",
            path.display()
        ));
    }
    Ok(Some(Arc::from(token)))
}

fn ensure_api_token_for_bind(bind: SocketAddr, has_api_token: bool) -> Result<(), String> {
    if !bind.ip().is_loopback() && !has_api_token {
        return Err(
            "LAMBDA_SAFE_API_TOKEN_FILE is required when LAMBDA_SAFE_BIND is not a loopback address"
                .to_owned(),
        );
    }
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        service: "lambda-safe-service",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn info_handler(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.compiler.info())
}

async fn check_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> axum::response::Response {
    let permit = match acquire_work_permit(&state) {
        Ok(permit) => permit,
        Err(response) => return *response,
    };
    let request: CompileRequest = match decode_json_request(&headers, &body) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    let compiler = state.compiler.clone();
    let format: SourceFormat = request.source.format.into();
    let content = request.source.content;
    match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        compiler.check(&content, format)
    })
    .await
    {
        Ok(result) => {
            let status = if result.valid {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            (status, Json(result)).into_response()
        }
        Err(error) => internal_error(error),
    }
}

async fn compile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> axum::response::Response {
    let permit = match acquire_work_permit(&state) {
        Ok(permit) => permit,
        Err(response) => return *response,
    };
    let request: CompileRequest = match decode_json_request(&headers, &body) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    let compiler = state.compiler.clone();
    let format: SourceFormat = request.source.format.into();
    let content = request.source.content;
    match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        compiler.compile(&content, format)
    })
    .await
    {
        Ok(result) => (http_status(result.status), Json(result)).into_response(),
        Err(error) => internal_error(error),
    }
}

async fn verify_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> axum::response::Response {
    let permit = match acquire_work_permit(&state) {
        Ok(permit) => permit,
        Err(response) => return *response,
    };
    let request: VerifyRequest = match decode_json_request(&headers, &body) {
        Ok(request) => request,
        Err(response) => return *response,
    };
    let compiler = state.compiler.clone();
    match tokio::task::spawn_blocking(move || {
        let _permit = permit;
        compiler.verify(&request.manifest, &request.certificate)
    })
    .await
    {
        Ok(result) => {
            let status = if result.valid {
                StatusCode::OK
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            (status, Json(result)).into_response()
        }
        Err(error) => internal_error(error),
    }
}

fn decode_json_request<T>(
    headers: &HeaderMap,
    body: &Bytes,
) -> Result<T, Box<axum::response::Response>>
where
    T: DeserializeOwned,
{
    let content_type_is_json = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !content_type_is_json {
        return Err(Box::new(request_error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "request content type must be application/json".to_owned(),
        )));
    }
    let source = std::str::from_utf8(body).map_err(|error| {
        Box::new(request_error(
            StatusCode::BAD_REQUEST,
            format!("request body is not UTF-8 JSON: {error}"),
        ))
    })?;
    deserialize_json_strict(source).map_err(|error| {
        Box::new(request_error(
            StatusCode::BAD_REQUEST,
            format!("invalid or ambiguous JSON request: {error}"),
        ))
    })
}

fn request_error(status: StatusCode, message: String) -> axum::response::Response {
    (
        status,
        Json(ErrorResponse {
            code: "LSC-I003",
            message,
        }),
    )
        .into_response()
}

fn acquire_work_permit(
    state: &AppState,
) -> Result<OwnedSemaphorePermit, Box<axum::response::Response>> {
    state
        .work_permits
        .clone()
        .try_acquire_owned()
        .map_err(|error| {
            error!(%error, "compiler work capacity unavailable");
            Box::new(
                (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(ErrorResponse {
                        code: "LSC-I002",
                        message: "compiler work capacity is unavailable".to_owned(),
                    }),
                )
                    .into_response(),
            )
        })
}

fn internal_error(error: tokio::task::JoinError) -> axum::response::Response {
    error!(%error, "blocking compiler task failed");
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            code: "LSC-I001",
            message: "compiler execution failed".to_owned(),
        }),
    )
        .into_response()
}

fn http_status(status: CompileStatus) -> StatusCode {
    match status {
        CompileStatus::StaticallyCertified => StatusCode::OK,
        CompileStatus::InvariantViolated | CompileStatus::InfeasibleContext => StatusCode::CONFLICT,
        CompileStatus::MalformedContract
        | CompileStatus::Unsupported
        | CompileStatus::AuthorityViolation => StatusCode::UNPROCESSABLE_ENTITY,
        CompileStatus::Indeterminate => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! {
                    result = tokio::signal::ctrl_c() => {
                        if let Err(error) = result {
                            error!(%error, "failed to wait for interrupt signal");
                        }
                    }
                    _ = terminate.recv() => {}
                }
            }
            Err(error) => {
                error!(%error, "failed to install termination signal handler");
                if let Err(error) = tokio::signal::ctrl_c().await {
                    error!(%error, "failed to wait for interrupt signal");
                }
            }
        }
    }
    #[cfg(not(unix))]
    if let Err(error) = tokio::signal::ctrl_c().await {
        error!(%error, "failed to wait for interrupt signal");
    }
}

fn initialize_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .try_init();
}

#[derive(Clone, Debug)]
struct Settings {
    bind: SocketAddr,
    solver: String,
    z3_path: PathBuf,
    timeout_ms: u64,
    max_compile_ms: u64,
    max_source_bytes: usize,
    max_body_bytes: usize,
    max_solver_obligations: usize,
    max_concurrent_requests: usize,
    signing_key_file: Option<PathBuf>,
    api_token_file: Option<PathBuf>,
}

impl Settings {
    fn from_environment() -> Result<Self, String> {
        let settings = Self {
            bind: value("LAMBDA_SAFE_BIND", "127.0.0.1:8080")
                .parse()
                .map_err(|error| format!("invalid LAMBDA_SAFE_BIND: {error}"))?,
            solver: value("LAMBDA_SAFE_SOLVER", "z3"),
            z3_path: PathBuf::from(value("LAMBDA_SAFE_Z3_PATH", "z3")),
            timeout_ms: parse_value("LAMBDA_SAFE_TIMEOUT_MS", 5_000)?,
            max_compile_ms: parse_value("LAMBDA_SAFE_MAX_COMPILE_MS", 30_000)?,
            max_source_bytes: parse_value("LAMBDA_SAFE_MAX_SOURCE_BYTES", 1_048_576)?,
            max_body_bytes: parse_value("LAMBDA_SAFE_MAX_BODY_BYTES", 1_200_000)?,
            max_solver_obligations: parse_value("LAMBDA_SAFE_MAX_SOLVER_OBLIGATIONS", 2_048)?,
            max_concurrent_requests: parse_value("LAMBDA_SAFE_MAX_CONCURRENT_REQUESTS", 32)?,
            signing_key_file: env::var_os("LAMBDA_SAFE_SIGNING_KEY_FILE").map(PathBuf::from),
            api_token_file: env::var_os("LAMBDA_SAFE_API_TOKEN_FILE").map(PathBuf::from),
        };
        if settings.max_concurrent_requests == 0 {
            return Err("LAMBDA_SAFE_MAX_CONCURRENT_REQUESTS must be greater than zero".to_owned());
        }
        Ok(settings)
    }

    fn create_solver(&self) -> Result<Arc<dyn DecisionProcedure>, String> {
        match self.solver.as_str() {
            "z3" => Z3ProcessSolver::discover(self.z3_path.clone())
                .map(|solver| Arc::new(solver) as Arc<dyn DecisionProcedure>)
                .map_err(|error| error.to_string()),
            "finite" => Ok(Arc::new(FiniteModelSolver::default())),
            other => Err(format!(
                "unsupported LAMBDA_SAFE_SOLVER `{other}`; expected `z3` or `finite`"
            )),
        }
    }
}

fn value(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_owned())
}

fn parse_value<T>(name: &str, default: T) -> Result<T, String>
where
    T: std::str::FromStr + ToString,
    T::Err: std::fmt::Display,
{
    value(name, &default.to_string())
        .parse()
        .map_err(|error| format!("invalid {name}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_app() -> Router {
        let compiler = Arc::new(Compiler::new(
            Arc::new(FiniteModelSolver::default()),
            CompilerConfig::default(),
        ));
        application(compiler, 1_200_000, 32, None)
    }

    #[tokio::test]
    async fn health_endpoint_is_available() {
        let response = test_app()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn compile_endpoint_certifies_valid_contract() {
        let contract = r#"lambda_safe "0.1";
contract api version "1" {
  unit count;
  input x: int<count> authority control_plane range [0, 2];
  assume bounded hard: x <= x;
  invariant bounded_again hard: x <= x;
}"#;
        let body = serde_json::json!({
            "source": {"format": "lambda", "content": contract}
        });
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/compile")
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn duplicate_json_envelope_members_are_rejected() {
        let body = r#"{
          "source":{"format":"lambda","content":"first"},
          "source":{"format":"lambda","content":"second"}
        }"#;
        let response = test_app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/check")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn api_routes_require_configured_bearer_token() {
        let compiler = Arc::new(Compiler::new(
            Arc::new(FiniteModelSolver::default()),
            CompilerConfig::default(),
        ));
        let token: Arc<[u8]> = Arc::from(&b"test-token-with-at-least-32-bytes"[..]);
        let app = application(compiler, 1_200_000, 32, Some(token));

        let unauthenticated = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/v1/info")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(health.status(), StatusCode::OK);

        let authenticated = app
            .oneshot(
                Request::builder()
                    .uri("/v1/info")
                    .header(AUTHORIZATION, "bEaReR test-token-with-at-least-32-bytes")
                    .body(Body::empty())
                    .expect("request should build"),
            )
            .await
            .expect("router should respond");
        assert_eq!(authenticated.status(), StatusCode::OK);
    }

    #[test]
    fn non_loopback_bind_requires_api_token() {
        let loopback = "127.0.0.1:8080".parse().expect("valid socket address");
        let wildcard = "0.0.0.0:8080".parse().expect("valid socket address");
        assert!(ensure_api_token_for_bind(loopback, false).is_ok());
        assert!(ensure_api_token_for_bind(wildcard, false).is_err());
        assert!(ensure_api_token_for_bind(wildcard, true).is_ok());
    }
}
