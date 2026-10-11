# HTTP API

The service is a stateless adapter over the public Rust `Compiler` facade.

## Configuration

| Environment variable | Default | Meaning |
|---|---|---|
| `LAMBDA_SAFE_BIND` | `127.0.0.1:8080` | Listen address |
| `LAMBDA_SAFE_SOLVER` | `z3` | `z3` or exact bounded `finite` |
| `LAMBDA_SAFE_Z3_PATH` | `z3` | Z3 executable |
| `LAMBDA_SAFE_TIMEOUT_MS` | `5000` | Deadline per solver obligation |
| `LAMBDA_SAFE_MAX_COMPILE_MS` | `30000` | Wall-clock budget for one complete compilation |
| `LAMBDA_SAFE_MAX_SOURCE_BYTES` | `1048576` | Compiler source limit |
| `LAMBDA_SAFE_MAX_BODY_BYTES` | `1200000` | HTTP body limit |
| `LAMBDA_SAFE_MAX_SOLVER_OBLIGATIONS` | `2048` | Maximum generated obligations per compile |
| `LAMBDA_SAFE_MAX_CONCURRENT_REQUESTS` | `32` | In-process concurrent request limit |
| `LAMBDA_SAFE_SIGNING_KEY_FILE` | unset | Optional hex Ed25519 secret-key file |
| `LAMBDA_SAFE_API_TOKEN_FILE` | unset | Bearer token file required for non-loopback binds |
| `RUST_LOG` | `info` | Tracing filter |

The service discovers Z3 before binding. Failure to discover the configured
binary prevents startup rather than accepting traffic that cannot certify.

The default loopback bind keeps an unauthenticated local instance private. A
non-loopback bind requires `LAMBDA_SAFE_API_TOKEN_FILE`; its file must contain
at least 32 visible ASCII characters. All `/v1/*` routes require that token in
an `Authorization: Bearer ...` header. Health and readiness routes remain
unauthenticated for probes. Keep the token file readable only by the service
account, terminate TLS at a trusted reverse proxy, and restrict network access
to that proxy. Do not expose the service directly to the public internet.

The container image binds to `0.0.0.0` inside the container and requires a
token at `/run/secrets/lambda-safe-api-token` before it starts. Provide that
file through the container platform's secret store, make it readable by UID
10001, and keep the mount read-only. The default bare-metal service bind is
loopback; use a trusted proxy for TLS and any additional rate limiting or
tenant-specific authorization.

For a local secret file, generate a high-entropy token outside the repository
with `openssl rand -hex 32` and mount it read-only at the configured path.

## Health and readiness

`GET /healthz` and `GET /readyz` return:

```json
{"status":"ok","service":"lambda-safe-service","version":"0.1.0"}
```

Because dependencies are established before listener startup, readiness does
not perform a solver process on every probe.

## Information

`GET /v1/info` returns compiler, language, manifest, certificate and solver
identity.

## Check

`POST /v1/check` accepts:

```json
{
  "source": {
    "format": "lambda",
    "content": "lambda_safe \"0.1\"; ..."
  }
}
```

`format` is `auto`, `lambda` or `json`. Check performs parsing and all static
gates but does not invoke the solver or emit artifacts.

## Compile

`POST /v1/compile` accepts the same request. Response fields are:

- `status`;
- stable `diagnostics`;
- ordered solver `verification` records;
- `excluded_claims` outside hard certification;
- `artifacts` only for `STATICALLY_CERTIFIED`.

The JSON request and response schemas are in `schemas/`.
Request envelopes and nested artifact maps reject duplicate JSON member names.

## Verify

`POST /v1/certificates/verify` accepts:

```json
{
  "manifest": {"schema_version":"0.1"},
  "certificate": {"body":{},"certificate_digest":"..."}
}
```

The abbreviated values above show shape only; complete objects must conform to
their schemas. Verification does not invoke the solver.

## Status codes

| HTTP | Meaning |
|---:|---|
| 200 | Check passed, compilation certified, or artifact valid |
| 400 | Malformed JSON envelope |
| 409 | Invariant counterexample or infeasible assumption context |
| 413 | Axum body-limit rejection |
| 415 | Unsupported request content type |
| 422 | Malformed, unsupported, authority-invalid or invalid artifact |
| 500 | Blocking compiler worker failed |
| 503 | Solver/resource indeterminate outcome |

## Operational characteristics

- No persistent storage.
- No tenant model, billing, remote callbacks or dynamic plugins.
- JSON logs through `tracing`.
- Graceful `SIGINT`/termination handling through Tokio.
- Compile/check/verify work uses a fail-fast semaphore. A permit remains owned
  by the blocking task after client cancellation, so disconnected requests
  cannot bypass the configured work limit; saturation returns 503.
- Requests may execute several solver processes; capacity planning must use the
  number of invariants and transition obligations, not only request rate.
