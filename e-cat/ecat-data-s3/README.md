# ecat-data-s3

[S3](https://aws.amazon.com/s3/) / [MinIO](https://min.io) object storage client for the e-cat ecosystem, powered by [rust-s3](https://crates.io/crates/rust-s3).

```rust
let client = S3Client::from_config(S3Config {
    endpoint: "http://localhost:9000".into(),
    region: "us-east-1".into(),
    access_key: "minioadmin".into(),
    secret_key: "minioadmin".into(),
    tls: None,
})?;

client.put("assets", "avatars/1.png", &bytes).await?;
let bytes = client.get("assets", "avatars/1.png").await?;
let keys = client.list("assets", "avatars/").await?;
client.delete("assets", "avatars/1.png").await?;
```

Implements `StorageClient` from `ecat-data`.

**Notes:** uses path-style addressing (S3 API compatible with MinIO); an `endpoint` without a scheme defaults to `https://` — prefix an explicit `http://` (as above, for local MinIO) to opt out, and use `tls.skip_verify` for self-signed endpoints. All operations (including `list`) run with the client's 60-second default request timeout, so a hung server returns an error instead of blocking forever.
