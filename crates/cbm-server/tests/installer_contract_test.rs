use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn manifest_points_agents_at_release_installers() {
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(root().join("packaging/mcp/manifest.json")).expect("read manifest"),
    )
    .expect("parse manifest");
    let install = &manifest["install"];
    let serialized = serde_json::to_string(install).expect("serialize install section");
    assert!(!serialized.contains("target/release"));
    assert!(!serialized.contains("target\\\\release"));
    assert_eq!(install["recommended_command"], "./install.ps1");
    assert!(install["windows_installer_url"]
        .as_str()
        .is_some_and(|url| url.contains("/packaging/windows/install.ps1")));
}

#[test]
fn release_installers_survive_github_api_limits() {
    let windows = fs::read_to_string(root().join("packaging/windows/install.ps1"))
        .expect("read Windows installer");
    assert!(windows.contains("Resolve-LatestVersion"));
    assert!(windows.contains("public release redirect"));
    assert!(windows.contains("GITHUB_TOKEN") && windows.contains("GH_TOKEN"));
    assert!(windows.contains("install --yes --all --json"));

    for relative in ["packaging/linux/install.sh", "packaging/macos/install.sh"] {
        let script = fs::read_to_string(root().join(relative)).expect("read Unix installer");
        assert!(script.contains("url_effective"), "{relative}");
        assert!(
            script.contains("GITHUB_TOKEN") && script.contains("GH_TOKEN"),
            "{relative}"
        );
    }
}
