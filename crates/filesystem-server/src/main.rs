use rmcp::{transport::stdio, ServiceExt};

use filesystem_server::{AllowedDirectories, FilesystemServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // --version, -V, "version" — print and exit immediately.
    if args
        .iter()
        .any(|a| a == "--version" || a == "-V" || a == "version")
    {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // Remaining positional arguments are allowed directory paths.
    let allowed = if args.is_empty() {
        eprintln!("Warning: No allowed directories specified. Tools requiring paths will fail.");
        AllowedDirectories::empty()
    } else {
        let dir_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        AllowedDirectories::from_existing_dirs(&dir_refs)
            .map_err(|e| format!("Failed to initialise allowed directories: {e}"))?
    };

    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    FilesystemServer::new(allowed)
        .serve(stdio())
        .await?
        .waiting()
        .await?;

    Ok(())
}
