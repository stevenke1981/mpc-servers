use rmcp::service::serve_server;
use rmcp::transport::io;
use time_server::TimeServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let handler = TimeServer;

    let (stdin, stdout) = io::stdio();
    serve_server(handler, (stdin, stdout)).await?;

    Ok(())
}
