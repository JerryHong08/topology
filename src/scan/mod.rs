pub mod directory;
pub mod markdown;

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::time::SystemTime;

use crate::graph::{Edge, EdgeKind, Graph};

pub trait Scanner {
    fn scan(&self, root: &Path) -> Result<Graph>;
}

const CACHE_FILE: &str = ".topology.json";

pub fn run_all(root: &Path, layer: Option<&str>) -> Result<Graph> {
    let mut graph = build_graph(root)?;

    if let Some(filter) = layer {
        graph.nodes.retain(|n| n.source == filter);
        let valid_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
        graph.edges.retain(|e| valid_ids.contains(e.source.as_str()) && valid_ids.contains(e.target.as_str()));
    }

    Ok(graph)
}

fn build_graph(root: &Path) -> Result<Graph> {
    let mut graph = Graph::default();
    let mut links = Vec::new();
    graph.add(directory::DirectoryScanner.scan(root)?);
    graph.add(markdown::MarkdownScanner.scan_with_links(root, &mut links)?);
    resolve_references(&mut graph, &links);
    Ok(graph)
}

pub fn run_cached(root: &Path, layer: Option<&str>) -> Result<Graph> {
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cache_path = if canon.is_file() {
        canon.parent().unwrap_or(&canon).join(CACHE_FILE)
    } else {
        canon.join(CACHE_FILE)
    };

    if let Some(graph) = read_cache_if_fresh(&cache_path, &canon) {
        let mut graph = graph;
        if let Some(filter) = layer {
            graph.nodes.retain(|n| n.source == filter);
            let valid_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
            graph.edges.retain(|e| valid_ids.contains(e.source.as_str()) && valid_ids.contains(e.target.as_str()));
        }
        return Ok(graph);
    }

    let graph = build_graph(root)?;
    let _ = write_cache(&cache_path, &graph);
    run_all(root, layer)
}

pub fn write_cache_for(root: &Path, graph: &Graph) {
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cache_path = if canon.is_file() {
        canon.parent().unwrap_or(&canon).join(CACHE_FILE)
    } else {
        canon.join(CACHE_FILE)
    };
    let _ = write_cache(&cache_path, graph);
}

fn write_cache(cache_path: &Path, graph: &Graph) -> Result<()> {
    let json = serde_json::to_string_pretty(graph)?;
    std::fs::write(cache_path, json)?;
    Ok(())
}

pub fn read_cache(root: &Path) -> Option<Graph> {
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cache_path = if canon.is_file() {
        canon.parent().unwrap_or(&canon).join(CACHE_FILE)
    } else {
        canon.join(CACHE_FILE)
    };
    let data = std::fs::read_to_string(&cache_path).ok()?;
    serde_json::from_str(&data).ok()
}

fn read_cache_if_fresh(cache_path: &Path, root: &Path) -> Option<Graph> {
    let cache_meta = std::fs::metadata(cache_path).ok()?;
    let cache_mtime = cache_meta.modified().ok()?;
    let newest = newest_source_mtime(root)?;
    if cache_mtime >= newest {
        let data = std::fs::read_to_string(cache_path).ok()?;
        serde_json::from_str(&data).ok()
    } else {
        None
    }
}

fn newest_source_mtime(root: &Path) -> Option<SystemTime> {
    let mut newest = SystemTime::UNIX_EPOCH;
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            name != ".git" && name != CACHE_FILE
        })
        .build();
    for entry in walker.flatten() {
        if let Ok(meta) = entry.metadata() {
            if let Ok(mtime) = meta.modified() {
                if mtime > newest {
                    newest = mtime;
                }
            }
        }
    }
    if newest == SystemTime::UNIX_EPOCH { None } else { Some(newest) }
}

fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "." | "" => {}
            ".." => { parts.pop(); }
            _ => parts.push(part),
        }
    }
    parts.join("/")
}

fn resolve_references(graph: &mut Graph, links: &[markdown::RawLink]) {
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    for link in links {
        let url = link.target_url.as_str();
        if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("mailto:") {
            continue;
        }

        let (path_part, anchor_part) = match url.split_once('#') {
            Some((p, a)) => (p, Some(a)),
            None => (url, None),
        };

        let resolved = if path_part.is_empty() {
            // #anchor only → same file
            match anchor_part {
                Some(anchor) => format!("{}#{}", link.source_file, anchor),
                None => continue,
            }
        } else {
            let source_dir = Path::new(&link.source_file)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();

            let full_path = if source_dir.is_empty() {
                path_part.to_string()
            } else {
                format!("{}/{}", source_dir, path_part)
            };

            let normalized = normalize_path(&full_path);

            match anchor_part {
                Some(anchor) => format!("{}#{}", normalized, anchor),
                None => normalized,
            }
        };

        if node_ids.contains(resolved.as_str()) {
            graph.edges.push(Edge {
                source: link.source_node.clone(),
                target: resolved,
                kind: EdgeKind::References,
            });
        }
    }
}
