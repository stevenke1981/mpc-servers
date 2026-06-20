use std::{
    env,
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    process::Command,
};

pub const DEFAULT_TIMEOUT_MS: u64 = 30_000;
pub const MIN_TIMEOUT_MS: u64 = 100;
pub const MAX_TIMEOUT_MS: u64 = 120_000;
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 1_048_576;
pub const MIN_OUTPUT_BYTES: usize = 1_024;
pub const MAX_OUTPUT_BYTES: usize = 4_194_304;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NuRunMode {
    Eval {
        command: String,
    },
    Script {
        script_path: String,
        args: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct NuRunOptions {
    pub mode: NuRunMode,
    pub cwd: Option<PathBuf>,
    pub stdin: Option<String>,
    pub timeout_ms: Option<u64>,
    pub max_output_bytes: Option<usize>,
    pub nu_path: Option<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct NuCommandInfo {
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, serde::Serialize)]
pub struct NuRunResult {
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub signal: Option<String>,
    pub timed_out: bool,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub command: NuCommandInfo,
    pub error: Option<String>,
}

#[derive(Debug)]
struct CapturedOutput {
    text: String,
    truncated: bool,
}

struct RunCommandContext {
    executable: String,
    args: Vec<String>,
    cwd: String,
    started: Instant,
    timeout_ms: u64,
    max_output_bytes: usize,
    stdin: Option<String>,
}

pub fn resolve_nu_path(explicit: Option<&str>) -> String {
    explicit
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| env::var("NUSHELL_MCP_NU_PATH").ok())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "nu".to_owned())
}

pub fn build_nu_args(mode: &NuRunMode) -> Vec<String> {
    match mode {
        NuRunMode::Eval { command } => {
            vec![
                "--no-config-file".to_owned(),
                "--commands".to_owned(),
                command.clone(),
            ]
        }
        NuRunMode::Script { script_path, args } => {
            let mut nu_args = vec!["--no-config-file".to_owned(), script_path.clone()];
            nu_args.extend(args.iter().cloned());
            nu_args
        }
    }
}

pub fn clamp_timeout_ms(timeout_ms: Option<u64>) -> u64 {
    timeout_ms
        .unwrap_or(DEFAULT_TIMEOUT_MS)
        .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS)
}

pub fn clamp_max_output_bytes(max_output_bytes: Option<usize>) -> usize {
    max_output_bytes
        .unwrap_or(DEFAULT_MAX_OUTPUT_BYTES)
        .clamp(MIN_OUTPUT_BYTES, MAX_OUTPUT_BYTES)
}

pub fn version_options(nu_path: Option<String>) -> NuRunOptions {
    NuRunOptions {
        mode: NuRunMode::Eval {
            command: "$nu.version | to json".to_owned(),
        },
        cwd: None,
        stdin: None,
        timeout_ms: Some(5_000),
        max_output_bytes: Some(64 * 1024),
        nu_path,
    }
}

pub async fn get_nu_version(nu_path: Option<String>) -> NuRunResult {
    let executable = resolve_nu_path(nu_path.as_deref());
    let cwd = current_dir_string();
    let started = Instant::now();
    let args = vec!["--version".to_owned()];
    let mut command = Command::new(&executable);
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_command(
        command,
        RunCommandContext {
            executable,
            args,
            cwd,
            started,
            timeout_ms: 5_000,
            max_output_bytes: 64 * 1024,
            stdin: None,
        },
    )
    .await
}

