use crate::graph::{EdgeKind, Graph, Node, NodeKind};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Compact,
    Ids,
    #[default]
    Tree,
}

pub fn print_json(graph: &Graph) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(graph)?);
    Ok(())
}

pub fn print_graph(graph: &Graph, format: &OutputFormat) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => print_json(graph),
        OutputFormat::Compact => {
            for node in &graph.nodes {
                println!("{}\t{}", node.id, node.label);
            }
            Ok(())
        }
        OutputFormat::Ids => {
            for node in &graph.nodes {
                println!("{}", node.id);
            }
            Ok(())
        }
        OutputFormat::Tree => print_tree(graph),
    }
}

pub fn print_count(graph: &Graph) {
    println!("{}", graph.nodes.len());
}

fn print_tree(graph: &Graph) -> anyhow::Result<()> {
    // Build children map from Contains edges only
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut has_parent = HashSet::new();
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Contains {
            children.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
            has_parent.insert(edge.target.as_str());
        }
    }

    let nodes: HashMap<&str, &Node> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Roots = nodes with no incoming Contains edge (within the result set)
    let roots: Vec<&str> = graph.nodes.iter()
        .map(|n| n.id.as_str())
        .filter(|id| !has_parent.contains(id))
        .collect();

    fn walk(id: &str, depth: usize, children: &HashMap<&str, Vec<&str>>, nodes: &HashMap<&str, &Node>) {
        let Some(node) = nodes.get(id) else { return };
        let indent = "  ".repeat(depth);
        let stable_id = node.metadata.as_ref()
            .and_then(|m| m.get("stable_id"))
            .and_then(|v| v.as_str());
        let prefix = stable_id.map(|sid| format!("{sid} ")).unwrap_or_default();
        let marker = if node.kind == NodeKind::Task {
            let status = node.metadata.as_ref()
                .and_then(|m| m.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("todo");
            match status {
                "done" => "[x] ",
                "in-progress" => "[-] ",
                "dropped" => "[~] ",
                _ => "[ ] ",
            }
        } else {
            ""
        };
        println!("{indent}{marker}{prefix}{label}", label = node.label);
        if let Some(kids) = children.get(id) {
            for kid in kids {
                if nodes.contains_key(kid) {
                    walk(kid, depth + 1, children, nodes);
                }
            }
        }
    }

    for root in &roots {
        walk(root, 0, &children, &nodes);
    }
    Ok(())
}
