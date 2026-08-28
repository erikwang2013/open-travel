// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
use async_trait::async_trait;
use bytes::Bytes;
use ecat_mq::{MessageQueue, MessageStream, MqError};
use futures_core::Stream;
use lapin::options::{BasicConsumeOptions, BasicPublishOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, Consumer};
use serde::Deserialize;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug, Clone, Deserialize)]
pub struct RabbitmqConfig {
    /// `amqp://` 或 `amqps://`。amqps 由 lapin 默认特性（rustls）原生支持，
    /// 使用系统信任根；自定义 CA 暂不支持（如需请改用服务端签发系统信任的证书）。
    pub url: String,
    #[serde(default)]
    pub exchange: Option<String>,
}

pub struct RabbitmqMq {
    channel: Channel,
    exchange: Option<String>,
}

impl RabbitmqMq {
    pub async fn connect(url: &str) -> Result<Self, MqError> {
        Self::from_config(RabbitmqConfig {
            url: url.to_string(),
            exchange: None,
        })
        .await
    }

    pub async fn from_config(cfg: RabbitmqConfig) -> Result<Self, MqError> {
        let conn = Connection::connect(&cfg.url, ConnectionProperties::default())
            .await
            .map_err(|e| MqError::Other(format!("rabbitmq connect: {e}")))?;
        let channel = conn
            .create_channel()
            .await
            .map_err(|e| MqError::Other(format!("rabbitmq channel: {e}")))?;
        Ok(Self {
            channel,
            exchange: cfg.exchange,
        })
    }
}

#[async_trait]
impl MessageQueue for RabbitmqMq {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqError> {
        let exchange = self.exchange.as_deref().unwrap_or("");
        self.channel
            .basic_publish(
                exchange,
                topic,
                BasicPublishOptions::default(),
                payload,
                BasicProperties::default(),
            )
            .await
            .map_err(|e| MqError::Other(format!("rabbitmq publish: {e}")))?
            .await
            .map_err(|e| MqError::Other(format!("rabbitmq publish: {e}")))?;
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn MessageStream>, MqError> {
        let queue = self
            .channel
            .queue_declare(topic, QueueDeclareOptions::default(), FieldTable::default())
            .await
            .map_err(|e| MqError::Other(format!("rabbitmq queue_declare: {e}")))?;
        let consumer = self
            .channel
            .basic_consume(
                queue.name().as_str(),
                "ecat",
                BasicConsumeOptions {
                    no_ack: true,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| MqError::Other(format!("rabbitmq consume: {e}")))?;
        Ok(Box::new(RabbitStream { consumer }))
    }
}

struct RabbitStream {
    consumer: Consumer,
}

impl MessageStream for RabbitStream {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, MqError>>> {
        match Pin::new(&mut self.consumer).poll_next(cx) {
            Poll::Ready(Some(Ok(delivery))) => Poll::Ready(Some(Ok(Bytes::from(delivery.data)))),
            Poll::Ready(Some(Err(e))) => {
                Poll::Ready(Some(Err(MqError::Other(format!("rabbitmq recv: {e}")))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_deserializes() {
        let cfg: RabbitmqConfig = serde_json::from_value(serde_json::json!({
            "url": "amqp://guest:guest@localhost:5672",
            "exchange": "events",
        }))
        .unwrap();
        assert_eq!(cfg.exchange.as_deref(), Some("events"));
    }

    #[tokio::test]
    async fn connect_fails_bad_url() {
        let result = RabbitmqMq::connect("amqp://127.0.0.1:1").await;
        assert!(result.is_err());
    }

    #[test]
    fn config_exchange_defaults_to_none() {
        let cfg: RabbitmqConfig =
            serde_json::from_value(serde_json::json!({"url": "amqp://localhost"})).unwrap();
        assert_eq!(cfg.url, "amqp://localhost");
        assert!(cfg.exchange.is_none());
    }

    #[test]
    fn config_rejects_missing_url() {
        let r = serde_json::from_value::<RabbitmqConfig>(serde_json::json!({}));
        assert!(r.is_err(), "url is required");
    }

    #[test]
    fn config_rejects_non_string_url() {
        let r = serde_json::from_value::<RabbitmqConfig>(serde_json::json!({"url": 42}));
        assert!(r.is_err());
    }
}