pub async fn run_nushell(options: NuRunOptions) -> NuRunResult {
    let executable = resolve_nu_path(options.nu_path.as_deref());
    let args = build_nu_args(&options.mode);
    let cwd_path = options
        .cwd
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cwd = cwd_path.to_string_lossy().to_string();
    let timeout_ms = clamp_timeout_ms(options.timeout_ms);
    let max_output_bytes = clamp_max_output_bytes(options.max_output_bytes);
    let started = Instant::now();

    let mut command = Command::new(&executable);
    command
        .args(&args)
        .current_dir(&cwd_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    run_command(
        command,
        RunCommandContext {
            executable,
            args,
            cwd,
            started,
            timeout_ms,
            max_output_bytes,
            stdin: options.stdin,
        },
    )
    .await
}

async fn run_command(mut command: Command, context: RunCommandContext) -> NuRunResult {
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return NuRunResult {
                ok: false,
                exit_code: None,
                signal: None,
                timed_out: false,
                duration_ms: context.started.elapsed().as_millis(),
                stdout: String::new(),
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                command: NuCommandInfo {
                    executable: context.executable,
                    args: context.args,
                    cwd: context.cwd,
                },
                error: Some(error.to_string()),
            };
        }
    };

    if let Some(mut child_stdin) = child.stdin.take() {
        let input = context.stdin.unwrap_or_default();
        tokio::spawn(async move {
            let _ = child_stdin.write_all(input.as_bytes()).await;
            let _ = child_stdin.shutdown().await;
        });
    }

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let stdout_task = tokio::spawn(read_limited(stdout, context.max_output_bytes));
    let stderr_task = tokio::spawn(read_limited(stderr, context.max_output_bytes));
    let mut timed_out = false;

    let status =
        match tokio::time::timeout(Duration::from_millis(context.timeout_ms), child.wait()).await {
            Ok(Ok(status)) => Some(status),
            Ok(Err(_)) => None,
            Err(_) => {
                timed_out = true;
                let _ = child.kill().await;
                child.wait().await.ok()
            }
        };

    let stdout = stdout_task.await.unwrap_or_else(|_error| CapturedOutput {
        text: String::new(),
        truncated: false,
    });
    let stderr = stderr_task.await.unwrap_or_else(|error| CapturedOutput {
        text: format!("failed to capture stderr: {error}"),
        truncated: false,
    });
    let exit_code = status.as_ref().and_then(|status| status.code());

    NuRunResult {
        ok: !timed_out && exit_code == Some(0),
        exit_code,
        signal: signal_text(&status),
        timed_out,
        duration_ms: context.started.elapsed().as_millis(),
        stdout: stdout.text,
        stderr: stderr.text,
        stdout_truncated: stdout.truncated,
        stderr_truncated: stderr.truncated,
        command: NuCommandInfo {
            executable: context.executable,
            args: context.args,
            cwd: context.cwd,
        },
        error: None,
    }
}

async fn read_limited<R>(reader: Option<R>, max_bytes: usize) -> CapturedOutput
where
    R: AsyncRead + Unpin,
{
    let Some(mut reader) = reader else {
        return CapturedOutput {
            text: String::new(),
            truncated: false,
        };
    };

    let mut output = Vec::with_capacity(max_bytes.min(8192));
    let mut buffer = [0_u8; 8192];
    let mut truncated = false;

    loop {
        let read = match reader.read(&mut buffer).await {
            Ok(0) => break,
            Ok(read) => read,
            Err(_) => break,
        };
        let remaining = max_bytes.saturating_sub(output.len());
        if remaining == 0 {
            truncated = true;
            continue;
        }
        let take = remaining.min(read);
        output.extend_from_slice(&buffer[..take]);
        if take < read {
            truncated = true;
        }
    }

    CapturedOutput {
        text: String::from_utf8_lossy(&output).to_string(),
        truncated,
    }
}

fn current_dir_string() -> String {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .to_string_lossy()
        .to_string()
}

#[cfg(unix)]
fn signal_text(status: &Option<std::process::ExitStatus>) -> Option<String> {
    use std::os::unix::process::ExitStatusExt;
    status
        .as_ref()
        .and_then(|status| status.signal())
        .map(|signal| signal.to_string())
}

#[cfg(not(unix))]
fn signal_text(_status: &Option<std::process::ExitStatus>) -> Option<String> {
    None
}
