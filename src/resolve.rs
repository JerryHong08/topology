use anyhow::{bail, Result};

use crate::graph::Graph;

/// Resolve a user-provided input to a canonical node ID.
///
/// Priority: exact → numeric stable_id → ID prefix → slug exact → slug prefix.
pub fn resolve(graph: &Graph, input: &str) -> Result<String> {
    // 1. Exact match
    if graph.nodes.iter().any(|n| n.id == input) {
        return Ok(input.to_string());
    }

    // 2. Numeric stable_id match (e.g. "1.3" matches node with stable_id: "1.3")
    let numeric_matches: Vec<&str> = graph
        .nodes
        .iter()
        .filter(|n| {
            n.metadata
                .as_ref()
                .and_then(|m| m.get("stable_id"))
                .and_then(|v| v.as_str())
                == Some(input)
        })
        .map(|n| n.id.as_str())
        .collect();
    match numeric_matches.len() {
        1 => return Ok(numeric_matches[0].to_string()),
        n if n > 1 => bail!(
            "ambiguous numeric ID '{}', matches {} nodes:\n  {}",
            input,
            n,
            numeric_matches.join("\n  ")
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
    fn resolve_exact_match() {
        let g = make_graph(&["foo#bar", "foo#baz"]);
        assert_eq!(resolve(&g, "foo#bar").unwrap(), "foo#bar");
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

    fn make_graph_with_stable_ids(entries: &[(&str, Option<&str>)]) -> Graph {
        let nodes = entries
            .iter()
            .map(|(id, stable_id)| Node {
                id: id.to_string(),
                kind: NodeKind::Task,
                source: "test".into(),
                label: id.to_string(),
                metadata: stable_id.map(|sid| serde_json::json!({"stable_id": sid})),
            })
            .collect();
        Graph {
            nodes,
            edges: vec![],
        }
    }

    #[test]
    fn resolve_numeric_id() {
        let g = make_graph_with_stable_ids(&[
            ("ROADMAP.md#scan-project-files", Some("1.1")),
            ("ROADMAP.md#stable-id", Some("1.3")),
        ]);
        assert_eq!(
            resolve(&g, "1.1").unwrap(),
            "ROADMAP.md#scan-project-files"
        );
        assert_eq!(resolve(&g, "1.3").unwrap(), "ROADMAP.md#stable-id");
    }

    #[test]
    fn resolve_numeric_id_nested() {
        let g = make_graph_with_stable_ids(&[
            ("ROADMAP.md#parent", Some("1.1")),
            ("ROADMAP.md#child", Some("1.1.1")),
        ]);
        // "1.1" should match exactly, not prefix-match to "1.1.1"
        assert_eq!(resolve(&g, "1.1").unwrap(), "ROADMAP.md#parent");
        assert_eq!(resolve(&g, "1.1.1").unwrap(), "ROADMAP.md#child");
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
