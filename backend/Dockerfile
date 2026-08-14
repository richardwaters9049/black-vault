FROM rust:1.88-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY .sqlx ./.sqlx

COPY src ./src
COPY migrations ./migrations

ENV SQLX_OFFLINE=true

RUN cargo build --release


FROM gcr.io/distroless/cc-debian12:nonroot AS runtime

WORKDIR /app

COPY --from=builder /app/target/release/black_vault_api /app/black_vault_api
COPY --from=builder /app/migrations /app/migrations

EXPOSE 8080

ENTRYPOINT ["/app/black_vault_api"]