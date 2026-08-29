# e-cat-client

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


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
