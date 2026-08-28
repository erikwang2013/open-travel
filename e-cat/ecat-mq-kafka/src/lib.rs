// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use bytes::Bytes;
use ecat_mq::{MessageQueue, MessageStream, MqError};
use futures_util::StreamExt;
use rdkafka::Message;
use rdkafka::config::ClientConfig;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
pub struct KafkaConfig {
    pub brokers: String,
    #[serde(default)]
    pub group_id: Option<String>,
    /// true 时开启 librdkafka 自动提交（每 5s），进程重启从最近提交点继续
    /// 消费（at-least-once）；false（默认）时 offset 不落盘，重启从最新
    /// 开始消费，停机期间的消息被静默跳过。
    #[serde(default)]
    pub auto_commit: bool,
    /// librdkafka 原生 TLS/SASL：设为 `ssl` 或 `sasl_ssl` 开启 TLS，
    /// 缺省（None）为明文直连。例：
    /// `{"security_protocol":"sasl_ssl","sasl_mechanism":"SCRAM-SHA-256",
    ///   "sasl_username":"u","sasl_password":"p"}`
    #[serde(default)]
    pub security_protocol: Option<String>,
    /// SASL mechanism（PLAIN / SCRAM-SHA-256 / SCRAM-SHA-512 等）。
    #[serde(default)]
    pub sasl_mechanism: Option<String>,
    #[serde(default)]
    pub sasl_username: Option<String>,
    #[serde(default)]
    pub sasl_password: Option<String>,
}

pub struct KafkaMq {
    producer: FutureProducer,
    config: KafkaConfig,
}

impl KafkaMq {
    pub async fn connect(brokers: &str) -> Result<Self, MqError> {
        Self::from_config(KafkaConfig {
            brokers: brokers.to_string(),
            group_id: None,
            auto_commit: false,
            security_protocol: None,
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
        })
        .await
    }

    pub async fn from_config(cfg: KafkaConfig) -> Result<Self, MqError> {
        let mut client_config = ClientConfig::new();
        client_config
            .set("bootstrap.servers", &cfg.brokers)
            .set("message.timeout.ms", "5000");
        apply_security(&cfg, &mut client_config);
        let producer: FutureProducer = client_config
            .create()
            .map_err(|e| MqError::Other(format!("kafka producer: {e}")))?;
        Ok(Self {
            producer,
            config: cfg,
        })
    }
}

