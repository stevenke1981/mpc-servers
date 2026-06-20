//! Compare runtime MCP tool definitions against checked-in specs under `mcps/`.

use codebase_memory_mcp::mcp::McpServer;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn load_checked_in_specs() -> HashMap<String, Value> {
    let dir = repo_root().join("mcps/codebase-memory-mcp/tools");
    let mut specs = HashMap::new();
    for entry in fs::read_dir(&dir).expect("mcps tool specs directory missing") {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.starts_with("rlm_"))
        {
            continue;
        }
        let raw = fs::read_to_string(&path).unwrap();
        let value: Value = serde_json::from_str(&raw).unwrap();
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .expect("spec missing name")
            .to_string();
        specs.insert(name, value);
    }
    specs
}

fn required_fields(schema: &Value) -> BTreeSet<String> {
    schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn property_keys(schema: &Value) -> BTreeSet<String> {
    schema
        .get("properties")
        .and_then(|v| v.as_object())
        .map(|obj| obj.keys().cloned().collect())
        .unwrap_or_default()
}

fn assert_no_boolean_property_schemas(value: &Value, path: &str) {
    match value {
        Value::Object(object) => {
            if let Some(properties) = object.get("properties").and_then(Value::as_object) {
                for (name, schema) in properties {
                    assert!(
                        !schema.is_boolean(),
                        "OpenCode rejects boolean schema node at {path}.properties.{name}"
                    );
                }
            }
            if object.get("items").is_some_and(Value::is_boolean) {
                panic!("OpenCode rejects boolean schema node at {path}.items");
            }
            for (key, child) in object {
                assert_no_boolean_property_schemas(child, &format!("{path}.{key}"));
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                assert_no_boolean_property_schemas(child, &format!("{path}[{index}]"));
            }
        }
        _ => {}
    }
}

fn compare_tool(runtime: &Value, spec: &Value, name: &str) -> Vec<String> {
    let mut errors = Vec::new();

    let rt_desc = runtime
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let spec_desc = spec
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if rt_desc != spec_desc {
        errors.push(format!("{name}: description mismatch (runtime vs spec)"));
    }

    let rt_schema = runtime.get("inputSchema").unwrap_or(&Value::Null);
    let spec_schema = spec.get("inputSchema").unwrap_or(&Value::Null);

    let rt_required = required_fields(rt_schema);
    let spec_required = required_fields(spec_schema);
    if rt_required != spec_required {
        errors.push(format!(
            "{name}: required mismatch runtime={rt_required:?} spec={spec_required:?}"
        ));
    }

    let rt_props = property_keys(rt_schema);
    let spec_props = property_keys(spec_schema);
    for key in spec_props.difference(&rt_props) {
        errors.push(format!(
            "{name}: runtime schema missing property `{key}` from spec"
        ));
    }

    errors
}

#[test]
fn runtime_tools_match_checked_in_specs() {
    let runtime: Vec<Value> = McpServer::generated_tool_definitions();
    let specs = load_checked_in_specs();

    let runtime_names: BTreeSet<_> = runtime
        .iter()
        .filter_map(|t| t.get("name").and_then(|v| v.as_str()))
        .collect();
    let spec_names: BTreeSet<_> = specs.keys().map(String::as_str).collect();

    assert_eq!(
        runtime_names, spec_names,
        "tool name set drift: runtime={runtime_names:?} spec={spec_names:?}"
    );

    let mut all_errors = Vec::new();
    for tool in &runtime {
        let name = tool.get("name").and_then(|v| v.as_str()).unwrap();
        let spec = specs.get(name).unwrap();
        all_errors.extend(compare_tool(tool, spec, name));
        assert_no_boolean_property_schemas(&tool["inputSchema"], name);
    }

    assert!(
        all_errors.is_empty(),
        "MCP tool schema drift:\n{}",
        all_errors.join("\n")
    );
}
