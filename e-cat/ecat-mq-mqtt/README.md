# ecat-mq-mqtt

[MQTT](https://mqtt.org) message queue backend for the e-cat ecosystem, powered by [rumqttc](https://crates.io/crates/rumqttc).

```rust
let mq = MqttMq::from_config(MqttConfig {
    url: "tcp://localhost:1883".into(),
    client_id: Some("sensor-1".into()),
    username: None,
    password: None,
})?;

mq.publish("sensors/temp", b"23.5").await?;
let mut stream = mq.subscribe("sensors/temp").await?;
```

Implements `MessageQueue` from `ecat-mq`.

**Notes:** publish uses QoS 0; each subscription gets its own connection (`<client_id>-sub<n>`) so slow consumers cannot stall each other; a keep-alive task drives the publisher's event loop.
