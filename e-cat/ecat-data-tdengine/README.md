# ecat-data-tdengine

[TDengine](https://www.tdengine.com) time-series database client for the e-cat ecosystem, backed by the TDengine REST API (`/rest/sql`).

```rust
let client = TdengineClient::from_config(TdengineConfig {
    base_url: "http://localhost:6041".into(),
    username: "root".into(),
    password: "taosdata".into(),
    database: Some("demo".into()),
    tls: None,
})?;

client.write(&[DataPoint { measurement: "sensor1".into(), timestamp: Some(1_700_000_000_000), tags: map!{"location" => "rack-a"}, fields: map!{"temp" => FieldValue::Float(23.5)} }]).await?;
let rows = client.query("SELECT * FROM sensor1").await?;
```

Implements `TsdbClient` from `ecat-data`.

**Limitations:** tags are flattened as columns in the generated `INSERT` statement (measurement = table name), so all points written to one measurement must share the same tag set.

Part of the [e-cat](https://github.com/erik/e-cat) ecosystem.
