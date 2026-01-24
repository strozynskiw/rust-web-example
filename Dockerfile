FROM rust:1.92-bookworm

# Install OpenSSL runtime libraries (OpenSSL 3.x for bookworm)
RUN apt-get update && apt-get install -y \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/service
COPY target/release/web-template .
COPY static static
COPY templates templates

CMD ["./web-template"]
