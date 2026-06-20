use crate::discover::IndexMode;
use crate::error::{Error, Result};
use crate::git;
use crate::pipeline::Pipeline;
use crate::project::normalize_project_name;
use crate::semantic;
use crate::store::{delete_project_db, SearchFilter, Store};
use crate::watcher::Watcher;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone)]
pub struct ToolHandler {
    watcher: Option<Arc<Watcher>>,
}

impl ToolHandler {
    pub fn new(watcher: Option<Arc<Watcher>>) -> Self {
        Self { watcher }
    }

    pub fn handle(&self, name: &str, args: &Value) -> Result<Value> {
        match name {
            "index_repository" => self.index_repository(args),
            "search_graph" => self.search_graph(args),
            "trace_path" => self.trace_path(args),
            "get_code_snippet" => self.get_code_snippet(args),
            "get_graph_schema" => self.get_graph_schema(args),
            "get_architecture" => self.get_architecture(args),
            "search_code" => self.search_code(args),
            "list_projects" => self.list_projects(),
            "delete_project" => self.delete_project(args),
            "index_status" => self.index_status(args),
            "query_graph" => self.query_graph(args),
            "detect_changes" => self.detect_changes(args),
            "manage_adr" => self.manage_adr(args),
            "ingest_traces" => self.ingest_traces(args),
            _ => Err(Error::InvalidArgument(format!("unknown tool: {name}"))),
        }
    }

    fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
        args.get(key)
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::InvalidArgument(format!("missing {key}")))
    }

    fn index_repository(&self, args: &Value) -> Result<Value> {
        let repo_path = Self::require_str(args, "repo_path")?;
        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
        let project = args.get("project").and_then(|v| v.as_str());
        let path = std::path::Path::new(repo_path);

        if mode.eq_ignore_ascii_case("cross-repo-intelligence") {
            let targets = crate::pipeline::parse_target_projects(args)?;
            let result = crate::pipeline::run_cross_repo_intelligence(path, project, &targets)?;
            return Ok(serde_json::to_value(result)?);
        }

        let incremental = args
            .get("incremental")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let persistence = args
            .get("persistence")
            .and_then(|v| v.as_bool())
            .unwrap_or_else(crate::persistence::env_enabled);
        let pipeline = Pipeline::new(IndexMode::parse(mode)).set_export_artifact(persistence);
        let _guard = self
            .watcher
            .as_ref()
            .map(|w| PipelineGuard::new(w.pipeline_busy()));
        let result = if incremental {
            pipeline.run_smart(path, project, true)?
        } else {
            pipeline.run(path, project)?
        };

        let project_name = &result.project;
        if let Some(w) = &self.watcher {
            w.register(
                project_name,
                path.canonicalize().unwrap_or_else(|_| path.to_path_buf()),
            );
        }

        Ok(serde_json::to_value(result)?)
    }

    fn search_graph(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;

        if let Some(vector_query) = args
            .get("vector_query")
            .or_else(|| args.get("semantic_query"))
            .and_then(|v| v.as_str())
        {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
            let result = semantic::vector_search(&store, vector_query, limit)?;
            return Ok(serde_json::to_value(result)?);
        }

        let filter = parse_search_filter(args);
        let result = store.search(&filter)?;
        Ok(serde_json::to_value(result)?)
    }

    fn trace_path(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let function_name = Self::require_str(args, "function_name")?;
        let direction = args
            .get("direction")
            .and_then(|v| v.as_str())
            .unwrap_or("both");
        let depth = args.get("depth").and_then(|v| v.as_u64()).unwrap_or(3) as usize;
        let store = Store::open(&project)?;
        let result = store.trace_path(function_name, direction, depth)?;
        Ok(serde_json::to_value(result)?)
    }

    fn get_code_snippet(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let qn = Self::require_str(args, "qualified_name")?;
        let store = Store::open(&project)?;
        let snippet = store.get_snippet(qn)?;
        Ok(serde_json::to_value(snippet)?)
    }

    fn get_graph_schema(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;
        Ok(json!(store.get_schema()))
    }

    fn get_architecture(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;
        let arch = store.get_architecture()?;
        Ok(serde_json::to_value(arch)?)
    }

    fn search_code(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let pattern = Self::require_str(args, "pattern")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as usize;
        let store = Store::open(&project)?;
        let matches = store.search_code(pattern, limit)?;
        Ok(json!({ "matches": matches }))
    }

    fn list_projects(&self) -> Result<Value> {
        let projects = Store::list_projects()?;
        Ok(json!({ "projects": projects }))
    }

    fn delete_project(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        if let Ok(store) = Store::open(&project) {
            store.delete_project()?;
        }
        delete_project_db(&project)?;
        Ok(json!({ "deleted": project }))
    }

    fn index_status(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;
        let status = store.index_status()?;
        let mut value = serde_json::to_value(status)?;
        if let Some(watcher) = &self.watcher {
            let projects = watcher.project_status();
            if let Some(w) = projects.iter().find(|p| p.project == project) {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert("watcher".into(), serde_json::to_value(w)?);
                }
            }
        }
        Ok(value)
    }

    fn query_graph(&self, args: &Value) -> Result<Value> {
        let query = Self::require_str(args, "query")?;
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;
        let result = store.query_select(query)?;
        Ok(serde_json::to_value(result)?)
    }

    fn detect_changes(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;
        let info = store.get_project()?;
        let repo = PathBuf::from(&info.repo_path);
        let indexed_head = store.get_meta("git_head")?;

        match git::status(&repo) {
            Ok(st) => {
                let changed = git::collect_incremental_paths(&repo, indexed_head.as_deref(), &st);
                let hash_drift = store
                    .files_with_fingerprint_drift(&repo)
                    .unwrap_or_default();
                Ok(json!({
                    "project": project,
                    "dirty": st.dirty,
                    "head": st.head,
                    "indexed_head": indexed_head,
                    "head_changed": indexed_head.as_ref().zip(st.head.as_ref()).map(|(a, b)| a != b).unwrap_or(false),
                    "changed_files": changed,
                    "deleted_files": st.deleted_files,
                    "hash_drift_files": hash_drift,
                    "needs_reindex": !changed.is_empty() || !hash_drift.is_empty(),
                }))
            }
            Err(e) => Ok(json!({
                "project": project,
                "dirty": false,
                "changed_files": [],
                "note": e.to_string()
            })),
        }
    }

    fn ingest_traces(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let store = Store::open(&project)?;
        let traces = args
            .get("traces")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::InvalidArgument("traces array required".into()))?;

        let mut pairs = Vec::new();
        for item in traces {
            let src = item
                .get("caller")
                .or_else(|| item.get("from"))
                .or_else(|| item.get("src"))
                .and_then(|v| v.as_str());
            let dst = item
                .get("callee")
                .or_else(|| item.get("to"))
                .or_else(|| item.get("dst"))
                .and_then(|v| v.as_str());
            if let (Some(s), Some(d)) = (src, dst) {
                pairs.push((s.to_string(), d.to_string()));
            }
        }

        let ingested = store.ingest_traces(&pairs)?;
        Ok(json!({
            "success": true,
            "project": project,
            "ingested": ingested,
            "edge_type": "RUNTIME_TRACE"
        }))
    }

    fn manage_adr(&self, args: &Value) -> Result<Value> {
        let project = normalize_project_name(Self::require_str(args, "project")?);
        let mode = args
            .get("mode")
            .or_else(|| args.get("action"))
            .and_then(|v| v.as_str())
            .unwrap_or("get");
        let store = Store::open(&project)?;
        match mode {
            "set" | "update" | "store" => {
                let content = Self::require_str(args, "content")?;
                store.set_adr(content)?;
                Ok(json!({ "status": "updated" }))
            }
            "delete" => {
                store.set_meta("adr", "")?;
                Ok(json!({ "status": "deleted" }))
            }
            "sections" => {
                let adr = store.get_adr()?;
                let sections = adr_list_sections(adr.as_deref().unwrap_or(""));
                Ok(json!({ "sections": sections }))
            }
            _ => {
                let adr = store.get_adr()?;
                if let Some(content) = adr.filter(|c| !c.is_empty()) {
                    Ok(json!({ "content": content }))
                } else {
                    Ok(json!({
                        "content": "",
                        "status": "no_adr",
                        "adr_hint": ADR_EMPTY_HINT
                    }))
                }
            }
        }
    }
}

