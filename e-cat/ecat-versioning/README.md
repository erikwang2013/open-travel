# e-cat-versioning

API versioning strategies for axum-based services.

## Strategies

- **PathPrefix** — `/v1/health`, `/v2/health`
- **Header** — `Accept: application/json; version="v2"` with middleware validation

## Usage

```rust
use ecat_versioning::{VersionedRouter, VersionStrategy};

let v1 = axum::Router::new().route("/health", get(health));
let router = VersionedRouter::new(VersionStrategy::PathPrefix)
    .add_version("v1", v1)
    .default_version("v1")
    .build();
```
