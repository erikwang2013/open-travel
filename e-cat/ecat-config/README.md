# e-cat-config

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


Configuration management for e-cat services.

## Sources

- `FileSource` — YAML/JSON config files
- `EnvSource` — environment variables
- `ObfuscatedSource` — XOR-obfuscated values in config files

> **Note:** `ObfuscatedSource` provides obfuscation (not encryption). Use a secrets manager for real security.

## Usage

```rust
use ecat_config::{Config, FileSource};

let mut cfg = Config::new();
cfg.load(&FileSource::new("config.yaml")).await.unwrap();
let value: String = cfg.get("key").unwrap();
```
