// order-service 入口：业务实现位于 ecat::business::order
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ecat::business::order::run().await
}
