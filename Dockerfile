# syntax=docker/dockerfile:1.7
FROM rust:1.85-bookworm AS build
WORKDIR /source
COPY . .
RUN cargo build --locked --release -p lambda-safe-service -p lambda-safe-cli

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install --yes --no-install-recommends ca-certificates curl z3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --system --gid 10001 lambda-safe \
    && useradd --system --uid 10001 --gid lambda-safe --home-dir /nonexistent lambda-safe
COPY --from=build /source/target/release/lambda-safe-service /usr/local/bin/lambda-safe-service
COPY --from=build /source/target/release/lambda-safe /usr/local/bin/lambda-safe
USER 10001:10001
EXPOSE 8080
ENV LAMBDA_SAFE_BIND=0.0.0.0:8080 \
    LAMBDA_SAFE_API_TOKEN_FILE=/run/secrets/lambda-safe-api-token \
    LAMBDA_SAFE_SOLVER=z3 \
    LAMBDA_SAFE_Z3_PATH=/usr/bin/z3 \
    RUST_LOG=info
ENTRYPOINT ["/usr/local/bin/lambda-safe-service"]
