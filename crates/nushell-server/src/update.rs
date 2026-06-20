use std::{env, path::PathBuf};

use serde::Serialize;

const REPO: &str = "stevenke1981/nushell-mcp";

#[derive(Debug, Serialize)]
pub struct UpdateReport {
    pub name: String,
    pub current_version: String,
    pub current_exe: Option<String>,
    pub stable_install_path: String,
    pub repo: String,
    pub latest_release_url: String,
    pub installer_command: String,
    pub notes: Vec<String>,
}

pub fn update_report() -> UpdateReport {
    let current_exe = env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_string());
    let stable_install_path = stable_install_path().to_string_lossy().to_string();

    UpdateReport {
        name: "nushell-mcp".to_owned(),
        current_version: env!("CARGO_PKG_VERSION").to_owned(),
        current_exe,
        stable_install_path,
        repo: REPO.to_owned(),
        latest_release_url: format!("https://github.com/{REPO}/releases/latest"),
        installer_command: "powershell -ExecutionPolicy Bypass -File .\\install.ps1 -Json"
            .to_owned(),
        notes: vec![
            "Run install.ps1 without -FromSource to download the latest GitHub Release asset."
                .to_owned(),
            "Run install.ps1 -FromSource to install the locally built release binary.".to_owned(),
            "Restart MCP clients after the reported installedPath changes.".to_owned(),
        ],
    }
}

fn stable_install_path() -> PathBuf {
    let home = env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".config")
        .join("nushell-mcp")
        .join("bin")
        .join(executable_name())
}

fn executable_name() -> &'static str {
    if cfg!(windows) {
        "nushell-mcp.exe"
    } else {
        "nushell-mcp"
    }
}
