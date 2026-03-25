use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::graph::{Edge, EdgeKind, Graph, Node, NodeKind};

pub struct MarkdownScanner;

pub struct RawLink {
    pub source_node: String,
    pub target_url: String,
    pub source_file: String,
}

impl MarkdownScanner {
    pub fn scan_with_links(&self, root: &Path, links: &mut Vec<RawLink>) -> Result<Graph> {
        let root = root.canonicalize()?;
        let mut graph = Graph::default();

        // Create a gitignore matcher for filtering
        let gitignore = ignore::gitignore::Gitignore::new(root.join(".gitignore")).0;

        for entry in ignore::WalkBuilder::new(&root)
            .hidden(false)
            .filter_entry(|e| e.file_name() != ".git")
            .build()
        {
            let entry = entry?;
            let abs = entry.path();
            if !abs.is_file() {
                continue;
            }
            let ext = abs.extension().and_then(|e| e.to_str());
            if ext != Some("md") {
                continue;
            }

            let rel = abs.strip_prefix(&root)?;
            let rel_str = rel.to_string_lossy();
            let file_id = if rel.as_os_str().is_empty() {
                root.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                rel_str.replace('\\', "/")
            };

            // Check if file is gitignored
            let is_gitignored = gitignore.matched(&rel, false).is_ignore();

            // Always allow ROADMAP.md and roadmap/*.md even if gitignored
            let is_roadmap = file_id == "ROADMAP.md" || file_id.starts_with("roadmap/");

            if is_gitignored && !is_roadmap {
                continue;
            }

            let content = fs::read_to_string(abs)?;
            parse_markdown(&file_id, &content, &mut graph, links);
        }

        Ok(graph)
    }
}

pub(crate) fn slugify(text: &str) -> String {
    let mut result = String::new();
    let mut prev_hyphen = true;
    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() {
            result.push(c);
            prev_hyphen = false;
        } else if !prev_hyphen {
            result.push('-');
            prev_hyphen = true;
        }
    }
    if result.ends_with('-') {
        result.pop();
    }
    result
}

fn heading_level_num(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

struct PendingTask {
    status: &'static str, // "todo", "done", "in-progress", "dropped"
    text: String,
    list_depth: usize,
    id: Option<String>,
    deferred_links: Vec<(String, String)>, // (target_url, source_file)
}

/// Extract a numeric task ID prefix like "1.1" or "1.1.1" from the start of a label.
/// Returns (numeric_id, remaining_label) if found.
pub fn extract_numeric_id(text: &str) -> Option<(&str, &str)> {
    let bytes = text.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }
    // Match pattern: digits followed by (.digits)+ then whitespace
    let mut i = 0;
    let mut dot_count = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            i += 1;
        } else if bytes[i] == b'.' && i + 1 < bytes.len() && bytes[i + 1].is_ascii_digit() {
            dot_count += 1;
            i += 1;
        } else {
            break;
        }
    }
    // Must have at least one dot (e.g. "1.1", not just "1")
    if dot_count == 0 {
        return None;
    }
    // Must be followed by whitespace
    if i >= bytes.len() || !bytes[i].is_ascii_whitespace() {
        return None;
    }
    let numeric_id = &text[..i];
    let rest = text[i..].trim_start();
    Some((numeric_id, rest))
}

/// Check if a list item starts with a custom task marker like `[-]` or `[~]`.
/// pulldown_cmark only recognizes `[ ]` and `[x]`, so we detect these manually.
/// Returns (marker_char, remaining_text) if found.
fn extract_custom_marker(text: &str) -> Option<(char, &str)> {
    let text = text.trim_start();
    if text.len() >= 3 && text.as_bytes()[0] == b'[' && text.as_bytes()[2] == b']' {
        let marker = text.as_bytes()[1] as char;
        if marker == '-' || marker == '~' {
            let rest = text[3..].trim_start();
            return Some((marker, rest));
        }
    }
    None
}

