# e-cat

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


Core application framework for the e-cat ecosystem.

## Features

- Modular service builder (`App`)
- Transport layer abstraction (HTTP/gRPC/WebSocket)
- Middleware pipeline (rate limiting, timeout, tracing)
- Health checks, metrics, distributed tracing
- Configuration management with obfuscation support

## Usage

```rust
use ecat::App;

let app = App::builder()
    .name("my-service")
    .build()
    .unwrap();
```
