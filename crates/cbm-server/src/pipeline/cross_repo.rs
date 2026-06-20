//! Cross-repo intelligence (`mode=cross-repo-intelligence`, `target_projects`).
//!
//! MVP: match outbound HTTP path literals in the source project against
//! `HTTP_ROUTE` edges in target project graphs.

use crate::error::{Error, Result};
use crate::project::{normalize_project_name, project_name_from_path};
use crate::store::{Edge, Store, Symbol};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::time::Instant;

const CROSS_EDGE_TYPES: &[&str] = &[
    "CROSS_HTTP_CALLS",
    "CROSS_ASYNC_CALLS",
    "CROSS_CHANNEL",
    "CROSS_GRPC_CALLS",
    "CROSS_GRAPHQL_CALLS",
    "CROSS_TRPC_CALLS",
];

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrossRepoResult {
    pub status: &'static str,
    pub mode: &'static str,
    pub project: String,
    pub target_projects: Vec<String>,
    pub projects_scanned: usize,
    pub cross_http_calls: usize,
    pub cross_async_calls: usize,
    pub cross_channel: usize,
    pub cross_grpc_calls: usize,
    pub cross_graphql_calls: usize,
    pub cross_trpc_calls: usize,
    pub total_cross_edges: usize,
    pub elapsed_ms: f64,
}

pub fn parse_target_projects(args: &serde_json::Value) -> Result<Vec<String>> {
    let Some(arr) = args.get("target_projects").and_then(|v| v.as_array()) else {
        return Err(Error::InvalidArgument(
            "target_projects is required for cross-repo-intelligence mode. \
             Use [\"*\"] for all projects. Run list_projects to see available."
                .into(),
        ));
    };
    if arr.is_empty() {
        return Err(Error::InvalidArgument(
            "target_projects is required for cross-repo-intelligence mode. \
             Use [\"*\"] for all projects. Run list_projects to see available."
                .into(),
        ));
    }
    Ok(arr
        .iter()
        .filter_map(|v| v.as_str())
        .map(normalize_target_name)
        .collect())
}

fn normalize_target_name(name: &str) -> String {
    if name == "*" {
        name.to_string()
    } else {
        normalize_project_name(name)
    }
}

pub fn resolve_target_projects(source: &str, targets: &[String]) -> Result<Vec<String>> {
    if targets.len() == 1 && targets[0] == "*" {
        Ok(Store::list_projects()?
            .into_iter()
            .map(|p| p.name)
            .filter(|name| name.as_str() != source)
            .collect())
    } else {
        Ok(targets
            .iter()
            .filter(|name| name.as_str() != source)
            .cloned()
            .collect())
    }
}

pub fn run_cross_repo_intelligence(
    repo_path: &Path,
    project: Option<&str>,
    target_projects: &[String],
) -> Result<CrossRepoResult> {
    let start = Instant::now();
    let source_project = match project {
        Some(p) => normalize_project_name(p),
        None => project_name_from_path(repo_path),
    };

    let source = Store::open(&source_project).map_err(|_| {
        Error::InvalidArgument(format!(
            "project {source_project} is not indexed; run index_repository first"
        ))
    })?;

    delete_cross_edges(&source)?;

    let resolved = resolve_target_projects(&source_project, target_projects)?;
    let mut cross_http = 0usize;
    let mut scanned = 0usize;

    let outbound = collect_outbound_http_paths(&source)?;
    for target_name in &resolved {
        let Ok(target) = Store::open(target_name) else {
            continue;
        };
        let routes = collect_http_routes(&target);
        if routes.is_empty() {
            continue;
        }
        scanned += 1;
        for (caller_qn, path) in &outbound {
            let norm = normalize_route_path(path);
            let Some((route_qn, handler_qn)) = routes.get(&norm) else {
                continue;
            };
            let props = cross_edge_props(target_name, handler_qn, path);
            source.insert_edge(&Edge {
                src_qn: caller_qn.clone(),
                dst_qn: route_qn.clone(),
                edge_type: "CROSS_HTTP_CALLS".into(),
                properties_json: Some(props),
            })?;
            cross_http += 1;
        }
    }

    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    Ok(CrossRepoResult {
        status: "success",
        mode: "cross-repo-intelligence",
        project: source_project,
        target_projects: resolved,
        projects_scanned: scanned,
        cross_http_calls: cross_http,
        cross_async_calls: 0,
        cross_channel: 0,
        cross_grpc_calls: 0,
        cross_graphql_calls: 0,
        cross_trpc_calls: 0,
        total_cross_edges: cross_http,
        elapsed_ms,
    })
}

fn delete_cross_edges(store: &Store) -> Result<()> {
    for edge_type in CROSS_EDGE_TYPES {
        store.delete_edges_by_type(edge_type)?;
    }
    Ok(())
}

fn normalize_route_path(path: &str) -> String {
    let p = path.trim();
    if p.is_empty() {
        return "/".into();
    }
    if p.starts_with('/') {
        p.to_string()
    } else {
        format!("/{p}")
    }
}

