// line-service 入口：业务实现位于 ecat::business::line
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ecat::business::line::run().await
}
