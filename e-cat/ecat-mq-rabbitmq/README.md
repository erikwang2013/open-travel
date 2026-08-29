# ecat-mq-rabbitmq

<p align="center"><img src="../../docs/mascot.svg" alt="Travly 小旅 — Open Travel 吉祥物" width="180"></p>


[RabbitMQ](https://www.rabbitmq.com) message queue backend for the e-cat ecosystem, powered by [lapin](https://crates.io/crates/lapin).

```rust
let mq = RabbitmqMq::from_config(RabbitmqConfig {
    url: "amqp://guest:guest@localhost:5672".into(),
    exchange: Some("events".into()),
})
.await?;

mq.publish("orders.created", b"{\"id\": 42}").await?;
let mut stream = mq.subscribe("orders.created").await?;
```

Implements `MessageQueue` from `ecat-mq`.

**Notes:** subscribe declares the queue if it does not exist and consumes with auto-ack; publish uses the configured exchange (default exchange, routing key = topic).
