# e-cat-security

WAF/security scanning middleware for e-cat services.

Detects SQL injection, XSS, and other attack patterns via the
`security-rust` crate. Blocks requests with High/Critical severity
findings.

## Usage

```rust
use ecat_security::SecurityLayer;

let layer = SecurityLayer::new();
```
