pub mod directory;
pub mod markdown;

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;

use crate::graph::Graph;

pub trait Scanner {
    fn scan(&self, root: &Path) -> Result<Graph>;
}

pub fn run_all(root: &Path, layer: Option<&str>) -> Result<Graph> {
    let scanners: Vec<Box<dyn Scanner>> = vec![
        Box::new(directory::DirectoryScanner),
        Box::new(markdown::MarkdownScanner),
    ];

    let mut graph = Graph::default();
    for scanner in &scanners {
        graph.add(scanner.scan(root)?);
    }
    graph.sort();

    if let Some(filter) = layer {
        graph.nodes.retain(|n| n.source == filter);
        let valid_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        graph.edges.retain(|e| valid_ids.contains(e.source.as_str()) && valid_ids.contains(e.target.as_str()));
    }

    Ok(graph)
}
