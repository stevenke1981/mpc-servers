use anyhow::{bail, Result};
use server_inventory::targets;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);

    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("{}", env!("CARGO_PKG_VERSION"));
        }
        Some("inventory") => {
            let json = serde_json::to_string_pretty(targets())?;
            println!("{json}");
        }
        Some("list") | None => {
            for target in targets() {
                println!(
                    "{:<20} {:?} {:?}",
                    target.name, target.source_kind, target.decision
                );
            }
        }
        Some(command) => {
            bail!("unknown command: {command}. Use: list, inventory, or --version");
        }
    }

    Ok(())
}
