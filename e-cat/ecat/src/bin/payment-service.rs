// payment-service 入口：业务实现位于 ecat::business::payment
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ecat::business::payment::run().await
}
