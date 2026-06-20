use rmcp::{ServiceExt, transport::stdio};
use serde_json::json;

use nushell_mcp::{nu::get_nu_version, server::NushellServer, update::update_report};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();

    match args.as_slice() {
        [flag] if flag == "--version" || flag == "-V" || flag == "version" => {
            println!("nushell-mcp {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [command, flag] if command == "health" && flag == "--json" => {
            let nu = get_nu_version(None).await;
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "name": "nushell-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                    "nu": nu,
                }))?
            );
            Ok(())
        }
        [command, flag] if command == "update" && flag == "--json" => {
            println!("{}", serde_json::to_string_pretty(&update_report())?);
            Ok(())
        }
        [flag] if flag == "--help" || flag == "-h" || flag == "help" => {
            println!(
                "nushell-mcp {}\n\nUsage:\n  nushell-mcp                 Start stdio MCP server\n  nushell-mcp --version       Print server version\n  nushell-mcp health --json   Print JSON health report\n  nushell-mcp update --json   Print machine-readable update instructions\n\nEnvironment:\n  NUSHELL_MCP_NU_PATH         Path to nu/nu.exe when it is not on PATH",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        [] => {
            tracing_subscriber::fmt()
                .with_writer(std::io::stderr)
                .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
                .init();

            NushellServer::new().serve(stdio()).await?.waiting().await?;
            Ok(())
        }
        _ => Err(format!("unknown arguments: {}", args.join(" ")).into()),
    }
}
