FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/webxash3d-proxy /usr/local/bin/

EXPOSE 27016

ENTRYPOINT ["webxash3d-proxy"]
