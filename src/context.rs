use anyhow::Result;
use serde::Serialize;
use std::path::Path;

use crate::graph::{EdgeKind, Graph, Node, NodeKind};
use crate::scan::markdown::slugify;

#[derive(Serialize)]
struct ContextJson {
    label: String,
    #[serde(rename = "type")]
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
    source: String,
    ancestors: Vec<AncestorJson>,
    children: Vec<ChildJson>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    links: Vec<LinkJson>,
}

#[derive(Serialize, Clone)]
struct LinkJson {
    path: String,
    lines: usize,
    tokens: usize,
}

#[derive(Serialize)]
struct AncestorJson {
    label: String,
}

#[derive(Serialize)]
struct ChildJson {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<String>,
}

pub fn run(id: &str, graph: &Graph, root: &Path, json: bool) -> Result<()> {
    let node = graph.nodes.iter().find(|n| n.id == id)
        .expect("resolved ID must exist in graph");

    let anc = ancestors(id, graph);
    let kids = children(id, graph);
    let link_paths = find_link_paths(id, graph, root);
    let links: Vec<LinkJson> = link_paths.iter().filter_map(|p| {
        let content = std::fs::read_to_string(root.join(p)).ok()?;
        Some(LinkJson {
            path: p.clone(),
            lines: content.lines().count(),
            tokens: estimate_tokens(&content),
        })
    }).collect();

    if json {
        let ctx = ContextJson {
            label: node.label.clone(),
            kind: format!("{:?}", node.kind).to_lowercase(),
            status: node_status(node).map(String::from),
            source: node.source.clone(),
            ancestors: anc.iter().map(|n| AncestorJson {
                label: n.label.clone(),
            }).collect(),
            children: kids.iter().map(|n| ChildJson {
                label: n.label.clone(),
                status: node_status(n).map(String::from),
            }).collect(),
            links: links.clone(),
        };
        println!("{}", serde_json::to_string_pretty(&ctx)?);
        return Ok(());
    }

    // Structured text output
    let status_str = node_status(node).unwrap_or("—");
    let kind_str = format!("{:?}", node.kind).to_lowercase();
    println!("# {}", node.label);
    println!("{} | {}", kind_str, status_str);

    // Compact ancestor breadcrumb
    if !anc.is_empty() {
        let breadcrumb: Vec<&str> = anc.iter().map(|n| n.label.as_str()).collect();
        println!("{}", breadcrumb.join(" > "));
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
                println!("- [{mark}] {}", c.label);
            } else {
                println!("  {}", c.label);
            }
        }
    }

    if !links.is_empty() {
        println!("\n## Links");
        for l in &links {
            println!("  {} ({} lines, ~{} tokens)", l.path, l.lines, l.tokens);
        }
    }

    Ok(())
}

fn node_status(node: &Node) -> Option<&str> {
    node.metadata
        .as_ref()
        .and_then(|m| m.get("status"))
        .and_then(|v| v.as_str())
}

/// Rough token estimate: ~1 token per 4 bytes for ASCII, CJK chars count individually.
fn estimate_tokens(text: &str) -> usize {
    let mut count = 0usize;
    for word in text.split_whitespace() {
        let cjk: usize = word.chars().filter(|c| is_cjk(*c)).count();
        if cjk > 0 {
            // CJK chars ≈ 1 token each, remaining ASCII ≈ 1 token per word
            count += cjk + if word.chars().count() > cjk { 1 } else { 0 };
        } else {
            // English: ~1.3 tokens per word on average
            count += 1;
        }
    }
    count
}

fn is_cjk(c: char) -> bool {
    matches!(c,
        '\u{4E00}'..='\u{9FFF}' |   // CJK Unified
        '\u{3400}'..='\u{4DBF}' |   // CJK Extension A
        '\u{3000}'..='\u{303F}' |   // CJK Symbols
        '\u{FF00}'..='\u{FFEF}'     // Fullwidth
    )
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

/// Find linked .md file paths: reference edges to .md files, plus convention fallback.
fn find_link_paths(id: &str, graph: &Graph, root: &Path) -> Vec<String> {
    let mut paths = Vec::new();

    // 1. Outgoing Reference edges pointing to .md files
    for edge in graph.edges.iter() {
        if edge.kind == EdgeKind::References && edge.source == id {
            let target = &edge.target;
            let file_path = target.split_once('#').map(|(f, _)| f).unwrap_or(target);
            if file_path.ends_with(".md") && root.join(file_path).exists() {
                let s = file_path.to_string();
                if !paths.contains(&s) {
                    paths.push(s);
                }
            }
        }
    }

    // 2. Convention fallback: roadmap/<slug>.md (only if no explicit links found)
    if paths.is_empty() {
        if let Some((_, slug)) = id.rsplit_once('#') {
            let candidate = format!("roadmap/{slug}.md");
            if root.join(&candidate).exists() {
                paths.push(candidate);
            } else {
                let slugified = slugify(slug);
                if slugified != slug {
                    let candidate = format!("roadmap/{slugified}.md");
                    if root.join(&candidate).exists() {
                        paths.push(candidate);
                    }
                }
            }
        }
    }

    paths
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
                make_node("root", NodeKind::Section, "root"),
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
            nodes: vec![make_node("root", NodeKind::Section, "root")],
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
    fn find_link_paths_returns_empty_when_no_match() {
        let graph = Graph { nodes: vec![], edges: vec![] };
        let tmp = std::env::temp_dir().join("topo_ctx_test_none3");
        let _ = std::fs::create_dir_all(&tmp);
        assert!(find_link_paths("foo#bar", &graph, &tmp).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_link_paths_convention_fallback() {
        let tmp = std::env::temp_dir().join("topo_ctx_test_conv3");
        let roadmap_dir = tmp.join("roadmap");
        let _ = std::fs::create_dir_all(&roadmap_dir);
        std::fs::write(roadmap_dir.join("scan.md"), "# Scan detail").unwrap();

        let graph = Graph { nodes: vec![], edges: vec![] };
        let paths = find_link_paths("ROADMAP.md#scan", &graph, &tmp);
        assert_eq!(paths, vec!["roadmap/scan.md"]);
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn find_link_paths_from_reference_edges() {
        let tmp = std::env::temp_dir().join("topo_ctx_test_refs");
        let _ = std::fs::create_dir_all(tmp.join(".claude/skills"));
        std::fs::write(tmp.join(".claude/skills/CONV.md"), "# Conv").unwrap();
        std::fs::write(tmp.join("notes.md"), "# Notes").unwrap();

        let graph = Graph {
            nodes: vec![make_node("R.md#task", NodeKind::Task, "Task")],
            edges: vec![
                Edge { source: "R.md#task".into(), target: ".claude/skills/CONV.md#conv".into(), kind: EdgeKind::References },
                Edge { source: "R.md#task".into(), target: "notes.md".into(), kind: EdgeKind::References },
            ],
        };
        let paths = find_link_paths("R.md#task", &graph, &tmp);
        assert_eq!(paths, vec![".claude/skills/CONV.md".to_string(), "notes.md".to_string()]);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
