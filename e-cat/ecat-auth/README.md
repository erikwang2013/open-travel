# e-cat-auth

Authentication and authorization middleware for e-cat services.

## Modules

- `jwt` — JWT Bearer token validation (HS256)
- `apikey` — API key validation (header or query param)
- `oauth2` — OAuth2 token introspection (RFC 7662)
- `claims` — `AuthClaims` struct with role-based access
- `helpers` — token extraction utilities

## Usage

```rust
use ecat_auth::JwtAuthLayer;

let auth = JwtAuthLayer::new("my-secret-key")
    .require_claims(&["sub", "role"]);
```
