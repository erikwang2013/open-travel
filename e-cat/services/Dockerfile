# open-travel 业务服务镜像（user-service / booking-service / admin-service）
# 构建上下文必须为仓库根（e-cat workspace 在 ./e-cat）
# 用法：docker build -f e-cat/services/Dockerfile -t travel-services .
#       docker run travel-services user-service   （或 booking-service）

FROM rust:1.88-slim AS build
WORKDIR /build
# protobuf-compiler: tonic 构建期 protoc；g++/make: rdkafka 编译 librdkafka；
# cmake/libssl-dev/libsasl2-dev/libzstd-dev: TLS/SASL/zstd 支持（gssapi feature 链接系统 sasl）
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler cmake g++ make pkg-config libssl-dev libsasl2-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*
COPY e-cat ./e-cat
RUN cargo build --release --manifest-path e-cat/Cargo.toml -p user-service -p booking-service -p admin-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libsasl2-2 libzstd1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/e-cat/target/release/user-service /usr/local/bin/user-service
COPY --from=build /build/e-cat/target/release/booking-service /usr/local/bin/booking-service
COPY --from=build /build/e-cat/target/release/admin-service /usr/local/bin/admin-service
EXPOSE 8001 8002 8003
CMD ["booking-service"]
