# Hello World

Minimal e-cat application demonstrating HTTP and gRPC transports.

## Run

```bash
cargo run -p helloworld
```

## Structure

- `main.rs` — Sets up logging, HTTP, and gRPC servers
- Uses `ecat`, `ecat-transport-http`, `ecat-transport-grpc`
