use std::time::Duration;

use fetch_server::{FetchConfig, FetchServer};
use rmcp::{transport::stdio, ServiceExt};

fn parse_u64(value: &str, flag: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} must be a positive integer"))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1);
    let mut config = FetchConfig::default();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" | "version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--allow-private-network" => {
                config.allow_private_network = true;
            }
            "--user-agent" => {
                config.user_agent = args
                    .next()
                    .ok_or_else(|| "missing value for --user-agent".to_string())?;
            }
            "--proxy-url" => {
                config.proxy_url = Some(
                    args.next()
                        .ok_or_else(|| "missing value for --proxy-url".to_string())?,
                );
            }
            "--max-bytes" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --max-bytes".to_string())?;
                config.max_response_bytes = parse_u64(&value, "--max-bytes")?;
            }
            "--timeout-seconds" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --timeout-seconds".to_string())?;
                config.timeout = Duration::from_secs(parse_u64(&value, "--timeout-seconds")?);
            }
            "--redirect-limit" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --redirect-limit".to_string())?;
                config.redirect_limit = parse_u64(&value, "--redirect-limit")? as usize;
            }
            _ => return Err(format!("unknown argument: {arg}").into()),
        }
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    FetchServer::new(config)?
        .serve(stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}