#[async_trait]
impl MessageQueue for KafkaMq {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqError> {
        let record: FutureRecord<'_, str, [u8]> = FutureRecord::to(topic).payload(payload);
        self.producer
            .send(record, Duration::from_secs(5))
            .await
            .map_err(|(e, _)| MqError::Other(format!("kafka publish: {e}")))?;
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn MessageStream>, MqError> {
        let consumer: StreamConsumer = build_consumer_config(&self.config, topic)
            .create()
            .map_err(|e| MqError::Other(format!("kafka consumer: {e}")))?;
        consumer
            .subscribe(&[topic])
            .map_err(|e| MqError::Other(format!("kafka subscribe: {e}")))?;

        let (tx, rx) = mpsc::channel::<Bytes>(1024);
        // 入队/已消费计数：stream 被 drop 时量化未交付消息数（auto_commit
        // 下这些消息的 offset 仍会被 5s 自动提交，造成静默丢失）。
        let queued = Arc::new(AtomicU64::new(0));
        let consumed = Arc::new(AtomicU64::new(0));
        let queued_ref = Arc::clone(&queued);
        let consumed_ref = Arc::clone(&consumed);
        tokio::spawn(async move {
            // StreamConsumer 由 tokio 驱动：消息到达立即唤醒，空闲时挂起，
            // 无固定 poll/sleep 延迟，也不阻塞 tokio worker 线程。
            let mut stream = consumer.stream();
            while let Some(msg) = stream.next().await {
                let msg = match msg {
                    Ok(m) => m,
                    Err(e) => {
                        // librdkafka 内部有重连 backoff，此处只记录不节流
                        log_poll_error(&e);
                        continue;
                    }
                };
                if let Some(payload) = msg.payload()
                    && tx.send(Bytes::copy_from_slice(payload)).await.is_err()
                {
                    // rx 已 drop：通道内未消费消息与本消息一并丢失
                    let lost = queued_ref.load(Ordering::Relaxed)
                        - consumed_ref.load(Ordering::Relaxed)
                        + 1;
                    tracing::warn!(
                        lost,
                        "kafka consumer dropped: receiver closed, undelivered messages lost"
                    );
                    break;
                }
                queued.fetch_add(1, Ordering::Relaxed);
            }
        });
        Ok(Box::new(KafkaStream { rx, consumed }))
    }
}

struct KafkaStream {
    rx: mpsc::Receiver<Bytes>,
    consumed: Arc<AtomicU64>,
}

impl MessageStream for KafkaStream {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, MqError>>> {
        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(data)) => {
                self.consumed.fetch_add(1, Ordering::Relaxed);
                Poll::Ready(Some(Ok(data)))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

impl Unpin for KafkaStream {}

fn topic_hash(topic: &str) -> String {
    Sha256::digest(topic.as_bytes())[..4]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 消费组名派生。无配置 group_id 时每次订阅用随机组（独立消费者）；
/// 有 group_id 时按 `{group}-{topic_hash}` 派生：同一 (group, topic) 跨
/// 实例一致（共享消费组负载均衡、offset 组名稳定），不同 topic 隔离，
/// hash 后缀消除 group/topic 中 "-" 直接拼接的歧义碰撞。
fn consumer_group_id(group_id: Option<&str>, topic: &str) -> String {
    match group_id {
        Some(g) => format!("{g}-{}", topic_hash(topic)),
        None => format!("ecat-mq-{}", Uuid::new_v4()),
    }
}

fn build_consumer_config(cfg: &KafkaConfig, topic: &str) -> ClientConfig {
    let mut config = ClientConfig::new();
    config
        .set("bootstrap.servers", &cfg.brokers)
        // offset 语义：默认 enable.auto.commit=false + reset=latest，offset
        // 不落盘，进程重启后从最新开始消费，停机期间消息被静默跳过；
        // auto_commit=true 时 librdkafka 每 5s 自动提交（at-least-once，
        // 重启从最近提交点继续）。
        .set(
            "enable.auto.commit",
            if cfg.auto_commit { "true" } else { "false" },
        )
        .set("auto.offset.reset", "latest");
    config.set(
        "group.id",
        consumer_group_id(cfg.group_id.as_deref(), topic),
    );
    apply_security(cfg, &mut config);
    config
}

/// TLS/SASL 由 librdkafka 原生处理（security.protocol / sasl.*），
/// 无需额外依赖。全部字段可选，缺省保持明文直连。
fn apply_security(cfg: &KafkaConfig, c: &mut ClientConfig) {
    if let Some(p) = &cfg.security_protocol {
        c.set("security.protocol", p);
    }
    if let Some(m) = &cfg.sasl_mechanism {
        c.set("sasl.mechanism", m);
    }
    if let Some(u) = &cfg.sasl_username {
        c.set("sasl.username", u);
    }
    if let Some(p) = &cfg.sasl_password {
        c.set("sasl.password", p);
    }
}

fn log_poll_error(e: &rdkafka::error::KafkaError) {
    tracing::warn!(error = %e, "kafka consumer poll error, message skipped");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes() {
        let cfg: KafkaConfig = serde_json::from_value(serde_json::json!({
            "brokers": "localhost:9092",
            "group_id": "my-group",
            "security_protocol": "sasl_ssl",
            "sasl_mechanism": "SCRAM-SHA-256",
            "sasl_username": "u",
            "sasl_password": "p",
        }))
        .unwrap();
        assert_eq!(cfg.group_id.as_deref(), Some("my-group"));
        assert_eq!(cfg.security_protocol.as_deref(), Some("sasl_ssl"));
        assert_eq!(cfg.sasl_mechanism.as_deref(), Some("SCRAM-SHA-256"));
    }

    #[test]
    fn group_id_without_configured_group_is_random_and_unique() {
        let a = consumer_group_id(None, "user.created");
        let b = consumer_group_id(None, "user.created");
        assert_ne!(a, b);
        assert!(a.starts_with("ecat-mq-"), "got: {a}");
        // 不同 topic 同样各得独立消费组
        assert_ne!(consumer_group_id(None, "order.paid"), a);
    }

    #[test]
    fn group_id_derives_configured_group_per_topic() {
        let a = consumer_group_id(Some("my-group"), "user.created");
        assert!(a.starts_with("my-group-"), "got: {a}");
        // 派生后缀为 8 位 hex topic hash（确定性，跨实例一致）
        let suffix = a.strip_prefix("my-group-").unwrap();
        assert_eq!(suffix.len(), 8);
        assert!(suffix.chars().all(|c| c.is_ascii_hexdigit()));
        // 同一 (group, topic) 幂等 → 多实例/多订阅共享消费组负载均衡
        assert_eq!(a, consumer_group_id(Some("my-group"), "user.created"));
        // 不同 topic 必须隔离，避免同组 roundrobin 把消息分给错误订阅者
        assert_ne!(a, consumer_group_id(Some("my-group"), "order.paid"));
    }

    #[test]
    fn group_id_derivation_disambiguates_dashes() {
        // 旧格式 {group}-{topic} 下这两组输入碰撞（拼接均为 "my-group-1-a"）；
        // hash 后缀消歧。
        let a = consumer_group_id(Some("my-group-1"), "a");
        let b = consumer_group_id(Some("my-group"), "1-a");
        assert_ne!(a, b);
    }

    #[test]
    fn topic_hash_is_stable_hex() {
        assert_eq!(topic_hash("user.created"), topic_hash("user.created"));
        assert_eq!(topic_hash("user.created").len(), 8);
        assert!(
            topic_hash("user.created")
                .chars()
                .all(|c| c.is_ascii_hexdigit())
        );
        assert_ne!(topic_hash("user.created"), topic_hash("order.paid"));
    }

    #[test]
    fn poll_error_is_logged_at_warn() {
        use tracing::subscriber::with_default;
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        with_default(
            CaptureSubscriber {
                events: events.clone(),
            },
            || {
                log_poll_error(&rdkafka::error::KafkaError::ClientCreation("boom".into()));
            },
        );
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, tracing::Level::WARN);
        assert!(
            events[0].1.contains("kafka consumer poll error"),
            "got: {}",
            events[0].1
        );
    }

    #[tokio::test]
    async fn producer_constructs() {
        let _mq = KafkaMq::connect("localhost:9092").await.unwrap();
    }

    #[test]
    fn auto_commit_false_defaults_to_manual_offset_control() {
        let cfg = build_consumer_config(&test_config(), "t");
        assert_eq!(cfg.get("enable.auto.commit"), Some("false"));
        assert_eq!(cfg.get("auto.offset.reset"), Some("latest"));
        assert!(cfg.get("group.id").unwrap().starts_with("g-"));
    }

    #[test]
    fn auto_commit_true_enables_automatic_commit() {
        let mut cfg = test_config();
        cfg.auto_commit = true;
        let c = build_consumer_config(&cfg, "t");
        assert_eq!(c.get("enable.auto.commit"), Some("true"));
    }

    #[test]
    fn security_settings_applied_to_producer_and_consumer() {
        let cfg = KafkaConfig {
            brokers: "h:9092".into(),
            group_id: None,
            auto_commit: false,
            security_protocol: Some("sasl_ssl".into()),
            sasl_mechanism: Some("PLAIN".into()),
            sasl_username: Some("u".into()),
            sasl_password: Some("p".into()),
        };
        let c = build_consumer_config(&cfg, "t");
        assert_eq!(c.get("security.protocol"), Some("sasl_ssl"));
        assert_eq!(c.get("sasl.mechanism"), Some("PLAIN"));
        assert_eq!(c.get("sasl.username"), Some("u"));
        assert_eq!(c.get("sasl.password"), Some("p"));
    }

    #[test]
    fn security_defaults_to_plaintext() {
        let c = build_consumer_config(&test_config(), "t");
        assert_eq!(c.get("security.protocol"), None);
        assert_eq!(c.get("sasl.mechanism"), None);
    }

    #[tokio::test]
    async fn stream_consumer_constructs_with_derived_group() {
        // 锁定订阅路径构造：无配置 group_id 时也能创建 StreamConsumer（
        // rdkafka create() 只校验配置不连 broker），避免回归到 INVALID_ARG。
        let consumer: StreamConsumer = build_consumer_config(&test_config(), "test.topic")
            .create()
            .unwrap();
        consumer.subscribe(&["test.topic"]).unwrap();
    }

    #[test]
    fn config_defaults_when_fields_missing() {
        let cfg: KafkaConfig =
            serde_json::from_value(serde_json::json!({"brokers": "h:9092"})).unwrap();
        assert_eq!(cfg.brokers, "h:9092");
        assert!(!cfg.auto_commit, "auto_commit must default to false");
        assert!(cfg.group_id.is_none());
        assert!(cfg.security_protocol.is_none());
        assert!(cfg.sasl_mechanism.is_none());
        assert!(cfg.sasl_username.is_none());
        assert!(cfg.sasl_password.is_none());
    }

    #[test]
    fn security_fields_apply_independently() {
        // 只配 SASL 用户名/机制、不配 protocol：字段仍须各自落到配置上
        let cfg = KafkaConfig {
            brokers: "h:9092".into(),
            group_id: None,
            auto_commit: false,
            security_protocol: None,
            sasl_mechanism: Some("PLAIN".into()),
            sasl_username: Some("u".into()),
            sasl_password: None,
        };
        let c = build_consumer_config(&cfg, "t");
        assert_eq!(c.get("security.protocol"), None);
        assert_eq!(c.get("sasl.mechanism"), Some("PLAIN"));
        assert_eq!(c.get("sasl.username"), Some("u"));
        assert_eq!(c.get("sasl.password"), None);
    }

    fn test_config() -> KafkaConfig {
        KafkaConfig {
            brokers: "localhost:9092".into(),
            group_id: Some("g".into()),
            auto_commit: false,
            security_protocol: None,
            sasl_mechanism: None,
            sasl_username: None,
            sasl_password: None,
        }
    }

    struct CaptureSubscriber {
        events: std::sync::Arc<std::sync::Mutex<Vec<(tracing::Level, String)>>>,
    }

    impl tracing::Subscriber for CaptureSubscriber {
        fn enabled(&self, _: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _: &tracing::span::Id, _: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _: &tracing::span::Id, _: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            let mut visitor = CaptureVisitor::default();
            event.record(&mut visitor);
            self.events
                .lock()
                .unwrap()
                .push((*event.metadata().level(), visitor.message));
        }
        fn enter(&self, _: &tracing::span::Id) {}
        fn exit(&self, _: &tracing::span::Id) {}
    }

    #[derive(Default)]
    struct CaptureVisitor {
        message: String,
    }

    impl tracing::field::Visit for CaptureVisitor {
        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" {
                self.message = format!("{value:?}");
            }
        }
    }
}
