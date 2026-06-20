use std::{
    fs,
    path::{Path, PathBuf},
};

use nushell_mcp::nu::{
    NuRunMode, NuRunOptions, build_nu_args, clamp_max_output_bytes, clamp_timeout_ms,
    get_nu_version, run_nushell,
};

fn fake_nu(dir: &Path) -> PathBuf {
    let path = if cfg!(windows) {
        dir.join("fake-nu.cmd")
    } else {
        dir.join("fake-nu")
    };

    if cfg!(windows) {
        fs::write(
            &path,
            r#"@echo off
setlocal enabledelayedexpansion
if "%~1"=="--version" (
  echo 0.100.0
  exit /b 0
)
if "%~3"=="sleep" (
  ping -n 10 127.0.0.1 >nul
  exit /b 0
)
if "%~3"=="fail" (
  echo intentional failure 1>&2
  exit /b 7
)
echo mode=eval
echo command=%~3
echo cwd=%CD%
exit /b 0
"#,
        )
        .unwrap();
    } else {
        fs::write(
            &path,
            r#"#!/usr/bin/env sh
if [ "$1" = "--version" ]; then
  echo "0.100.0"
  exit 0
fi
if [ "$3" = "sleep" ]; then
  sleep 10
  exit 0
fi
if [ "$3" = "fail" ]; then
  echo "intentional failure" >&2
  exit 7
fi
echo "mode=eval"
echo "command=$3"
echo "cwd=$(pwd)"
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&path, permissions).unwrap();
        }
    }

    path
}

#[test]
fn builds_eval_and_script_args() {
    assert_eq!(
        build_nu_args(&NuRunMode::Eval {
            command: "1 + 1".to_owned()
        }),
        vec!["--no-config-file", "--commands", "1 + 1"]
    );
    assert_eq!(
        build_nu_args(&NuRunMode::Script {
            script_path: "task.nu".to_owned(),
            args: vec!["a".to_owned(), "b".to_owned()]
        }),
        vec!["--no-config-file", "task.nu", "a", "b"]
    );
}

#[test]
fn clamps_timeout_and_output_limits() {
    assert_eq!(clamp_timeout_ms(Some(1)), 100);
    assert_eq!(clamp_timeout_ms(Some(999_999)), 120_000);
    assert_eq!(clamp_timeout_ms(None), 30_000);
    assert_eq!(clamp_max_output_bytes(Some(1)), 1_024);
    assert_eq!(clamp_max_output_bytes(Some(99_999_999)), 4_194_304);
}

#[tokio::test]
async fn captures_success_output_and_cwd() {
    let temp = tempfile::tempdir().unwrap();
    let nu_path = fake_nu(temp.path()).to_string_lossy().to_string();
    let result = run_nushell(NuRunOptions {
        mode: NuRunMode::Eval {
            command: "echo hello".to_owned(),
        },
        cwd: Some(temp.path().to_path_buf()),
        stdin: None,
        timeout_ms: None,
        max_output_bytes: None,
        nu_path: Some(nu_path),
    })
    .await;

    assert!(result.ok, "{result:#?}");
    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout.contains("mode=eval"));
    assert!(result.stdout.contains("command=echo hello"));
    assert_eq!(result.command.cwd, temp.path().to_string_lossy());
}

#[tokio::test]
async fn preserves_non_zero_exit_and_stderr() {
    let temp = tempfile::tempdir().unwrap();
    let nu_path = fake_nu(temp.path()).to_string_lossy().to_string();
    let result = run_nushell(NuRunOptions {
        mode: NuRunMode::Eval {
            command: "fail".to_owned(),
        },
        cwd: None,
        stdin: None,
        timeout_ms: None,
        max_output_bytes: None,
        nu_path: Some(nu_path),
    })
    .await;

    assert!(!result.ok);
    assert_eq!(result.exit_code, Some(7));
    assert!(result.stderr.contains("intentional failure"));
}

#[tokio::test]
async fn times_out_and_kills_child() {
    let temp = tempfile::tempdir().unwrap();
    let nu_path = fake_nu(temp.path()).to_string_lossy().to_string();
    let result = run_nushell(NuRunOptions {
        mode: NuRunMode::Eval {
            command: "sleep".to_owned(),
        },
        cwd: None,
        stdin: None,
        timeout_ms: Some(150),
        max_output_bytes: None,
        nu_path: Some(nu_path),
    })
    .await;

    assert!(!result.ok);
    assert!(result.timed_out);
}

#[tokio::test]
async fn reports_missing_executable() {
    let result = get_nu_version(Some("definitely-not-a-real-nu-binary".to_owned())).await;

    assert!(!result.ok);
    assert!(result.error.is_some());
    assert_eq!(result.exit_code, None);
}

#[tokio::test]
async fn checks_version_with_configured_binary() {
    let temp = tempfile::tempdir().unwrap();
    let nu_path = fake_nu(temp.path()).to_string_lossy().to_string();
    let result = get_nu_version(Some(nu_path)).await;

    assert!(result.ok, "{result:#?}");
    assert_eq!(result.stdout.trim(), "0.100.0");
    assert_eq!(result.command.args, vec!["--version"]);
}
