# e-cat example Dockerfile (multi-stage)
# NOTE: This image runs the helloworld example service. For production,
# replace the helloworld build/copy below with your own service binary.
# The workspace builds with edition 2024, so the builder image must be >= 1.85.
FROM rust:1.85-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p ecat-cli -p helloworld

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
# The CLI binary is named `ecat` (from ecat-cli), not `ecat-app`.
COPY --from=builder /app/target/release/ecat /usr/local/bin/ecat
# Example service binary; swap for your own service in production.
COPY --from=builder /app/target/release/helloworld /usr/local/bin/helloworld
EXPOSE 8000 9000
HEALTHCHECK --interval=10s --timeout=3s CMD curl -f http://localhost:8000/health || exit 1
CMD ["/usr/local/bin/helloworld"]
