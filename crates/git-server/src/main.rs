use std::path::PathBuf;

use git_server::{validate_repo_path, GitServer};
use rmcp::{transport::stdio, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1).peekable();
    let mut repository: Option<PathBuf> = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--version" | "-V" | "version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--repository" | "-r" => {
                let value = args
                    .next()
                    .ok_or_else(|| "missing value for --repository".to_string())?;
                repository = Some(PathBuf::from(value));
            }
            "-v" | "--verbose" => {
                // Accepted for upstream CLI compatibility. Logging level is controlled by RUST_LOG.
            }
            _ if repository.is_none() => {
                repository = Some(PathBuf::from(arg));
            }
            _ => {
                return Err(format!("unknown argument: {arg}").into());
            }
        }
    }

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let repository = repository
        .as_deref()
        .map(|path| validate_repo_path(path, None))
        .transpose()
        .map_err(|e| format!("invalid --repository: {e}"))?;

    GitServer::new(repository)
        .serve(stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}
