pub mod markdown;

use anyhow::Result;
use std::collections::HashSet;
use std::path::Path;
use std::time::SystemTime;

use crate::graph::{Edge, EdgeKind, Graph};

const CACHE_FILE: &str = ".topology.json";

pub fn run_all(root: &Path) -> Result<Graph> {
    build_graph(root)
}

fn build_graph(root: &Path) -> Result<Graph> {
    let mut graph = Graph::default();
    let mut links = Vec::new();
    graph.add(markdown::MarkdownScanner.scan_with_links(root, &mut links)?);
    resolve_references(&mut graph, &links);
    Ok(graph)
}

pub fn run_cached(root: &Path) -> Result<Graph> {
    let canon = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let cache_path = if canon.is_file() {
        canon.parent().unwrap_or(&canon).join(CACHE_FILE)
    } else {
        canon.join(CACHE_FILE)
    };

    if let Some(graph) = read_cache_if_fresh(&cache_path, &canon) {
        return Ok(graph);
    }

    let graph = build_graph(root)?;
    let _ = write_cache(&cache_path, &graph);
    run_all(root)
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

/// Resolve a relative path through the filesystem to get the canonical relative path.
/// This handles symlinks: `.claude/skills/foo.md` → `.agents/skills/foo.md`.
fn resolve_real_path(rel_path: &str, root: &Path) -> Option<String> {
    let abs = root.join(rel_path).canonicalize().ok()?;
    let canon_root = root.canonicalize().ok()?;
    let stripped = abs.strip_prefix(&canon_root).ok()?;
    Some(stripped.to_string_lossy().replace('\\', "/"))
}

fn resolve_references(graph: &mut Graph, links: &[markdown::RawLink]) {
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();
    // Derive root from first node's source file (best effort)
    let root = std::env::current_dir().unwrap_or_default();

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

            let mut normalized = normalize_path(&full_path);

            // Resolve symlinks to match scanner's canonical paths
            if let Some(real) = resolve_real_path(&normalized, &root) {
                normalized = real;
            }

            match anchor_part {
                Some(anchor) => format!("{}#{}", normalized, anchor),
                None => normalized,
            }
        };

        let target = if node_ids.contains(resolved.as_str()) {
            Some(resolved)
        } else {
            // File-level link (e.g. "roadmap/scan.md") — resolve to the first
            // section node in that file by finding the shortest matching ID.
            let prefix = format!("{}#", resolved);
            graph.nodes.iter()
                .filter(|n| n.id.starts_with(&prefix))
                .min_by_key(|n| n.id.len())
                .map(|n| n.id.clone())
        };

        if let Some(target) = target {
            graph.edges.push(Edge {
                source: link.source_node.clone(),
                target,
                kind: EdgeKind::References,
            });
        }
    }
}
