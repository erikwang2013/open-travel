# Hello World

<p align="center"><img src="../../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


Minimal e-cat application demonstrating HTTP and gRPC transports.

## Run

```bash
cargo run -p helloworld
```

## Structure

- `main.rs` — Sets up logging, HTTP, and gRPC servers
- Uses `ecat`, `ecat-transport-http`, `ecat-transport-grpc`
