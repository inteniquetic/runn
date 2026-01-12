FROM rust:1.80-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY core/Cargo.toml core/Cargo.toml
COPY runn/Cargo.toml runn/Cargo.toml

RUN cargo fetch

COPY core/src core/src
COPY runn/src runn/src

RUN cargo build --release -p runn

FROM debian:bookworm-slim

RUN useradd --create-home --uid 10001 appuser

COPY --from=builder /app/target/release/runn /usr/local/bin/runn

USER appuser
EXPOSE 8080

ENTRYPOINT ["/usr/local/bin/runn"]