pub(crate) fn make_id(file_id: &str, slug: &str, slug_counts: &mut HashMap<String, usize>) -> String {
    let count = slug_counts.entry(slug.to_string()).or_insert(0);
    *count += 1;
    if *count == 1 {
        format!("{file_id}#{slug}")
    } else {
        format!("{file_id}#{slug}-{}", *count)
    }
}

fn looks_like_path(s: &str) -> bool {
    if s.contains('/') {
        return true;
    }
    const EXTENSIONS: &[&str] = &[
        ".rs", ".md", ".toml", ".json", ".yaml", ".yml",
        ".js", ".ts", ".py", ".go", ".sh", ".txt",
        ".html", ".css", ".xml",
    ];
    EXTENSIONS.iter().any(|ext| s.ends_with(ext))
}

pub(crate) fn parse_markdown(file_id: &str, content: &str, graph: &mut Graph, links: &mut Vec<RawLink>) {
    let parser = Parser::new_ext(content, Options::ENABLE_TASKLISTS);

    let mut heading_stack: Vec<(u8, String)> = Vec::new();
    let mut slug_counts: HashMap<String, usize> = HashMap::new();
    let mut task_stack: Vec<PendingTask> = Vec::new();
    let mut list_depth: usize = 0;
    let mut last_sibling: HashMap<String, String> = HashMap::new();

    let mut in_heading = false;
    let mut current_heading_level: u8 = 0;
    let mut heading_text = String::new();
    let mut in_code_block = false;
    let mut in_list_item = false;
    let mut is_task_item = false; // true if TaskListMarker was seen for this item
    let mut plain_item_text = String::new(); // accumulates text for non-task list items
    let mut item_links: Vec<(String, String)> = Vec::new(); // deferred links for current list item

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                current_heading_level = heading_level_num(level);
                heading_text.clear();
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
                let slug = slugify(&heading_text);
                let section_id = make_id(file_id, &slug, &mut slug_counts);

                while heading_stack
                    .last()
                    .is_some_and(|(lvl, _)| *lvl >= current_heading_level)
                {
                    heading_stack.pop();
                }

                let parent_id = heading_stack
                    .last()
                    .map(|(_, id)| id.clone())
                    .unwrap_or_else(|| file_id.to_string());

                graph.nodes.push(Node {
                    id: section_id.clone(),
                    kind: NodeKind::Section,
                    source: "markdown".into(),
                    label: heading_text.clone(),
                    metadata: Some(serde_json::json!({"level": current_heading_level})),
                });
                graph.edges.push(Edge {
                    source: parent_id.clone(),
                    target: section_id.clone(),
                    kind: EdgeKind::Contains,
                });

                if let Some(prev) = last_sibling.get(&parent_id) {
                    graph.edges.push(Edge {
                        source: prev.clone(),
                        target: section_id.clone(),
                        kind: EdgeKind::Sequence,
                    });
                }
                last_sibling.insert(parent_id, section_id.clone());

                heading_stack.push((current_heading_level, section_id));
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth -= 1;
            }
            Event::Start(Tag::Item) => {
                in_list_item = true;
                is_task_item = false;
                plain_item_text.clear();
                item_links.clear();
            }
            Event::TaskListMarker(checked) => {
                is_task_item = true;
                task_stack.push(PendingTask {
                    status: if checked { "done" } else { "todo" },
                    text: String::new(),
                    list_depth,
                    id: None,
                    deferred_links: Vec::new(),
                });
            }
            Event::End(TagEnd::Item)
                if !task_stack.is_empty()
                    && task_stack.last().unwrap().list_depth == list_depth =>
            {
                in_list_item = false;
                let mut task = task_stack.pop().unwrap();
                let raw_label = task.text.trim().to_string();

                // Extract numeric ID prefix if present (e.g. "1.1 Scan..." → id="1.1", label="Scan...")
                let (stable_id, label) = match extract_numeric_id(&raw_label) {
                    Some((nid, rest)) => (Some(nid.to_string()), rest.to_string()),
                    None => (None, raw_label),
                };

                let slug = slugify(&label);

                let task_id = task
                    .id
                    .take()
                    .unwrap_or_else(|| make_id(file_id, &slug, &mut slug_counts));

                let status = task.status;

                let parent_id = if let Some(parent) = task_stack.last_mut() {
                    if parent.id.is_none() {
                        let parent_slug = slugify(parent.text.trim());
                        parent.id = Some(make_id(file_id, &parent_slug, &mut slug_counts));
                    }
                    parent.id.clone().unwrap()
                } else {
                    heading_stack
                        .last()
                        .map(|(_, id)| id.clone())
                        .unwrap_or_else(|| file_id.to_string())
                };

                let mut meta = serde_json::json!({"status": status});
                if let Some(ref sid) = stable_id {
                    meta["stable_id"] = serde_json::json!(sid);
                }

                graph.nodes.push(Node {
                    id: task_id.clone(),
                    kind: NodeKind::Task,
                    source: "markdown".into(),
                    label,
                    metadata: Some(meta),
                });
                graph.edges.push(Edge {
                    source: parent_id.clone(),
                    target: task_id.clone(),
                    kind: EdgeKind::Contains,
                });

                if let Some(prev) = last_sibling.get(&parent_id) {
                    graph.edges.push(Edge {
                        source: prev.clone(),
                        target: task_id.clone(),
                        kind: EdgeKind::Sequence,
                    });
                }
                last_sibling.insert(parent_id, task_id.clone());

                // Drain deferred links from both the task and item_links
                for (target_url, source_file) in task.deferred_links.drain(..) {
                    links.push(RawLink {
                        source_node: task_id.clone(),
                        target_url,
                        source_file,
                    });
                }
                for (target_url, source_file) in item_links.drain(..) {
                    links.push(RawLink {
                        source_node: task_id.clone(),
                        target_url,
                        source_file,
                    });
                }
            }
            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else if !in_code_block {
                    if in_list_item && !is_task_item {
                        // Accumulate text for potential custom marker detection
                        plain_item_text.push_str(&text);
                    } else if let Some(top) = task_stack.last_mut() {
                        if list_depth == top.list_depth {
                            top.text.push_str(&text);
                        }
                    }
                }
            }
            Event::Code(code) => {
                if in_heading {
                    heading_text.push_str(&code);
                } else if !in_code_block {
                    if let Some(top) = task_stack.last_mut() {
                        if list_depth == top.list_depth {
                            top.text.push_str(&code);
                        }
                    }
                    if looks_like_path(&code) {
                        if in_list_item {
                            if let Some(top) = task_stack.last_mut().filter(|t| t.list_depth == list_depth) {
                                top.deferred_links.push((code.to_string(), file_id.to_string()));
                            } else {
                                item_links.push((code.to_string(), file_id.to_string()));
                            }
                        } else {
                            let source_node = heading_stack
                                .last()
                                .map(|(_, id)| id.clone())
                                .unwrap_or_else(|| file_id.to_string());
                            links.push(RawLink {
                                source_node,
                                target_url: code.to_string(),
                                source_file: file_id.to_string(),
                            });
                        }
                    }
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                if !in_code_block {
                    if in_list_item {
                        if let Some(top) = task_stack.last_mut().filter(|t| t.list_depth == list_depth) {
                            top.deferred_links.push((dest_url.to_string(), file_id.to_string()));
                        } else {
                            item_links.push((dest_url.to_string(), file_id.to_string()));
                        }
                    } else {
                        let source_node = heading_stack
                            .last()
                            .map(|(_, id)| id.clone())
                            .unwrap_or_else(|| file_id.to_string());
                        links.push(RawLink {
                            source_node,
                            target_url: dest_url.to_string(),
                            source_file: file_id.to_string(),
                        });
                    }
                }
            }
            Event::End(TagEnd::Item) => {
                // Check if this non-task list item has a custom marker [-] or [~]
                if in_list_item && !is_task_item && !plain_item_text.is_empty() {
                    if let Some((marker, rest)) = extract_custom_marker(&plain_item_text) {
                        let status = match marker {
                            '-' => "in-progress",
                            '~' => "dropped",
                            _ => unreachable!(),
                        };
                        let raw_label = rest.to_string();
                        let (stable_id, label) = match extract_numeric_id(&raw_label) {
                            Some((nid, remainder)) => (Some(nid.to_string()), remainder.to_string()),
                            None => (None, raw_label),
                        };
                        let slug = slugify(&label);
                        let task_id = make_id(file_id, &slug, &mut slug_counts);

                        let parent_id = heading_stack
                            .last()
                            .map(|(_, id)| id.clone())
                            .unwrap_or_else(|| file_id.to_string());

                        let mut meta = serde_json::json!({"status": status});
                        if let Some(ref sid) = stable_id {
                            meta["stable_id"] = serde_json::json!(sid);
                        }

                        graph.nodes.push(Node {
                            id: task_id.clone(),
                            kind: NodeKind::Task,
                            source: "markdown".into(),
                            label,
                            metadata: Some(meta),
                        });
                        graph.edges.push(Edge {
                            source: parent_id.clone(),
                            target: task_id.clone(),
                            kind: EdgeKind::Contains,
                        });

                        if let Some(prev) = last_sibling.get(&parent_id) {
                            graph.edges.push(Edge {
                                source: prev.clone(),
                                target: task_id.clone(),
                                kind: EdgeKind::Sequence,
                            });
                        }
                        last_sibling.insert(parent_id, task_id.clone());

                        // Drain deferred item links
                        for (target_url, source_file) in item_links.drain(..) {
                            links.push(RawLink {
                                source_node: task_id.clone(),
                                target_url,
                                source_file,
                            });
                        }
                    }
                }
                in_list_item = false;
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Graph, NodeKind};

    fn parse(md: &str) -> Graph {
        let mut g = Graph::default();
        let mut links = Vec::new();
        parse_markdown("test.md", md, &mut g, &mut links);
        g
    }

    fn parse_links(md: &str) -> Vec<RawLink> {
        let mut g = Graph::default();
        let mut links = Vec::new();
        parse_markdown("test.md", md, &mut g, &mut links);
        links
    }

    fn node_ids(g: &Graph, kind: NodeKind) -> Vec<&str> {
        g.nodes
            .iter()
            .filter(|n| n.kind == kind)
            .map(|n| n.id.as_str())
            .collect()
    }

    fn edge_pairs(g: &Graph) -> Vec<(&str, &str)> {
        g.edges.iter().map(|e| (e.source.as_str(), e.target.as_str())).collect()
    }

    fn seq_edges(g: &Graph) -> Vec<(&str, &str)> {
        g.edges.iter()
            .filter(|e| e.kind == EdgeKind::Sequence)
            .map(|e| (e.source.as_str(), e.target.as_str()))
            .collect()
    }

    #[test]
    fn heading_hierarchy() {
        let g = parse("# A\n## B\n### C\n");
        let ids = node_ids(&g, NodeKind::Section);
        assert_eq!(ids, vec!["test.md#a", "test.md#b", "test.md#c"]);
        let edges = edge_pairs(&g);
        assert!(edges.contains(&("test.md", "test.md#a")));
        assert!(edges.contains(&("test.md#a", "test.md#b")));
        assert!(edges.contains(&("test.md#b", "test.md#c")));
    }

    #[test]
    fn flat_tasks() {
        let g = parse("# Tasks\n- [ ] Alpha\n- [x] Beta\n");
        let ids = node_ids(&g, NodeKind::Task);
        assert_eq!(ids, vec!["test.md#alpha", "test.md#beta"]);
        let edges = edge_pairs(&g);
        assert!(edges.contains(&("test.md#tasks", "test.md#alpha")));
        assert!(edges.contains(&("test.md#tasks", "test.md#beta")));
        // check status metadata
        let beta = g.nodes.iter().find(|n| n.id == "test.md#beta").unwrap();
        assert_eq!(beta.metadata.as_ref().unwrap()["status"], "done");
    }

    #[test]
    fn nested_tasks() {
        let g = parse("# S\n- [ ] Parent\n  - [ ] Child\n");
        let ids = node_ids(&g, NodeKind::Task);
        assert!(ids.contains(&"test.md#parent"));
        assert!(ids.contains(&"test.md#child"));
        let edges = edge_pairs(&g);
        assert!(edges.contains(&("test.md#parent", "test.md#child")));
    }

    #[test]
    fn slug_dedup() {
        let g = parse("# S\n- [ ] Dup\n- [ ] Dup\n");
        let ids = node_ids(&g, NodeKind::Task);
        assert_eq!(ids.len(), 2);
        assert_eq!(ids[0], "test.md#dup");
        assert_eq!(ids[1], "test.md#dup-2");
    }

    #[test]
    fn single_file_id() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("topo_test_single_file");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("NOTE.md");
        let mut f = std::fs::File::create(&file).unwrap();
        writeln!(f, "# Hello\n- [ ] Do thing").unwrap();
        drop(f);

        let scanner = MarkdownScanner;
        let mut links = Vec::new();
        let g = scanner.scan_with_links(&file, &mut links).unwrap();
        assert!(g.nodes.iter().all(|n| n.id.starts_with("NOTE.md")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn code_block_skipping() {
        let md = "# S\n- [ ] Real task\n```\n- [ ] Not a task\n```\n";
        let g = parse(md);
        let tasks = node_ids(&g, NodeKind::Task);
        assert_eq!(tasks.len(), 1);
        assert_eq!(g.nodes.iter().find(|n| n.kind == NodeKind::Task).unwrap().label, "Real task");
    }

    #[test]
    fn numeric_id_extraction() {
        let g = parse("# S\n- [ ] 1.1 Scan project\n- [x] 1.2 Query done\n");
        let tasks: Vec<_> = g.nodes.iter().filter(|n| n.kind == NodeKind::Task).collect();
        assert_eq!(tasks.len(), 2);

        // Labels should have numeric ID stripped
        assert_eq!(tasks[0].label, "Scan project");
        assert_eq!(tasks[1].label, "Query done");

        // stable_id should be in metadata
        assert_eq!(tasks[0].metadata.as_ref().unwrap()["stable_id"], "1.1");
        assert_eq!(tasks[1].metadata.as_ref().unwrap()["stable_id"], "1.2");

        // slug-based ID should derive from label without numeric prefix
        assert_eq!(tasks[0].id, "test.md#scan-project");
        assert_eq!(tasks[1].id, "test.md#query-done");
    }

    #[test]
    fn numeric_id_nested() {
        let g = parse("# S\n- [ ] 1.1 Parent\n  - [ ] 1.1.1 Child\n");
        let tasks: Vec<_> = g.nodes.iter().filter(|n| n.kind == NodeKind::Task).collect();
        // Child is emitted first (inner item closes before outer)
        let child = tasks.iter().find(|t| t.label == "Child").unwrap();
        let parent = tasks.iter().find(|t| t.label == "Parent").unwrap();
        assert_eq!(child.metadata.as_ref().unwrap()["stable_id"], "1.1.1");
        assert_eq!(parent.metadata.as_ref().unwrap()["stable_id"], "1.1");
    }

    #[test]
    fn no_numeric_id_when_absent() {
        let g = parse("# S\n- [ ] Just a normal task\n");
        let task = g.nodes.iter().find(|n| n.kind == NodeKind::Task).unwrap();
        assert_eq!(task.label, "Just a normal task");
        assert!(task.metadata.as_ref().unwrap().get("stable_id").is_none());
    }

    #[test]
    fn custom_marker_in_progress() {
        let g = parse("# S\n- [-] 1.1 In progress task\n");
        let tasks: Vec<_> = g.nodes.iter().filter(|n| n.kind == NodeKind::Task).collect();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "In progress task");
        assert_eq!(tasks[0].metadata.as_ref().unwrap()["status"], "in-progress");
        assert_eq!(tasks[0].metadata.as_ref().unwrap()["stable_id"], "1.1");
    }

    #[test]
    fn custom_marker_dropped() {
        let g = parse("# S\n- [~] 2.1 Dropped task\n");
        let tasks: Vec<_> = g.nodes.iter().filter(|n| n.kind == NodeKind::Task).collect();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].label, "Dropped task");
        assert_eq!(tasks[0].metadata.as_ref().unwrap()["status"], "dropped");
    }

    #[test]
    fn deterministic_output() {
        let md = "# B\n## A\n- [ ] Z\n- [ ] Y\n# C\n- [ ] X\n";
        let a = serde_json::to_string(&parse(md)).unwrap();
        let b = serde_json::to_string(&parse(md)).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn link_extraction_path() {
        let links = parse_links("# S\nSee [readme](README.md) for details.\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_url, "README.md");
        assert_eq!(links[0].source_node, "test.md#s");
        assert_eq!(links[0].source_file, "test.md");
    }

    #[test]
    fn link_extraction_anchor() {
        let links = parse_links("# S\nSee [section](#other-section) below.\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_url, "#other-section");
    }

    #[test]
    fn link_extraction_inline_code_path() {
        let links = parse_links("# S\nCheck `src/main.rs` for entry point.\n");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_url, "src/main.rs");
    }

    #[test]
    fn link_extraction_skips_code_block() {
        let md = "# S\n```\n[not a link](path.md)\n`src/file.rs`\n```\n";
        let links = parse_links(md);
        assert!(links.is_empty());
    }

    #[test]
    fn link_extraction_skips_inline_code_in_heading() {
        let links = parse_links("# The `main.rs` module\nSome text.\n");
        assert!(links.is_empty());
    }

    #[test]
    fn link_extraction_multiple() {
        let md = "# S\n[a](one.md) and [b](two.md)\n";
        let links = parse_links(md);
        assert_eq!(links.len(), 2);
        assert_eq!(links[0].target_url, "one.md");
        assert_eq!(links[1].target_url, "two.md");
    }

    #[test]
    fn sequence_edges_sibling_tasks() {
        let g = parse("# S\n- [ ] A\n- [ ] B\n- [ ] C\n");
        let seq = seq_edges(&g);
        assert_eq!(seq.len(), 2);
        assert!(seq.contains(&("test.md#a", "test.md#b")));
        assert!(seq.contains(&("test.md#b", "test.md#c")));
    }

    #[test]
    fn sequence_edges_sibling_sections() {
        let g = parse("# S\n## X\n## Y\n");
        let seq = seq_edges(&g);
        assert_eq!(seq.len(), 1);
        assert!(seq.contains(&("test.md#x", "test.md#y")));
    }

    #[test]
    fn sequence_edges_nested_no_cross_level() {
        let g = parse("# S\n- [ ] A\n  - [ ] A1\n  - [ ] A2\n- [ ] B\n");
        let seq = seq_edges(&g);
        assert!(seq.contains(&("test.md#a", "test.md#b")));
        assert!(seq.contains(&("test.md#a1", "test.md#a2")));
        assert_eq!(seq.len(), 2);
    }

    #[test]
    fn task_link_source_is_task_node() {
        let md = "# S\n- [ ] My task — see [details](roadmap/detail.md)\n";
        let links = parse_links(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_url, "roadmap/detail.md");
        assert_eq!(links[0].source_node, "test.md#my-task-see-details");
    }

    #[test]
    fn task_code_path_source_is_task_node() {
        let md = "# S\n- [ ] Scan files — `src/scan/mod.rs`\n";
        let links = parse_links(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_url, "src/scan/mod.rs");
        assert_eq!(links[0].source_node, "test.md#scan-files-src-scan-mod-rs");
    }

    #[test]
    fn custom_marker_link_source_is_task_node() {
        let md = "# S\n- [~] 3.7 Dropped — see [details](roadmap/detail.md)\n";
        let links = parse_links(md);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_url, "roadmap/detail.md");
        assert_eq!(links[0].source_node, "test.md#dropped-see-details");
    }
}
