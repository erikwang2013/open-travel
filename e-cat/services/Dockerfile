# open-travel 业务服务镜像（user/booking/admin/search/line/order/flight/hotel/payment 九个二进制）
# 构建上下文必须为仓库根（e-cat workspace 在 ./e-cat）
# 用法：docker build -f e-cat/services/Dockerfile -t travel-services .
#       docker run travel-services user-service   （或任一服务名）

FROM rust:1.88-slim AS build
WORKDIR /build
# protobuf-compiler: tonic 构建期 protoc；g++/make: rdkafka 编译 librdkafka；
# cmake/libssl-dev/libsasl2-dev/libzstd-dev: TLS/SASL/zstd 支持（gssapi feature 链接系统 sasl）
RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler cmake g++ make pkg-config libssl-dev libsasl2-dev libzstd-dev \
    && rm -rf /var/lib/apt/lists/*
COPY e-cat ./e-cat
RUN cargo build --release --manifest-path e-cat/Cargo.toml \
    -p user-service -p booking-service -p admin-service -p search-service -p line-service -p order-service \
    -p flight-service -p hotel-service -p payment-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libsasl2-2 libzstd1 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/e-cat/target/release/user-service /usr/local/bin/user-service
COPY --from=build /build/e-cat/target/release/booking-service /usr/local/bin/booking-service
COPY --from=build /build/e-cat/target/release/admin-service /usr/local/bin/admin-service
COPY --from=build /build/e-cat/target/release/search-service /usr/local/bin/search-service
COPY --from=build /build/e-cat/target/release/line-service /usr/local/bin/line-service
COPY --from=build /build/e-cat/target/release/order-service /usr/local/bin/order-service
COPY --from=build /build/e-cat/target/release/flight-service /usr/local/bin/flight-service
COPY --from=build /build/e-cat/target/release/hotel-service /usr/local/bin/hotel-service
COPY --from=build /build/e-cat/target/release/payment-service /usr/local/bin/payment-service
EXPOSE 8001 8002 8003 8004 8005 8006 8007 8008 8009
CMD ["booking-service"]
