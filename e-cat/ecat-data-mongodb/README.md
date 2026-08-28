# ecat-data-mongodb

[MongoDB](https://www.mongodb.com) document database client for the e-cat ecosystem, powered by the official [mongodb](https://crates.io/crates/mongodb) driver.

```rust
let client = MongoClient::from_config(MongoConfig {
    url: "mongodb://localhost:27017".into(),
    database: "app".into(),
    tls: None,
})
.await?;

let id = client.insert("users", &json!({"name": "erik"})).await?;
let users = client.find("users", &json!({"name": "erik"})).await?;
let n = client.update("users", &json!({"name": "erik"}), &json!({"$set": {"age": 42}})).await?;
let deleted = client.delete("users", &json!({"name": "erik"})).await?;
```

Implements `DocumentClient` from `ecat-data`. Documents are passed as `serde_json::Value` and converted to BSON internally.

**Notes:** TLS is configured through the connection string (`mongodb+srv://` or `tls=true`); the `tls` config field is reserved for future use.
