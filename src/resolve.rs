use anyhow::{bail, Result};

use crate::graph::Graph;

const FNV_OFFSET: u32 = 2166136261;
const FNV_PRIME: u32 = 16777619;
const MASK_28: u32 = 0x0FFF_FFFF;

/// FNV-1a hash → lower 28 bits → 7 hex chars.
pub fn short_hash(id: &str) -> String {
    let mut h = FNV_OFFSET;
    for b in id.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(FNV_PRIME);
    }
    format!("{:07x}", h & MASK_28)
}

/// Resolve a user-provided input to a canonical node ID.
///
/// Priority: exact match → short hash match → unique prefix match.
pub fn resolve(graph: &Graph, input: &str) -> Result<String> {
    // 1. Exact match
    if graph.nodes.iter().any(|n| n.id == input) {
        return Ok(input.to_string());
    }

    // 2. Short hash match
    let hash_matches: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| short_hash(&n.id) == input)
        .map(|n| n.id.as_str())
        .collect();
    match hash_matches.len() {
        1 => return Ok(hash_matches[0].to_string()),
        n if n > 1 => bail!(
            "ambiguous short hash '{}', matches {} nodes:\n  {}",
            input,
            n,
            hash_matches.join("\n  ")
        ),
        _ => {}
    }

    // 3. Unique prefix match on full ID
    let prefix_matches: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| n.id.starts_with(input))
        .map(|n| n.id.as_str())
        .collect();
    match prefix_matches.len() {
        1 => return Ok(prefix_matches[0].to_string()),
        n if n > 1 => bail!(
            "ambiguous prefix '{}', matches {} nodes:\n  {}",
            input,
            n,
            prefix_matches.join("\n  ")
        ),
        _ => {}
    }

    // 4. Slug match (portion after # in node IDs)
    let slug_exact: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| n.id.rsplit_once('#').map(|(_, s)| s) == Some(input))
        .map(|n| n.id.as_str())
        .collect();
    match slug_exact.len() {
        1 => return Ok(slug_exact[0].to_string()),
        n if n > 1 => bail!(
            "ambiguous slug '{}', matches {} nodes:\n  {}",
            input,
            n,
            slug_exact.join("\n  ")
        ),
        _ => {}
    }

    // 5. Slug prefix match
    let slug_prefix: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| {
            n.id
                .rsplit_once('#')
                .map(|(_, s)| s.starts_with(input))
                .unwrap_or(false)
        })
        .map(|n| n.id.as_str())
        .collect();
    match slug_prefix.len() {
        1 => return Ok(slug_prefix[0].to_string()),
        n if n > 1 => bail!(
            "ambiguous slug prefix '{}', matches {} nodes:\n  {}",
            input,
            n,
            slug_prefix.join("\n  ")
        ),
        _ => {}
    }

    bail!("no node matching '{}'", input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Node, NodeKind};

    fn make_graph(ids: &[&str]) -> Graph {
        let nodes = ids
            .iter()
            .map(|id| Node {
                id: id.to_string(),
                kind: NodeKind::Section,
                source: "test".into(),
                label: id.to_string(),
                metadata: None,
            })
            .collect();
        Graph {
            nodes,
            edges: vec![],
        }
    }

    #[test]
    fn hash_deterministic() {
        let a = short_hash("ROADMAP.md#some-task");
        let b = short_hash("ROADMAP.md#some-task");
        assert_eq!(a, b);
    }

    #[test]
    fn hash_7_hex_chars() {
        let h = short_hash("anything");
        assert_eq!(h.len(), 7);
        assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn resolve_exact_match() {
        let g = make_graph(&["foo#bar", "foo#baz"]);
        assert_eq!(resolve(&g, "foo#bar").unwrap(), "foo#bar");
    }

    #[test]
    fn resolve_short_hash() {
        let g = make_graph(&["ROADMAP.md#some-long-task-id"]);
        let h = short_hash("ROADMAP.md#some-long-task-id");
        assert_eq!(
            resolve(&g, &h).unwrap(),
            "ROADMAP.md#some-long-task-id"
        );
    }

    #[test]
    fn resolve_prefix() {
        let g = make_graph(&["ROADMAP.md#unique-task", "src/main.rs"]);
        assert_eq!(
            resolve(&g, "ROADMAP.md#unique").unwrap(),
            "ROADMAP.md#unique-task"
        );
    }

    #[test]
    fn resolve_ambiguous_prefix() {
        let g = make_graph(&["ROADMAP.md#stage-1", "ROADMAP.md#stage-2"]);
        let err = resolve(&g, "ROADMAP.md#stage-").unwrap_err();
        assert!(err.to_string().contains("ambiguous prefix"));
        assert!(err.to_string().contains("ROADMAP.md#stage-1"));
        assert!(err.to_string().contains("ROADMAP.md#stage-2"));
    }

    #[test]
    fn resolve_no_match() {
        let g = make_graph(&["foo#bar"]);
        let err = resolve(&g, "nonexistent").unwrap_err();
        assert!(err.to_string().contains("no node matching"));
    }

    #[test]
    fn resolve_exact_over_prefix() {
        // "foo" is both an exact match and a prefix of "foo#bar"
        let g = make_graph(&["foo", "foo#bar"]);
        assert_eq!(resolve(&g, "foo").unwrap(), "foo");
    }

    #[test]
    fn resolve_slug_exact() {
        let g = make_graph(&["ROADMAP.md#scan-project-files-into-graph-json"]);
        assert_eq!(
            resolve(&g, "scan-project-files-into-graph-json").unwrap(),
            "ROADMAP.md#scan-project-files-into-graph-json"
        );
    }

    #[test]
    fn resolve_slug_prefix() {
        let g = make_graph(&["ROADMAP.md#stage-1-graph-foundation"]);
        assert_eq!(
            resolve(&g, "stage-1").unwrap(),
            "ROADMAP.md#stage-1-graph-foundation"
        );
    }

    #[test]
    fn resolve_ambiguous_slug_prefix() {
        let g = make_graph(&[
            "ROADMAP.md#stage-1-graph-foundation",
            "ROADMAP.md#stage-2-query-engine",
        ]);
        let err = resolve(&g, "stage-").unwrap_err();
        assert!(err.to_string().contains("ambiguous slug prefix"));
        assert!(err.to_string().contains("stage-1"));
        assert!(err.to_string().contains("stage-2"));
    }
}
