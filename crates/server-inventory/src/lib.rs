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
        decision: ReuseDecision::DirectUse,
        notes: "Use memlong as the Rust memory line; align rmcp and release packaging before merging.",
    },
    ServerTarget {
        name: "rlm",
        source: "stevenke1981/rlm-mcp",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: None,
        local_version: Some("0.1.6"),
        decision: ReuseDecision::DirectUse,
        notes: "Standalone Rust rmcp server; keep as an independent product in the workspace.",
    },
    ServerTarget {
        name: "codebase-memory",
        source: "stevenke1981/cbm-mcp",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: None,
        local_version: Some("0.2.3"),
        decision: ReuseDecision::DirectUse,
        notes: "Reuse release, installer, schema-normalization, and OpenCode/Codex smoke patterns.",
    },
    ServerTarget {
        name: "nushell",
        source: "stevenke1981/nushell-mcp",
        source_kind: SourceKind::ExistingRustServer,
        upstream_version: None,
        local_version: Some("0.1.0"),
        decision: ReuseDecision::DirectUse,
        notes: "Useful local shell MCP server; not a direct upstream servers replacement.",
    },
    ServerTarget {
        name: "filesystem",
        source: "stevenke1981/servers/src/filesystem",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.3"),
        local_version: None,
        decision: ReuseDecision::PortFromReference,
        notes: "Port access-control, roots support, path validation, and structured content tests.",
    },
    ServerTarget {
        name: "git",
        source: "stevenke1981/servers/src/git",
        source_kind: SourceKind::UpstreamReference,
        upstream_version: Some("0.6.2"),
        local_version: None,
        decision: ReuseDecision::PortFromReference,
        notes: "Port git operations with strict repository boundaries and native argument handling on Windows.",
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
        local_version: None,
        decision: ReuseDecision::PortFromReference,
        notes: "Port HTTP fetch and content extraction after security policy is defined.",
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
        local_version: None,
        decision: ReuseDecision::ReusePattern,
        notes: "Use as protocol feature testbed for prompts, resources, tools, and transports.",
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
