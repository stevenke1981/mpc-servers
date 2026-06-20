use std::path::PathBuf;

use crate::nu::{NuRunMode, NuRunOptions, NuRunResult, run_nushell};

#[derive(Debug, Clone)]
pub struct NuToolCommon {
    pub cwd: Option<PathBuf>,
    pub timeout_ms: Option<u64>,
    pub max_output_bytes: Option<usize>,
    pub nu_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NuGrepOptions {
    pub pattern: String,
    pub path: Option<String>,
    pub recursive: bool,
    pub ignore_case: bool,
    pub line_number: bool,
    pub max_lines: Option<u64>,
    pub common: NuToolCommon,
}

#[derive(Debug, Clone)]
pub struct NuFindOptions {
    pub path: Option<String>,
    pub name: Option<String>,
    pub extension: Option<String>,
    pub entry_type: Option<String>,
    pub recursive: bool,
    pub max_results: Option<u64>,
    pub common: NuToolCommon,
}

#[derive(Debug, Clone)]
pub struct NuReadOptions {
    pub file: String,
    pub mode: String,
    pub lines: Option<u64>,
    pub offset: Option<u64>,
    pub line_numbers: bool,
    pub common: NuToolCommon,
}

#[derive(Debug, Clone)]
pub struct NuLsOptions {
    pub path: Option<String>,
    pub all: bool,
    pub long: bool,
    pub common: NuToolCommon,
}

pub async fn nu_grep(options: NuGrepOptions) -> NuRunResult {
    let path = options.path.unwrap_or_else(|| ".".to_owned());
    let max_lines = options.max_lines.unwrap_or(200).min(10_000);
    let pattern = if options.ignore_case {
        format!("(?i){}", options.pattern)
    } else {
        options.pattern
    };
    let command = if options.recursive {
        format!(
            "glob {glob_pattern} | each {{|p| try {{ open --raw $p | lines | enumerate | where item =~ {pattern} | each {{|row| if {line_number} {{ $'($p):(($row.index + 1)):($row.item)' }} else {{ $'($p):($row.item)' }} }} }} catch {{ [] }} }} | flatten | first {max_lines}",
            glob_pattern = nu_string(&join_glob(&path)),
            pattern = nu_string(&pattern),
            line_number = options.line_number,
        )
    } else {
        format!(
            "open --raw {path} | lines | enumerate | where item =~ {pattern} | each {{|row| if {line_number} {{ $'(($row.index + 1)):($row.item)' }} else {{ $row.item }} }} | first {max_lines}",
            path = nu_string(&path),
            pattern = nu_string(&pattern),
            line_number = options.line_number,
        )
    };
    run_tool_command(command, options.common).await
}

pub async fn nu_find(options: NuFindOptions) -> NuRunResult {
    let path = options.path.unwrap_or_else(|| ".".to_owned());
    let max_results = options.max_results.unwrap_or(200).min(10_000);
    let glob = if options.recursive {
        join_glob(&path)
    } else {
        format!("{}/*", path.trim_end_matches(['/', '\\']))
    };
    let mut filters = Vec::new();
    if let Some(name) = options.name.filter(|value| !value.trim().is_empty()) {
        filters.push(format!("($it | path basename) =~ {}", nu_string(&name)));
    }
    if let Some(extension) = options.extension.filter(|value| !value.trim().is_empty()) {
        let normalized = extension.trim_start_matches('.');
        filters.push(format!(
            "($it | path parse | get extension) == {}",
            nu_string(normalized)
        ));
    }
    if let Some(entry_type) = options.entry_type.filter(|value| value != "any") {
        if entry_type == "file" {
            filters.push("($it | path type) == file".to_owned());
        } else if entry_type == "directory" {
            filters.push("($it | path type) == dir".to_owned());
        }
    }
    let filter = if filters.is_empty() {
        String::new()
    } else {
        format!(" | where {{|it| {} }}", filters.join(" and "))
    };
    let command = format!(
        "glob {glob}{filter} | first {max_results}",
        glob = nu_string(&glob),
    );
    run_tool_command(command, options.common).await
}

pub async fn nu_read(options: NuReadOptions) -> NuRunResult {
    let lines = options.lines.unwrap_or(50).min(20_000);
    let offset = options.offset.unwrap_or(0);
    let base = format!("open --raw {} | lines", nu_string(&options.file));
    let sliced = match options.mode.as_str() {
        "tail" => format!("{base} | last {lines}"),
        "cat" => format!("{base} | skip {offset} | first {lines}"),
        _ => format!("{base} | skip {offset} | first {lines}"),
    };
    let command = if options.line_numbers {
        format!(
            "{sliced} | enumerate | each {{|row| $'(($row.index + 1 + {offset})):($row.item)' }}"
        )
    } else {
        sliced
    };
    run_tool_command(command, options.common).await
}

pub async fn nu_ls(options: NuLsOptions) -> NuRunResult {
    let path = options.path.unwrap_or_else(|| ".".to_owned());
    let mut command = "ls".to_owned();
    if options.all {
        command.push_str(" --all");
    }
    command.push(' ');
    command.push_str(&nu_string(&path));
    if !options.long {
        command.push_str(" | select name type size modified");
    }
    run_tool_command(command, options.common).await
}

async fn run_tool_command(command: String, common: NuToolCommon) -> NuRunResult {
    run_nushell(NuRunOptions {
        mode: NuRunMode::Eval { command },
        cwd: common.cwd,
        stdin: None,
        timeout_ms: common.timeout_ms,
        max_output_bytes: common.max_output_bytes,
        nu_path: common.nu_path,
    })
    .await
}

fn join_glob(path: &str) -> String {
    format!("{}/**/*", path.trim_end_matches(['/', '\\']))
}

fn nu_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}
