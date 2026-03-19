use crate::graph::{EdgeKind, Graph};
use crate::resolve::short_hash;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Json,
    Compact,
    Ids,
    Tree,
}

pub fn print_json(graph: &Graph, hash: bool) -> anyhow::Result<()> {
    let mut val = serde_json::to_value(graph)?;
    if hash {
        if let Some(nodes) = val.get_mut("nodes").and_then(|v| v.as_array_mut()) {
            for node in nodes {
                if let Some(id) = node.get("id").and_then(|v| v.as_str()) {
                    let short = short_hash(id);
                    node.as_object_mut()
                        .unwrap()
                        .insert("short".into(), serde_json::Value::String(short));
                }
            }
        }
    }
    println!("{}", serde_json::to_string_pretty(&val)?);
    Ok(())
}

pub fn print_graph(graph: &Graph, format: &OutputFormat, hash: bool) -> anyhow::Result<()> {
    match format {
        OutputFormat::Json => print_json(graph, hash),
        OutputFormat::Compact => {
            for node in &graph.nodes {
                if hash {
                    println!("[{}] {}\t{}", short_hash(&node.id), node.id, node.label);
                } else {
                    println!("{}\t{}", node.id, node.label);
                }
            }
            Ok(())
        }
        OutputFormat::Ids => {
            for node in &graph.nodes {
                if hash {
                    println!("[{}] {}", short_hash(&node.id), node.id);
                } else {
                    println!("{}", node.id);
                }
            }
            Ok(())
        }
        OutputFormat::Tree => print_tree(graph, hash),
    }
}

pub fn print_count(graph: &Graph) {
    println!("{}", graph.nodes.len());
}

fn print_tree(graph: &Graph, hash: bool) -> anyhow::Result<()> {
    // Build children map from Contains edges only
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut has_parent = HashSet::new();
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Contains {
            children.entry(edge.source.as_str()).or_default().push(edge.target.as_str());
            has_parent.insert(edge.target.as_str());
        }
    }

    // Label lookup
    let labels: HashMap<&str, &str> = graph.nodes.iter().map(|n| (n.id.as_str(), n.label.as_str())).collect();
    let node_set: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    // Roots = nodes with no incoming Contains edge (within the result set)
    let roots: Vec<&str> = graph.nodes.iter()
        .map(|n| n.id.as_str())
        .filter(|id| !has_parent.contains(id))
        .collect();

    fn walk(id: &str, depth: usize, hash: bool, children: &HashMap<&str, Vec<&str>>, labels: &HashMap<&str, &str>, node_set: &HashSet<&str>) {
        let indent = "  ".repeat(depth);
        let label = labels.get(id).unwrap_or(&id);
        if hash {
            println!("{indent}[{}] {label}", short_hash(id));
        } else {
            println!("{indent}{label}");
        }
        if let Some(kids) = children.get(id) {
            for kid in kids {
                if node_set.contains(kid) {
                    walk(kid, depth + 1, hash, children, labels, node_set);
                }
            }
        }
    }

    for root in &roots {
        walk(root, 0, hash, &children, &labels, &node_set);
    }
    Ok(())
}
