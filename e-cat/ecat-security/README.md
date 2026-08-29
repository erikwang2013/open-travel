# e-cat-security

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


WAF/security scanning middleware for e-cat services.

Detects SQL injection, XSS, and other attack patterns via the
`security-rust` crate. Blocks requests with High/Critical severity
findings.

## Usage

```rust
use ecat_security::SecurityLayer;

let layer = SecurityLayer::new();
```
