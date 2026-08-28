# open-travel 业务服务镜像（user-service / booking-service）
# 构建上下文必须为仓库根（services 的 e-cat path 依赖位于 ../e-cat）
# 用法：docker build -f services/Dockerfile -t travel-services .
#       docker run travel-services user-service   （或 booking-service）

FROM rust:1.88-slim AS build
WORKDIR /build
RUN apt-get update && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
COPY e-cat ./e-cat
COPY services ./services
RUN cargo build --release --manifest-path services/Cargo.toml

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /build/services/target/release/user-service /usr/local/bin/user-service
COPY --from=build /build/services/target/release/booking-service /usr/local/bin/booking-service
EXPOSE 8001 8002
CMD ["booking-service"]
