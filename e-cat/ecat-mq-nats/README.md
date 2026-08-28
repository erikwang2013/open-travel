# ecat-mq-nats

[NATS](https://nats.io) message queue backend for the e-cat ecosystem, powered by [async-nats](https://crates.io/crates/async-nats).

```rust
let mq = NatsMq::connect("nats://localhost:4222").await?;

mq.publish("orders.created", b"{\"id\": 42}").await?;
let mut stream = mq.subscribe("orders.created").await?;
```

Implements `MessageQueue` from `ecat-mq`.

**Notes:** NATS subjects map directly to topics; `subscribe` returns a live `Subscription` stream (no delivery acknowledgements, per NATS semantics).
