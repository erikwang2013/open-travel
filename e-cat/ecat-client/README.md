# e-cat-client

HTTP and gRPC service client with service discovery and load balancing.

## Components

- `ServiceResolver` — resolve service names to endpoints
- `LoadBalancer` — round-robin or random endpoint selection
- `HttpClient` — HTTP client with resolver + balancer
- `GrpcClient` — gRPC client with resolver + balancer

## Usage

```rust
use ecat_client::{HttpClient, StaticResolver};

let resolver = StaticResolver::single("auth", "http://localhost:8080");
let client = HttpClient::builder().resolver(resolver).build().unwrap();
let resp = client.get("auth", "/health").await.unwrap();
```
