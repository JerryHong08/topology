use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::graph::{Edge, EdgeKind, Graph, Node};
use crate::scan;

#[derive(Serialize)]
pub struct DiffOutput {
    pub nodes: NodeDiff,
    pub edges: EdgeDiff,
}

#[derive(Serialize)]
pub struct NodeDiff {
    pub added: Vec<Node>,
    pub removed: Vec<Node>,
    pub changed: Vec<NodeChange>,
}

#[derive(Serialize)]
pub struct NodeChange {
    pub id: String,
    pub before: Node,
    pub after: Node,
}

#[derive(Serialize)]
pub struct EdgeDiff {
    pub added: Vec<Edge>,
    pub removed: Vec<Edge>,
}

type EdgeKey = (String, String, String);

fn edge_key(e: &Edge) -> EdgeKey {
    let kind = match e.kind {
        EdgeKind::Contains => "contains",
        EdgeKind::References => "references",
        EdgeKind::Sequence => "sequence",
    };
    (e.source.clone(), e.target.clone(), kind.to_string())
}

pub fn run(root: &Path) -> Result<()> {
    let before = scan::read_cache(root).unwrap_or_default();
    let after = scan::run_all(root, None)?;

    let output = compute(&before, &after);

    scan::write_cache_for(root, &after);
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

pub fn compute(before: &Graph, after: &Graph) -> DiffOutput {
    let before_map: HashMap<&str, &Node> = before.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let after_map: HashMap<&str, &Node> = after.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();

    for (id, node) in &after_map {
        match before_map.get(id) {
            None => added.push((*node).clone()),
            Some(old) if *old != *node => changed.push(NodeChange {
                id: id.to_string(),
                before: (*old).clone(),
                after: (*node).clone(),
            }),
            _ => {}
        }
    }
    for (id, node) in &before_map {
        if !after_map.contains_key(id) {
            removed.push((*node).clone());
        }
    }

    let before_edges: HashSet<EdgeKey> = before.edges.iter().map(|e| edge_key(e)).collect();
    let after_edges: HashSet<EdgeKey> = after.edges.iter().map(|e| edge_key(e)).collect();
    let before_edge_map: HashMap<EdgeKey, &Edge> = before.edges.iter().map(|e| (edge_key(e), e)).collect();
    let after_edge_map: HashMap<EdgeKey, &Edge> = after.edges.iter().map(|e| (edge_key(e), e)).collect();

    let added_edges: Vec<Edge> = after_edges.difference(&before_edges)
        .filter_map(|k| after_edge_map.get(k).map(|e| (*e).clone()))
        .collect();
    let removed_edges: Vec<Edge> = before_edges.difference(&after_edges)
        .filter_map(|k| before_edge_map.get(k).map(|e| (*e).clone()))
        .collect();

    DiffOutput {
        nodes: NodeDiff { added, removed, changed },
        edges: EdgeDiff { added: added_edges, removed: removed_edges },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{EdgeKind, Node, NodeKind};

    fn node(id: &str, label: &str) -> Node {
        Node { id: id.into(), kind: NodeKind::Task, source: "markdown".into(), label: label.into(), metadata: Some(serde_json::json!({"status": "todo"})) }
    }

    fn edge(src: &str, tgt: &str) -> Edge {
        Edge { source: src.into(), target: tgt.into(), kind: EdgeKind::Contains }
    }

    #[test]
    fn identical_graphs_empty_diff() {
        let g = Graph { nodes: vec![node("a", "A")], edges: vec![edge("r", "a")] };
        let d = compute(&g, &g);
        assert!(d.nodes.added.is_empty());
        assert!(d.nodes.removed.is_empty());
        assert!(d.nodes.changed.is_empty());
        assert!(d.edges.added.is_empty());
        assert!(d.edges.removed.is_empty());
    }

    #[test]
    fn added_node() {
        let before = Graph::default();
        let after = Graph { nodes: vec![node("a", "A")], edges: vec![] };
        let d = compute(&before, &after);
        assert_eq!(d.nodes.added.len(), 1);
        assert_eq!(d.nodes.added[0].id, "a");
    }

    #[test]
    fn removed_node() {
        let before = Graph { nodes: vec![node("a", "A")], edges: vec![] };
        let after = Graph::default();
        let d = compute(&before, &after);
        assert_eq!(d.nodes.removed.len(), 1);
        assert_eq!(d.nodes.removed[0].id, "a");
    }

    #[test]
    fn changed_node() {
        let n1 = node("a", "A");
        let mut n2 = node("a", "A");
        n2.metadata = Some(serde_json::json!({"status": "done"}));
        let before = Graph { nodes: vec![n1], edges: vec![] };
        let after = Graph { nodes: vec![n2], edges: vec![] };
        let d = compute(&before, &after);
        assert_eq!(d.nodes.changed.len(), 1);
        assert_eq!(d.nodes.changed[0].id, "a");
    }

    #[test]
    fn added_removed_edges() {
        let e1 = edge("a", "b");
        let e2 = edge("a", "c");
        let before = Graph { nodes: vec![], edges: vec![e1.clone()] };
        let after = Graph { nodes: vec![], edges: vec![e2.clone()] };
        let d = compute(&before, &after);
        assert_eq!(d.edges.added.len(), 1);
        assert_eq!(d.edges.added[0].target, "c");
        assert_eq!(d.edges.removed.len(), 1);
        assert_eq!(d.edges.removed[0].target, "b");
    }
}
