use server::{Config, Runtime};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = Config::load()?;
    let runtime = Runtime::build(config).await?;
    runtime
        .run(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}