fn collect_http_routes(store: &Store) -> HashMap<String, (String, String)> {
    let mut routes = HashMap::new();
    for edge in store.list_edges().unwrap_or_default() {
        if edge.edge_type != "HTTP_ROUTE" {
            continue;
        }
        let path = edge
            .properties_json
            .as_ref()
            .and_then(|p| extract_json_str(p, "path"))
            .or_else(|| route_path_from_qn(&edge.dst_qn))
            .unwrap_or_default();
        if !path.is_empty() {
            routes.insert(
                normalize_route_path(&path),
                (edge.dst_qn.clone(), edge.src_qn.clone()),
            );
        }
    }
    routes
}

fn route_path_from_qn(qn: &str) -> Option<String> {
    qn.split("::Route::")
        .nth(1)
        .and_then(|rest| rest.split('@').next())
        .map(normalize_route_path)
}

fn collect_outbound_http_paths(store: &Store) -> Result<Vec<(String, String)>> {
    let symbols: Vec<Symbol> = store
        .list_symbols()?
        .into_iter()
        .filter(|s| s.label == "Function")
        .collect();
    let url_patterns = [
        Regex::new(r#"(?i)(?:fetch|axios|request|httpx?|reqwest)[^"';\n]*["']([^"']+)["']"#)
            .unwrap(),
        Regex::new(r#"(?i)\.(get|post|put|delete|patch)\(\s*["']([^"']+)["']"#).unwrap(),
        Regex::new(r#"["'](/[\w./-]+)["']"#).unwrap(),
    ];

    let mut hits = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for file in store.list_files()? {
        let file_symbols: Vec<&Symbol> = symbols
            .iter()
            .filter(|s| s.file_path == file.path)
            .collect();
        for (line_no, line) in file.content.lines().enumerate() {
            let line_i = (line_no + 1) as i64;
            for re in &url_patterns {
                for cap in re.captures_iter(line) {
                    let path = cap
                        .get(2)
                        .or_else(|| cap.get(1))
                        .map(|m| m.as_str())
                        .unwrap_or("");
                    if !path.starts_with('/') {
                        continue;
                    }
                    let Some(caller) = function_at_line(&file_symbols, line_i) else {
                        continue;
                    };
                    let key = (caller.clone(), normalize_route_path(path));
                    if seen.insert(key.clone()) {
                        hits.push(key);
                    }
                }
            }
        }
    }
    Ok(hits)
}

fn function_at_line(symbols: &[&Symbol], line: i64) -> Option<String> {
    symbols
        .iter()
        .filter(|s| line >= s.line_start && line <= s.line_end)
        .max_by_key(|s| s.line_start)
        .map(|s| s.qualified_name.clone())
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pattern = format!(r#""{key}":"([^"]+)""#);
    Regex::new(&pattern)
        .ok()?
        .captures(json)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn cross_edge_props(target_project: &str, handler_qn: &str, path: &str) -> String {
    let handler = handler_qn
        .split("::Function::")
        .nth(1)
        .and_then(|s| s.split('@').next())
        .unwrap_or(handler_qn);
    format!(
        r#"{{"target_project":"{target_project}","target_function":"{handler}","url_path":"{path}"}}"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discover::IndexMode;
    use crate::pipeline::Pipeline;
    use crate::test_lock;
    use tempfile::TempDir;

    #[test]
    fn parse_target_projects_rejects_empty() {
        let args = serde_json::json!({"target_projects": []});
        assert!(parse_target_projects(&args).is_err());
    }

    #[test]
    fn resolve_star_expands_list_projects() {
        let _guard = test_lock::acquire();
        let dir = tempfile::TempDir::new().unwrap();
        std::env::set_var("CBRLM_CACHE_DIR", dir.path());

        let client = TempDir::new().unwrap();
        std::fs::write(
            client.path().join("client.py"),
            "import requests\ndef call_users():\n    requests.get('/users')\n",
        )
        .unwrap();
        Pipeline::new(IndexMode::Full)
            .run(client.path(), Some("client-svc"))
            .unwrap();

        let server = TempDir::new().unwrap();
        std::fs::write(
            server.path().join("api.py"),
            "@app.get('/users')\ndef list_users():\n    pass\n",
        )
        .unwrap();
        Pipeline::new(IndexMode::Full)
            .run(server.path(), Some("server-svc"))
            .unwrap();

        let source = normalize_project_name("client-svc");
        let resolved = resolve_target_projects(&source, &["*".to_string()]).unwrap();
        assert!(
            resolved.iter().any(|p| p.contains("server-svc")),
            "resolved targets: {resolved:?}"
        );

        let result = run_cross_repo_intelligence(
            client.path(),
            Some("client-svc"),
            &[normalize_target_name("server-svc")],
        )
        .unwrap();
        assert_eq!(result.status, "success");
        assert!(result.cross_http_calls >= 1, "{result:?}");

        let store = Store::open(&source).unwrap();
        let cross: Vec<_> = store
            .list_edges()
            .unwrap()
            .into_iter()
            .filter(|e| e.edge_type == "CROSS_HTTP_CALLS")
            .collect();
        assert!(!cross.is_empty(), "expected CROSS_HTTP_CALLS edges");

        let _ = crate::store::delete_project_db(&source);
        let server = normalize_target_name("server-svc");
        let _ = crate::store::delete_project_db(&server);
    }
}
