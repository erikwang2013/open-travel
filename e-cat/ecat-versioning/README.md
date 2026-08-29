# e-cat-versioning

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


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
