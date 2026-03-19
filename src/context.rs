use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::graph::{EdgeKind, Graph, Node, NodeKind};
use crate::resolve::short_hash;
use crate::scan::markdown::slugify;

#[derive(Serialize)]
struct ContextJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    label: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    source: String,
    ancestors: Vec<AncestorJson>,
    children: Vec<ChildJson>,
    references: Vec<RefJson>,
    referenced_by: Vec<RefJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Serialize)]
struct AncestorJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    label: String,
}

#[derive(Serialize)]
struct ChildJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

#[derive(Serialize)]
struct RefJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    short: Option<String>,
    label: String,
}

fn hash_if(id: &str, bare: bool) -> Option<String> {
    if bare { None } else { Some(short_hash(id)) }
}

pub fn run(id: &str, graph: &Graph, root: &Path, json: bool, bare: bool) -> Result<()> {
    let node = graph.nodes.iter().find(|n| n.id == id)
        .expect("resolved ID must exist in graph");

    let anc = ancestors(id, graph);
    let kids = children(id, graph);
    let refs = references(id, graph);
    let ref_by = referenced_by(id, graph);
    let detail_path = find_detail_path(id, graph, root);

    if json {
        let ctx = ContextJson {
            short: hash_if(id, bare),
            label: node.label.clone(),
            kind: format!("{:?}", node.kind).to_lowercase(),
            status: node_status(node).map(String::from),
            source: node.source.clone(),
            ancestors: anc.iter().map(|n| AncestorJson {
                short: hash_if(&n.id, bare),
                label: n.label.clone(),
            }).collect(),
            children: kids.iter().map(|n| ChildJson {
                short: hash_if(&n.id, bare),
                label: n.label.clone(),
                status: node_status(n).map(String::from),
            }).collect(),
            references: refs.iter().map(|n| RefJson {
                short: hash_if(&n.id, bare),
                label: n.label.clone(),
            }).collect(),
            referenced_by: ref_by.iter().map(|n| RefJson {
                short: hash_if(&n.id, bare),
                label: n.label.clone(),
            }).collect(),
            detail: detail_path.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&ctx)?);
        return Ok(());
    }

    // Structured text output
    let status_str = node_status(node).unwrap_or("—");
    let kind_str = format!("{:?}", node.kind).to_lowercase();
    if bare {
        println!("# {} — {}", node.label, node.source);
        println!("{} | {}", kind_str, status_str);
    } else {
        println!("# {} — {}", node.label, node.source);
        println!("[{}] {} | {}", short_hash(id), kind_str, status_str);
    }

    if !anc.is_empty() {
        println!("\n## Ancestors");
        for a in &anc {
            if bare {
                println!("  {}", a.label);
            } else {
                println!("  [{}] {}", short_hash(&a.id), a.label);
            }
        }
    }

    if !kids.is_empty() {
        let has_tasks = kids.iter().any(|n| n.kind == NodeKind::Task);
        if has_tasks {
            println!("\n## Subtasks");
        } else {
            println!("\n## Children");
        }
        for c in &kids {
            if c.kind == NodeKind::Task {
                let mark = if node_status(c) == Some("done") { "x" } else { " " };
                if bare {
                    println!("- [{mark}] {}", c.label);
                } else {
                    println!("- [{mark}] [{}] {}", short_hash(&c.id), c.label);
                }
            } else if bare {
                println!("  {}", c.label);
            } else {
                println!("  [{}] {}", short_hash(&c.id), c.label);
            }
        }
    }

    if !refs.is_empty() {
        println!("\n## References");
        for r in &refs {
            if bare {
                println!("  {}", r.label);
            } else {
                println!("  [{}] {}", short_hash(&r.id), r.label);
            }
        }
    }

    if !ref_by.is_empty() {
        println!("\n## Referenced by");
        for r in &ref_by {
            if bare {
                println!("  {}", r.label);
            } else {
                println!("  [{}] {}", short_hash(&r.id), r.label);
            }
        }
    }

    if let Some(path) = &detail_path {
        println!("\n## Detail");
        println!("  {path}");
    }

    Ok(())
}

fn node_status(node: &Node) -> Option<&str> {
    node.metadata
        .as_ref()
        .and_then(|m| m.get("status"))
        .and_then(|v| v.as_str())
}

/// Walk Contains edges upward to build ancestor chain (root first).
fn ancestors<'a>(id: &str, graph: &'a Graph) -> Vec<&'a Node> {
    let mut chain = Vec::new();
    let mut current = id;
    loop {
        let parent = graph.edges.iter().find(|e| {
            e.kind == EdgeKind::Contains && e.target == current
        });
        match parent {
            Some(edge) => {
                if let Some(node) = graph.nodes.iter().find(|n| n.id == edge.source) {
                    chain.push(node);
                    current = &node.id;
                } else {
                    break;
                }
            }
            None => break,
        }
    }
    chain.reverse();
    chain
}

/// Get direct children via Contains edges.
fn children<'a>(id: &str, graph: &'a Graph) -> Vec<&'a Node> {
    graph.edges.iter()
        .filter(|e| e.kind == EdgeKind::Contains && e.source == id)
        .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.target))
        .collect()
}

