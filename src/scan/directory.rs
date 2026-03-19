use anyhow::Result;
use std::path::Path;

use crate::graph::{Edge, EdgeKind, Graph, Node, NodeKind};
use super::Scanner;

pub struct DirectoryScanner;

impl Scanner for DirectoryScanner {
    fn scan(&self, root: &Path) -> Result<Graph> {
        let root = root.canonicalize()?;
        let mut graph = Graph::default();

        for entry in ignore::WalkBuilder::new(&root)
            .hidden(false)
            .filter_entry(|e| {
                e.file_name() != ".git"
            })
            .build()
        {
            let entry = entry?;
            let abs = entry.path();
            let rel = abs.strip_prefix(&root)?;

            let id = if rel.as_os_str().is_empty() {
                ".".to_string()
            } else {
                rel.to_string_lossy().replace('\\', "/")
            };

            let kind = if abs.is_dir() {
                NodeKind::Directory
            } else {
                NodeKind::File
            };

            let label = if id == "." {
                root.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ".".into())
            } else {
                rel.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            };

            graph.nodes.push(Node {
                id: id.clone(),
                kind,
                source: "filesystem".into(),
                label,
                metadata: None,
            });

            // Contains edge from parent → child
            if id != "." {
                let parent_id = match rel.parent() {
                    Some(p) if p.as_os_str().is_empty() => ".".to_string(),
                    Some(p) => p.to_string_lossy().replace('\\', "/"),
                    None => ".".to_string(),
                };
                graph.edges.push(Edge {
                    source: parent_id,
                    target: id,
                    kind: EdgeKind::Contains,
                });
            }
        }

        Ok(graph)
    }
}
