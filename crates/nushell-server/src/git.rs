use std::{
    env,
    path::PathBuf,
    process::Stdio,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

use crate::nu::{
    DEFAULT_MAX_OUTPUT_BYTES, DEFAULT_TIMEOUT_MS, clamp_max_output_bytes, clamp_timeout_ms,
};

const GIT_CRLF_ADVICE: &str = "core.safecrlf=false";

#[derive(Debug, Clone)]
pub struct GitRunOptions {
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub timeout_ms: Option<u64>,
    pub max_output_bytes: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct GitCommandInfo {
    pub executable: String,
    pub args: Vec<String>,
    pub cwd: String,
}

#[derive(Debug, Serialize)]
pub struct GitRunResult {
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub timed_out: bool,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub command: GitCommandInfo,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GitCommitResult {
    pub ok: bool,
    pub hash: Option<String>,
    pub subject: Option<String>,
    pub branch: Option<String>,
    pub stat: Option<String>,
    pub add: GitRunResult,
    pub commit: Option<GitRunResult>,
    pub error: Option<String>,
}

#[derive(Debug)]
struct CapturedOutput {
    text: String,
    truncated: bool,
}

pub fn sanitize_git_output(text: &str) -> String {
    text.lines()
        .filter(|line| {
            !line
                .to_ascii_lowercase()
                .starts_with("warning: in the working copy of ")
        })
        .collect::<Vec<_>>()
        .join("\n")
        .replace("\n\n\n", "\n\n")
        .trim()
        .to_owned()
}

pub async fn run_git(options: GitRunOptions) -> GitRunResult {
    let cwd_path = options
        .cwd
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let cwd = cwd_path.to_string_lossy().to_string();
    let mut args = vec![
        "-c".to_owned(),
        GIT_CRLF_ADVICE.to_owned(),
        "-C".to_owned(),
        cwd.clone(),
    ];
    args.extend(options.args);
    let timeout_ms = clamp_timeout_ms(options.timeout_ms.or(Some(DEFAULT_TIMEOUT_MS)));
    let max_output_bytes =
        clamp_max_output_bytes(options.max_output_bytes.or(Some(DEFAULT_MAX_OUTPUT_BYTES)));
    let started = Instant::now();

    let mut command = Command::new("git");
    command
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return GitRunResult {
                ok: false,
                exit_code: None,
                timed_out: false,
                duration_ms: started.elapsed().as_millis(),
                stdout: String::new(),
                stderr: String::new(),
                stdout_truncated: false,
                stderr_truncated: false,
                command: GitCommandInfo {
                    executable: "git".to_owned(),
                    args,
                    cwd,
                },
                error: Some(error.to_string()),
            };
        }
    };

    let stdout_task = tokio::spawn(read_limited(child.stdout.take(), max_output_bytes));
    let stderr_task = tokio::spawn(read_limited(child.stderr.take(), max_output_bytes));
    let mut timed_out = false;
    let status = match tokio::time::timeout(Duration::from_millis(timeout_ms), child.wait()).await {
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

    GitRunResult {
        ok: !timed_out && exit_code == Some(0),
        exit_code,
        timed_out,
        duration_ms: started.elapsed().as_millis(),
        stdout: sanitize_git_output(&stdout.text),
        stderr: sanitize_git_output(&stderr.text),
        stdout_truncated: stdout.truncated,
        stderr_truncated: stderr.truncated,
        command: GitCommandInfo {
            executable: "git".to_owned(),
            args,
            cwd,
        },
        error: None,
    }
}

pub async fn git_status(cwd: Option<PathBuf>, porcelain: bool) -> GitRunResult {
    let mut args = vec!["status".to_owned()];
    if porcelain {
        args.push("--porcelain".to_owned());
    }
    run_git(GitRunOptions {
        args,
        cwd,
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await
}

pub async fn git_diff(
    cwd: Option<PathBuf>,
    staged: bool,
    reference: Option<String>,
) -> GitRunResult {
    let mut args = vec!["diff".to_owned()];
    if staged {
        args.push("--cached".to_owned());
    }
    if let Some(reference) = reference.filter(|value| !value.trim().is_empty()) {
        args.push(reference);
    }
    run_git(GitRunOptions {
        args,
        cwd,
        timeout_ms: None,
        max_output_bytes: Some(2 * DEFAULT_MAX_OUTPUT_BYTES),
    })
    .await
}

pub async fn git_log(cwd: Option<PathBuf>, count: Option<u64>, oneline: bool) -> GitRunResult {
    let mut args = vec![
        "log".to_owned(),
        "-n".to_owned(),
        count.unwrap_or(10).min(200).to_string(),
    ];
    if oneline {
        args.push("--oneline".to_owned());
    }
    run_git(GitRunOptions {
        args,
        cwd,
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await
}

pub async fn git_tree(
    cwd: Option<PathBuf>,
    count: Option<u64>,
    all: bool,
    reference: Option<String>,
) -> GitRunResult {
    let mut args = vec![
        "log".to_owned(),
        "--graph".to_owned(),
        "--oneline".to_owned(),
        "--decorate".to_owned(),
        "-n".to_owned(),
        count.unwrap_or(20).min(500).to_string(),
    ];
    if all {
        args.push("--all".to_owned());
    }
    if let Some(reference) = reference.filter(|value| !value.trim().is_empty()) {
        args.push(reference);
    }
    run_git(GitRunOptions {
        args,
        cwd,
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await
}

pub async fn git_branch(
    cwd: Option<PathBuf>,
    action: String,
    branch_name: Option<String>,
) -> GitRunResult {
    let args = match action.as_str() {
        "list" => vec!["branch".to_owned(), "-a".to_owned()],
        "create" => match branch_name.filter(|value| !value.trim().is_empty()) {
            Some(branch) => vec!["switch".to_owned(), "-c".to_owned(), branch],
            None => vec!["branch".to_owned(), "-a".to_owned()],
        },
        "switch" => match branch_name.filter(|value| !value.trim().is_empty()) {
            Some(branch) => vec!["switch".to_owned(), branch],
            None => vec!["branch".to_owned(), "-a".to_owned()],
        },
        _ => vec!["branch".to_owned(), "-a".to_owned()],
    };
    run_git(GitRunOptions {
        args,
        cwd,
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await
}

pub async fn git_stash(
    cwd: Option<PathBuf>,
    action: String,
    message: Option<String>,
    index: Option<u64>,
) -> GitRunResult {
    let args = match action.as_str() {
        "push" => {
            let mut args = vec!["stash".to_owned(), "push".to_owned()];
            if let Some(message) = message.filter(|value| !value.trim().is_empty()) {
                args.extend(["-m".to_owned(), message]);
            }
            args
        }
        "pop" => vec![
            "stash".to_owned(),
            "pop".to_owned(),
            format!("stash@{{{}}}", index.unwrap_or(0)),
        ],
        "drop" => vec![
            "stash".to_owned(),
            "drop".to_owned(),
            format!("stash@{{{}}}", index.unwrap_or(0)),
        ],
        _ => vec!["stash".to_owned(), "list".to_owned()],
    };
    run_git(GitRunOptions {
        args,
        cwd,
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await
}

pub async fn git_precommit_review(cwd: Option<PathBuf>, max_chars: Option<usize>) -> GitRunResult {
    let result = run_git(GitRunOptions {
        args: vec!["diff".to_owned(), "--cached".to_owned()],
        cwd: cwd.clone(),
        timeout_ms: None,
        max_output_bytes: max_chars.map(|value| value.clamp(1_024, 256_000)),
    })
    .await;
    if !result.ok || result.stdout.trim().is_empty() {
        return result;
    }

    let stat = run_git(GitRunOptions {
        args: vec![
            "diff".to_owned(),
            "--cached".to_owned(),
            "--stat".to_owned(),
        ],
        cwd,
        timeout_ms: None,
        max_output_bytes: Some(64 * 1024),
    })
    .await;
    let body = [
        "## Staged changes summary",
        stat.stdout.as_str(),
        "",
        "## Staged diff",
        result.stdout.as_str(),
        "",
        "Review checklist: logic errors, missing tests, secrets, unrelated files, commit scope.",
    ]
    .join("\n");

    GitRunResult {
        stdout: body,
        ..result
    }
}

pub async fn git_commit(
    cwd: Option<PathBuf>,
    message: String,
    files: Vec<String>,
    amend: bool,
) -> GitCommitResult {
    let normalized_message = normalize_commit_message(&message);
    if normalized_message.is_empty() {
        let add = empty_error(cwd.clone(), "commit message must not be empty");
        return GitCommitResult {
            ok: false,
            hash: None,
            subject: None,
            branch: None,
            stat: None,
            add,
            commit: None,
            error: Some("commit message must not be empty".to_owned()),
        };
    }

    let clean_files = files
        .into_iter()
        .map(|file| file.trim().to_owned())
        .filter(|file| !file.is_empty())
        .collect::<Vec<_>>();
    let mut add_args = vec!["add".to_owned()];
    if clean_files.is_empty() {
        add_args.push("-A".to_owned());
    } else {
        add_args.extend(clean_files);
    }
    let add = run_git(GitRunOptions {
        args: add_args,
        cwd: cwd.clone(),
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await;
    if !add.ok {
        return GitCommitResult {
            ok: false,
            hash: None,
            subject: None,
            branch: None,
            stat: None,
            add,
            commit: None,
            error: Some("git add failed".to_owned()),
        };
    }

    let msg_path = temp_commit_message_path();
    if let Err(error) = tokio::fs::write(&msg_path, normalized_message).await {
        return GitCommitResult {
            ok: false,
            hash: None,
            subject: None,
            branch: None,
            stat: None,
            add,
            commit: None,
            error: Some(error.to_string()),
        };
    }

    let mut commit_args = vec![
        "commit".to_owned(),
        "-F".to_owned(),
        msg_path.to_string_lossy().to_string(),
        "-q".to_owned(),
    ];
    if amend {
        commit_args.push("--amend".to_owned());
    }
    let commit = run_git(GitRunOptions {
        args: commit_args,
        cwd: cwd.clone(),
        timeout_ms: None,
        max_output_bytes: None,
    })
    .await;
    let _ = tokio::fs::remove_file(&msg_path).await;

    if !commit.ok {
        return GitCommitResult {
            ok: false,
            hash: None,
            subject: None,
            branch: None,
            stat: None,
            add,
            commit: Some(commit),
            error: Some("git commit failed".to_owned()),
        };
    }

    let hash = read_git_scalar(cwd.clone(), vec!["rev-parse", "--short", "HEAD"]).await;
    let subject = read_git_scalar(cwd.clone(), vec!["log", "-1", "--format=%s"]).await;
    let branch = read_git_scalar(cwd.clone(), vec!["branch", "--show-current"]).await;
    let stat = read_git_scalar(cwd, vec!["show", "--stat", "--format=", "HEAD"]).await;

    GitCommitResult {
        ok: true,
        hash,
        subject,
        branch,
        stat,
        add,
        commit: Some(commit),
        error: None,
    }
}

async fn read_git_scalar(cwd: Option<PathBuf>, args: Vec<&str>) -> Option<String> {
    let result = run_git(GitRunOptions {
        args: args.into_iter().map(ToOwned::to_owned).collect(),
        cwd,
        timeout_ms: None,
        max_output_bytes: Some(64 * 1024),
    })
    .await;
    result
        .ok
        .then(|| result.stdout.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn normalize_commit_message(message: &str) -> String {
    message.replace("\r\n", "\n").trim().to_owned()
}

fn temp_commit_message_path() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    env::temp_dir().join(format!(
        "nushell-mcp-git-commit-{}-{nanos}.txt",
        std::process::id()
    ))
}

fn empty_error(cwd: Option<PathBuf>, message: &str) -> GitRunResult {
    let cwd_path = cwd.unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    GitRunResult {
        ok: false,
        exit_code: None,
        timed_out: false,
        duration_ms: 0,
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        command: GitCommandInfo {
            executable: "git".to_owned(),
            args: Vec::new(),
            cwd: cwd_path.to_string_lossy().to_string(),
        },
        error: Some(message.to_owned()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_git_output_removes_crlf_warnings() {
        let output = [
            "warning: in the working copy of 'plan.md', LF will be replaced by CRLF the next time Git touches it",
            "warning: in the working copy of '.gitignore', LF will be replaced by CRLF the next time Git touches it",
            "real stderr line",
        ]
        .join("\n");

        assert_eq!(sanitize_git_output(&output), "real stderr line");
    }

    #[tokio::test]
    async fn run_git_suppresses_crlf_warnings() {
        let temp = tempfile::tempdir().unwrap();
        let repo = temp.path();
        std::fs::write(repo.join("file.txt"), "one\ntwo\n").unwrap();

        let init = run_git(GitRunOptions {
            args: vec!["init".to_owned(), "--quiet".to_owned()],
            cwd: Some(repo.to_path_buf()),
            timeout_ms: None,
            max_output_bytes: None,
        })
        .await;
        assert!(init.ok, "{init:?}");

        let config = run_git(GitRunOptions {
            args: vec![
                "config".to_owned(),
                "core.autocrlf".to_owned(),
                "true".to_owned(),
            ],
            cwd: Some(repo.to_path_buf()),
            timeout_ms: None,
            max_output_bytes: None,
        })
        .await;
        assert!(config.ok, "{config:?}");

        let add = run_git(GitRunOptions {
            args: vec!["add".to_owned(), "file.txt".to_owned()],
            cwd: Some(repo.to_path_buf()),
            timeout_ms: None,
            max_output_bytes: None,
        })
        .await;
        assert!(add.ok, "{add:?}");
        assert_eq!(add.stderr, "");
    }
}
