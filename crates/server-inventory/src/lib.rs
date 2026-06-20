use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SourceKind {
    UpstreamReference,
    ExistingRustServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReuseDecision {
    DirectUse,
    ImplementedPort,
    PortFromReference,
    ReusePattern,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ServerTarget {
    pub name: &'static str,
    pub source: &'static str,
    pub source_kind: SourceKind,
    pub upstream_version: Option<&'static str>,
    pub local_version: Option<&'static str>,
    pub decision: ReuseDecision,
    pub notes: &'static str,
}

pub const SERVER_TARGETS: &[ServerTarget] = &[
    ServerTarget {
        name: "memory",
        source: "stevenke1981/memlong",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: Some("0.6.3"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Imported from memlong as memory-core, memory-mcp-server, and memory-cli; preserves 7 long-term memory MCP tools and local-first storage.",
    },
    ServerTarget {
        name: "rlm",
        source: "stevenke1981/rlm-mcp",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: None,
        local_version: Some("0.1.6"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Imported in crates/rlm-server as the rlm-mcp package; preserves 33 rlm_* tools, safe REPL opt-in behavior, and original tests.",
    },
    ServerTarget {
        name: "cbm",
        source: "stevenke1981/cbm-mcp",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: None,
        local_version: Some("0.2.3"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Imported in crates/cbm-server as the codebase-memory-mcp package; preserves cbm binary, 14 tools, schema normalization, and original tests.",
    },
    ServerTarget {
        name: "nushell",
        source: "stevenke1981/nushell-mcp",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: None,
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Imported in crates/nushell-server as the nushell-mcp package; preserves 15 Nu/Git tools, bounded process execution, and original tests.",
    },
    ServerTarget {
        name: "filesystem",
        source: "stevenke1981/servers/src/filesystem",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.3"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Implemented in crates/filesystem-server with 14 filesystem tools, path safety, and MCP Roots dynamic updates.",
    },
    ServerTarget {
        name: "git",
        source: "stevenke1981/servers/src/git",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.2"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Implemented in crates/git-server with 12 Git tools, repository validation, and native git argv execution.",
    },
    ServerTarget {
        name: "time",
        source: "stevenke1981/servers/src/time",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.2"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Implemented in crates/time-server with get_current_time and convert_time.",
    },
    ServerTarget {
        name: "fetch",
        source: "stevenke1981/servers/src/fetch",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.3"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Implemented in crates/fetch-server with bounded public-web fetching and SSRF protections.",
    },
    ServerTarget {
        name: "sequential-thinking",
        source: "stevenke1981/servers/src/sequentialthinking",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.2"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Implemented in crates/sequential-thinking-server with session-scoped thought and branch state.",
    },
    ServerTarget {
        name: "everything",
        source: "stevenke1981/servers/src/everything",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("2.0.0"),
        local_version: Some("0.1.0"),
        decision: ReuseDecision::ImplementedPort,
        notes: "Implemented in crates/everything-server as an MCP compatibility testbed with 19 tools, prompts, resources, templates, and documented protocol feature fallbacks.",
    },
];

pub fn targets() -> &'static [ServerTarget] {
    SERVER_TARGETS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_includes_all_upstream_reference_servers() {
        let names: Vec<_> = targets().iter().map(|target| target.name).collect();

        for expected in [
            "everything",
            "fetch",
            "filesystem",
            "git",
            "memory",
            "sequential-thinking",
            "time",
        ] {
            assert!(names.contains(&expected), "missing target: {expected}");
        }
    }

    #[test]
    fn direct_reuse_entries_have_local_versions() {
        for target in targets()
            .iter()
            .filter(|target| target.decision == ReuseDecision::DirectUse)
        {
            assert!(
                target.local_version.is_some(),
                "direct reuse target must carry a local version: {}",
                target.name
            );
        }
    }
}
