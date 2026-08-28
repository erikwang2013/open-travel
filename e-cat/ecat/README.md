# e-cat

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
