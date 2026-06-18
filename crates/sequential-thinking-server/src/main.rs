use rmcp::{transport::stdio, ServiceExt};
use sequential_thinking_server::SequentialThinkingServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--version" || arg == "-V" || arg == "version")
    {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    SequentialThinkingServer::new()
        .serve(stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}
