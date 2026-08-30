// user-service 入口：业务实现位于 ecat::business::user
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    ecat::business::user::run().await
}
