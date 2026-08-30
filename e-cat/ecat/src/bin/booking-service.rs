// booking-service 入口：业务实现位于 ecat::business::booking
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ecat::business::booking::run().await
}
