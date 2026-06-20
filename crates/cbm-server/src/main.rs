use clap::{Parser, Subcommand};
use codebase_memory_mcp::cli;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "cbm", version, about = "cbm — Rust knowledge graph MCP server")]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,

    /// Enable HTTP graph UI (also CBM_UI=1)
    #[arg(long, default_value_t = false)]
    ui: bool,

    /// HTTP UI port (also CBM_PORT)
    #[arg(long, default_value_t = 9749)]
    port: u16,
}

#[derive(Subcommand)]
enum Command {
    /// Run a single MCP tool from CLI
    Cli {
        tool: String,
        #[arg(long)]
        json: bool,
        /// Suppress diagnostic logs (stdout/json only for CLI tools)
        #[arg(long)]
        quiet: bool,
        args: Option<String>,
    },
    /// Install binary and configure MCP for coding agents
    Install {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        force: bool,
        #[arg(short = 'y', long)]
        yes: bool,
        #[arg(long)]
        all: bool,
        /// Print the install report as JSON on stdout
        #[arg(long)]
        json: bool,
    },
    /// Remove MCP integration and hooks
    Uninstall {
        #[arg(long)]
        dry_run: bool,
        #[arg(short = 'y', long)]
        yes: bool,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        keep_binary: bool,
    },
    /// SessionStart reminder hook (prints graph-first guidance)
    HookSessionStart,
    /// PreToolUse graph augmenter (reads hook JSON from stdin)
    HookAugment,
    /// Config utilities
    Config { action: String },
    /// HTTP graph UI only (no MCP stdio)
    Ui {
        #[arg(long, default_value_t = 9749)]
        port: u16,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let cli_quiet = matches!(&args.command, Some(Command::Cli { quiet: true, .. }));
    if !cli_quiet {
        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::from_default_env()
                    .add_directive("codebase_memory_mcp=info".parse().unwrap()),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    let result = match args.command {
        Some(Command::Cli {
            tool,
            json,
            quiet,
            args,
        }) => cli::run_cli(&tool, args.as_deref(), json, quiet),
        Some(Command::Install {
            dry_run,
            force,
            yes,
            all,
            json,
        }) => cli::run_install(
            codebase_memory_mcp::install::InstallOptions {
                dry_run,
                force,
                yes,
                all_agents: all,
                binary: None,
            },
            json,
        ),
        Some(Command::Uninstall {
            dry_run,
            yes,
            all,
            keep_binary,
        }) => cli::run_uninstall(codebase_memory_mcp::install::UninstallOptions {
            dry_run,
            yes,
            all_agents: all,
            keep_binary,
        }),
        Some(Command::HookAugment) => {
            cli::run_hook_augment();
            Ok(())
        }
        Some(Command::HookSessionStart) => {
            std::process::exit(codebase_memory_mcp::hooks::hook_session_start());
        }
        Some(Command::Config { action }) => cli::run_config(&action),
        Some(Command::Ui { port }) => cli::run_ui_server(port),
        None => {
            cli::run_mcp_server(codebase_memory_mcp::http::UiConfig::from_env_and_args(
                args.ui, args.port,
            ))
            .await
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
