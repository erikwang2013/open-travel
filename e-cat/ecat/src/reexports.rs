// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
// 聚合 crate 的 feature 化 re-export：按需启用组件，默认 http+grpc。
// 嵌套模块命名（registry.consul / config.remote / data.redis）避免与
// 未来可能新增的顶层模块冲突。

#[cfg(feature = "http")]
pub use ecat_transport_http as transport_http;

#[cfg(feature = "grpc")]
pub use ecat_transport_grpc as transport_grpc;

#[cfg(feature = "middleware")]
pub use ecat_middleware as middleware;

#[cfg(feature = "auth")]
pub use ecat_auth as auth;

#[cfg(feature = "client")]
pub use ecat_client as client;

#[cfg(feature = "events")]
pub use ecat_events as events;

#[cfg(feature = "metrics")]
pub use ecat_metrics as metrics;

#[cfg(feature = "tracing")]
pub use ecat_tracing as tracing;

#[cfg(feature = "circuit-breaker")]
pub use ecat_circuit_breaker as circuit_breaker;

#[cfg(feature = "consul")]
pub mod registry {
    pub use ecat_registry_consul as consul;
}

#[cfg(feature = "remote")]
pub mod config {
    pub use ecat_config_remote as remote;
}

#[cfg(feature = "redis")]
pub mod data {
    pub use ecat_data_redis as redis;
}