/// Get outgoing Reference edges.
fn references<'a>(id: &str, graph: &'a Graph) -> Vec<&'a Node> {
    graph.edges.iter()
        .filter(|e| e.kind == EdgeKind::References && e.source == id)
        .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.target))
        .collect()
}

/// Get incoming Reference edges.
fn referenced_by<'a>(id: &str, graph: &'a Graph) -> Vec<&'a Node> {
    graph.edges.iter()
        .filter(|e| e.kind == EdgeKind::References && e.target == id)
        .filter_map(|e| graph.nodes.iter().find(|n| n.id == e.source))
        .collect()
}

/// Find detail doc path: first via Reference edges to roadmap/*.md, then convention fallback.
/// Returns the relative path string, not the content.
fn find_detail_path(id: &str, graph: &Graph, root: &Path) -> Option<String> {
    // 1. Check outgoing Reference edges for roadmap/*.md targets
    for edge in graph.edges.iter() {
        if edge.kind == EdgeKind::References && edge.source == id {
            let target = &edge.target;
            if target.starts_with("roadmap/") && target.ends_with(".md") {
                if root.join(target.as_str()).exists() {
                    return Some(target.clone());
                }
            }
        }
    }

    // 2. Convention fallback: extract slug from ID after #
    if let Some((_, slug)) = id.rsplit_once('#') {
        let candidate = format!("roadmap/{slug}.md");
        if root.join(&candidate).exists() {
            return Some(candidate);
        }
        let slugified = slugify(slug);
        if slugified != slug {
            let candidate = format!("roadmap/{slugified}.md");
            if root.join(&candidate).exists() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Edge, Node};

    fn make_node(id: &str, kind: NodeKind, label: &str) -> Node {
        Node {
            id: id.into(),
            kind,
            source: "markdown".into(),
            label: label.into(),
            metadata: None,
        }
    }

    fn make_task(id: &str, label: &str, status: &str) -> Node {
        Node {
            id: id.into(),
            kind: NodeKind::Task,
            source: "markdown".into(),
            label: label.into(),
            metadata: Some(serde_json::json!({"status": status})),
        }
    }

    #[test]
    fn ancestors_returns_chain_root_first() {
        let graph = Graph {
            nodes: vec![
                make_node("root", NodeKind::File, "root"),
                make_node("root#a", NodeKind::Section, "A"),
                make_node("root#a#b", NodeKind::Section, "B"),
            ],
            edges: vec![
                Edge { source: "root".into(), target: "root#a".into(), kind: EdgeKind::Contains },
                Edge { source: "root#a".into(), target: "root#a#b".into(), kind: EdgeKind::Contains },
            ],
        };
        let chain = ancestors("root#a#b", &graph);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].id, "root");
        assert_eq!(chain[1].id, "root#a");
    }

    #[test]
    fn ancestors_empty_for_root() {
        let graph = Graph {
            nodes: vec![make_node("root", NodeKind::File, "root")],
            edges: vec![],
        };
        assert!(ancestors("root", &graph).is_empty());
    }

    #[test]
    fn children_returns_direct_children() {
        let graph = Graph {
            nodes: vec![
                make_task("p", "Parent", "todo"),
                make_task("c1", "Child 1", "done"),
                make_task("c2", "Child 2", "todo"),
            ],
            edges: vec![
                Edge { source: "p".into(), target: "c1".into(), kind: EdgeKind::Contains },
                Edge { source: "p".into(), target: "c2".into(), kind: EdgeKind::Contains },
            ],
        };
        let kids = children("p", &graph);
        assert_eq!(kids.len(), 2);
    }

    #[test]
    fn references_and_referenced_by() {
        let graph = Graph {
            nodes: vec![
                make_node("a", NodeKind::Task, "A"),
                make_node("b", NodeKind::File, "B"),
            ],
            edges: vec![
                Edge { source: "a".into(), target: "b".into(), kind: EdgeKind::References },
            ],
        };
        assert_eq!(references("a", &graph).len(), 1);
        assert_eq!(references("a", &graph)[0].id, "b");
        assert_eq!(referenced_by("b", &graph).len(), 1);
        assert_eq!(referenced_by("b", &graph)[0].id, "a");
    }

    #[test]
    fn find_detail_path_returns_none_when_no_match() {
        let graph = Graph { nodes: vec![], edges: vec![] };
        let tmp = std::env::temp_dir().join("topo_ctx_test_none2");
        let _ = std::fs::create_dir_all(&tmp);
        assert!(find_detail_path("foo#bar", &graph, &tmp).is_none());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_detail_path_convention_fallback() {
        let tmp = std::env::temp_dir().join("topo_ctx_test_conv2");
        let roadmap_dir = tmp.join("roadmap");
        let _ = std::fs::create_dir_all(&roadmap_dir);
        std::fs::write(roadmap_dir.join("scan.md"), "# Scan detail").unwrap();

        let graph = Graph { nodes: vec![], edges: vec![] };
        let path = find_detail_path("ROADMAP.md#scan", &graph, &tmp);
        assert_eq!(path.unwrap(), "roadmap/scan.md");
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