const ADR_EMPTY_HINT: &str = "No ADR yet. Create one with manage_adr(mode='update', \
content='## PURPOSE\\n...\\n\\n## STACK\\n...\\n\\n## ARCHITECTURE\\n...\\n\\n## PATTERNS\\n...\\n\\n## TRADEOFFS\\n...\\n\\n## PHILOSOPHY\\n...'). \
For guided creation: explore the codebase with get_architecture, then draft and store. \
Sections: PURPOSE, STACK, ARCHITECTURE, PATTERNS, TRADEOFFS, PHILOSOPHY.";

fn adr_list_sections(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim_end_matches('\r'))
        .filter(|line| line.starts_with('#'))
        .map(str::to_string)
        .collect()
}

fn parse_search_filter(args: &Value) -> SearchFilter {
    SearchFilter {
        query: args
            .get("query")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        label: args
            .get("label")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        name_pattern: args
            .get("name_pattern")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        qn_pattern: args
            .get("qn_pattern")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        file_pattern: args
            .get("file_pattern")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        relationship: args
            .get("relationship")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        direction: args
            .get("direction")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        min_degree: args
            .get("min_degree")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        max_degree: args
            .get("max_degree")
            .and_then(|v| v.as_u64())
            .map(|n| n as usize),
        include_connected: args
            .get("include_connected")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        exclude_entry_points: args
            .get("exclude_entry_points")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        limit: args.get("limit").and_then(|v| v.as_u64()).unwrap_or(200) as usize,
        offset: args.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
    }
}

struct PipelineGuard {
    busy: Arc<std::sync::atomic::AtomicBool>,
}

impl PipelineGuard {
    fn new(busy: Arc<std::sync::atomic::AtomicBool>) -> Self {
        busy.store(true, std::sync::atomic::Ordering::SeqCst);
        Self { busy }
    }
}

impl Drop for PipelineGuard {
    fn drop(&mut self) {
        self.busy.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
