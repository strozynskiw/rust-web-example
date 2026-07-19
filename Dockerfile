FROM rust:1.97-bookworm AS builder

WORKDIR /usr/src/service
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim AS runtime
# Install OpenSSL runtime libraries (OpenSSL 3.x for bookworm)
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/service

COPY --from=builder /usr/src/service/target/release/web-template .
COPY static static
COPY templates templates

CMD ["./web-template"]
