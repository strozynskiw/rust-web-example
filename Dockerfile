FROM rust:1.97-bookworm AS builder

WORKDIR /usr/src/service
COPY . .
RUN cargo build --release --locked --bins

FROM debian:bookworm-slim AS runtime
# Install OpenSSL runtime libraries (OpenSSL 3.x for bookworm)
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/service

COPY --from=builder /usr/src/service/target/release/web-template .
COPY --from=builder /usr/src/service/target/release/create_admin .
COPY static static
COPY templates templates

RUN useradd --uid 10001 --create-home app && mkdir /data && chown app:app /data
USER app
ENV HOST=0.0.0.0 DATABASE_URL=sqlite:/data/data.db
EXPOSE 3000

CMD ["./web-template"]
