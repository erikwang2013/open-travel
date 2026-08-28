<!-- Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz -->
# e-cat Security Integration Design

**Date:** 2026-07-29  
**Status:** Draft  
**Crate:** `ecat-security`  
**Upstream:** [security-rust](https://crates.io/crates/security-rust) v1.0.4

## 1. 目标

将 `security-rust` 的 27 个攻击检测器接入 e-cat，提供 Tower Layer 中间件自动扫描 HTTP 请求。检测到攻击时仅日志告警，不阻断请求。

## 2. API 设计

```rust
// === 便捷封装 ===

pub struct SecurityScanner { scanner: Scanner }

impl SecurityScanner {
    /// 创建启用全部 27 个检测器的 scanner
    pub fn new() -> Self;
    /// 只启用指定类别的检测器
    pub fn with_categories(categories: &[AttackCategory]) -> Self;
    /// 扫描单个字符串
    pub fn scan(&self, input: &str) -> Vec<DetectionResult>;
    /// 扫描多个来源（path, query, headers, body...）
    pub fn scan_parts(&self, parts: &[&str]) -> Vec<DetectionResult>;
}

// === Tower Layer ===

pub struct SecurityLayer { scanner: Arc<SecurityScanner> }

impl SecurityLayer {
    pub fn new() -> Self;
    pub fn with_categories(cats: &[AttackCategory]) -> Self;
}

impl<S> Layer<S> for SecurityLayer { ... }  // → SecurityService<S>
```

### Layer 扫描策略

从 `http::Request` 中提取以下部分进行扫描：
1. **URI path + query** — 完整的 `path_and_query`
2. **Header values** — 遍历所有 header value

检测到攻击时：
```rust
tracing::warn!(
    attack_type = %r.attack_type,
    category = ?r.category,
    severity = ?r.severity,
    matched = %r.matched_pattern,
    "attack detected"
);
```

## 3. 文件结构

```
ecat-security/
├── Cargo.toml       → security-rust, tracing, tower, http
└── src/
    └── lib.rs       → re-exports, SecurityScanner, SecurityLayer
```

## 4. 使用示例

```rust
// 方式 1: Tower Layer（自动扫描所有请求）
use ecat_security::SecurityLayer;
use tower::ServiceBuilder;

let layer = ServiceBuilder::new()
    .layer(SecurityLayer::new());

// 方式 2: 函数式 API（handler 内按需扫描）
use ecat_security::SecurityScanner;

let scanner = SecurityScanner::new();
let results = scanner.scan(user_input);
```

## 5. 依赖

```toml
[dependencies]
security-rust = "1.0"
tracing = "0.1"
tower = { version = "0.5", features = ["util"] }
http = "1"
```

## 6. 测试

- scanner 检测 SQL 注入
- scanner 检测 XSS
- scanner 对正常输入返回空
- layer 编译 + 基本功能验证
