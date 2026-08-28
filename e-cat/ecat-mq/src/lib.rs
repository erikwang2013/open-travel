// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! In-memory pub/sub: messages are dropped when no subscriber is attached or
//! when a slow subscriber's channel overflows (the subscriber sees a "lagged"
//! error frame). Not a durable broker — use a real MQ backend for production.
use async_trait::async_trait;
use bytes::Bytes;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::futures::OwnedNotified;
use tokio::sync::{Notify, broadcast};

#[async_trait]
pub trait MessageQueue: Send + Sync {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqError>;
    async fn subscribe(&self, topic: &str) -> Result<Box<dyn MessageStream>, MqError>;
}

pub trait MessageStream: Send + Unpin {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, MqError>>>;
}

#[derive(Debug, PartialEq, thiserror::Error)]
pub enum MqError {
    #[error("mq error: {0}")]
    Other(String),
}

pub struct InMemoryMq {
    senders: std::sync::RwLock<HashMap<String, Vec<SenderEntry>>>,
}

struct SenderEntry {
    tx: broadcast::Sender<Bytes>,
    notify: Arc<Notify>,
}

impl InMemoryMq {
    pub fn new() -> Self {
        Self {
            senders: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryMq {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageQueue for InMemoryMq {
    async fn publish(&self, topic: &str, payload: &[u8]) -> Result<(), MqError> {
        let map = self
            .senders
            .read()
            .map_err(|e| MqError::Other(e.to_string()))?;
        if let Some(entries) = map.get(topic) {
            let data = Bytes::copy_from_slice(payload);
            for entry in entries {
                if entry.tx.send(data.clone()).is_err() {
                    // 订阅者已 drop（broadcast 无存活接收者）：消息被静默丢弃
                    tracing::warn!(topic, "in-memory mq: subscriber gone, message dropped");
                }
                entry.notify.notify_waiters();
            }
        }
        Ok(())
    }

    async fn subscribe(&self, topic: &str) -> Result<Box<dyn MessageStream>, MqError> {
        let (tx, rx) = broadcast::channel(256);
        let notify = Arc::new(Notify::new());
        let mut map = self
            .senders
            .write()
            .map_err(|e| MqError::Other(e.to_string()))?;
        map.entry(topic.to_string()).or_default().push(SenderEntry {
            tx,
            notify: notify.clone(),
        });
        Ok(Box::new(InMemoryStream {
            rx,
            notify,
            notified: None,
        }))
    }
}

struct InMemoryStream {
    rx: broadcast::Receiver<Bytes>,
    notify: Arc<Notify>,
    // OwnedNotified 自带 Arc<Notify>，避免对 self 的自引用借用
    notified: Option<Pin<Box<OwnedNotified>>>,
}

impl MessageStream for InMemoryStream {
    fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Bytes, MqError>>> {
        // 先同步取数据；空时挂起在 Notify 上等待 publish 唤醒，不做自旋。
        match self.rx.try_recv() {
            Ok(data) => return Poll::Ready(Some(Ok(data))),
            // 订阅者滞后（channel 满被丢消息）：返回错误帧，调用方可决定继续消费
            Err(broadcast::error::TryRecvError::Lagged(n)) => {
                return Poll::Ready(Some(Err(MqError::Other(format!(
                    "subscriber lagged: {n} messages dropped"
                )))));
            }
            Err(broadcast::error::TryRecvError::Closed) => return Poll::Ready(None),
            Err(broadcast::error::TryRecvError::Empty) => {}
        }

        if self.notified.is_none() {
            self.notified = Some(Box::pin(Arc::clone(&self.notify).notified_owned()));
        }
        match self.notified.as_mut().expect("notified").as_mut().poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(()) => {
                self.notified = None;
                match self.rx.try_recv() {
                    Ok(data) => Poll::Ready(Some(Ok(data))),
                    Err(broadcast::error::TryRecvError::Lagged(n)) => Poll::Ready(Some(Err(
                        MqError::Other(format!("subscriber lagged: {n} messages dropped")),
                    ))),
                    Err(broadcast::error::TryRecvError::Closed) => Poll::Ready(None),
                    // 唤醒竞态下数据尚未可见：保持 Pending，下次 poll 会重新注册
                    Err(broadcast::error::TryRecvError::Empty) => Poll::Pending,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn inmemory_pub_sub() {
        let mq = InMemoryMq::new();
        mq.publish("test", b"hello").await.unwrap();
        let _stream = mq.subscribe("test").await.unwrap();
    }

    #[test]
    fn mq_error_display() {
        let e = MqError::Other("boom".into());
        assert!(e.to_string().contains("boom"));
    }

    #[tokio::test]
    async fn stream_receives_published_message() {
        let mq = InMemoryMq::new();
        let mut stream = mq.subscribe("topic").await.unwrap();
        mq.publish("topic", b"hello").await.unwrap();
        let msg = std::future::poll_fn(|cx| stream.poll_recv(cx)).await;
        assert_eq!(msg, Some(Ok(Bytes::from_static(b"hello"))));
    }

    #[tokio::test]
    async fn publish_after_subscriber_drop_logs_warning() {
        use tracing::subscriber::with_default;
        let mq = InMemoryMq::new();
        {
            let _stream = mq.subscribe("gone").await.unwrap();
        }
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let ev = events.clone();
        with_default(CaptureSubscriber { events: ev }, || {
            // publish 体内无真实 await，手动驱动一次即可；with_default 是同步闭包
            let mut fut = std::pin::pin!(mq.publish("gone", b"x"));
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            assert!(fut.as_mut().poll(&mut cx).is_ready());
        });
        let events = events.lock().unwrap();
        assert!(
            events
                .iter()
                .any(|(l, m)| *l == tracing::Level::WARN && m.contains("subscriber gone")),
            "expected drop warning, got: {events:?}"
        );
    }

    #[tokio::test]
    async fn stream_empty_does_not_spin() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::task::Wake;

        struct CountWaker(Arc<AtomicUsize>);
        impl Wake for CountWaker {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mq = InMemoryMq::new();
        let mut stream = mq.subscribe("topic").await.unwrap();
        let wakes = Arc::new(AtomicUsize::new(0));
        let waker = std::task::Waker::from(Arc::new(CountWaker(wakes.clone())));
        let mut cx = std::task::Context::from_waker(&waker);

        assert!(matches!(
            stream.poll_recv(&mut cx),
            std::task::Poll::Pending
        ));
        // 空 channel 时不得自唤醒（修复前 wake_by_ref 会立即 +1）
        assert_eq!(wakes.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn stream_reports_lagged_when_subscriber_falls_behind() {
        let mq = InMemoryMq::new();
        let mut stream = mq.subscribe("topic").await.unwrap();
        for i in 0..300 {
            mq.publish("topic", format!("m{i}").as_bytes())
                .await
                .unwrap();
        }
        // channel 容量 256：300 条消息超出保留窗口，广播语义下返回 lagged 错误帧
        let msg = std::future::poll_fn(|cx| stream.poll_recv(cx)).await;
        match &msg {
            Some(Err(MqError::Other(e))) => assert!(e.contains("lagged"), "got: {msg:?}"),
            other => panic!("expected lagged error frame, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn stream_closes_when_all_senders_dropped() {
        let mq = InMemoryMq::new();
        let mut stream = mq.subscribe("topic").await.unwrap();
        drop(mq);
        let msg = std::future::poll_fn(|cx| stream.poll_recv(cx)).await;
        assert_eq!(msg, None, "dropping the only sender must close the stream");
    }

    #[tokio::test]
    async fn all_subscribers_receive_each_message() {
        let mq = InMemoryMq::new();
        let mut a = mq.subscribe("topic").await.unwrap();
        let mut b = mq.subscribe("topic").await.unwrap();
        mq.publish("topic", b"hi").await.unwrap();
        let pa = std::future::poll_fn(|cx| a.poll_recv(cx)).await;
        let pb = std::future::poll_fn(|cx| b.poll_recv(cx)).await;
        assert_eq!(pa, Some(Ok(Bytes::from_static(b"hi"))));
        assert_eq!(pb, Some(Ok(Bytes::from_static(b"hi"))));
    }

    #[tokio::test]
    async fn publish_without_subscribers_is_ok() {
        let mq = InMemoryMq::new();
        mq.publish("nobody", b"x").await.unwrap();
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
