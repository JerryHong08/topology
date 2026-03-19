use anyhow::Result;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::Scanner;
use crate::graph::{Edge, EdgeKind, Graph, Node, NodeKind};

pub struct MarkdownScanner;

impl Scanner for MarkdownScanner {
    fn scan(&self, root: &Path) -> Result<Graph> {
        let root = root.canonicalize()?;
        let mut graph = Graph::default();

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
            let file_id = if rel.as_os_str().is_empty() {
                root.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default()
            } else {
                rel.to_string_lossy().replace('\\', "/")
            };
            let content = fs::read_to_string(abs)?;
            parse_markdown(&file_id, &content, &mut graph);
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
    checked: bool,
    text: String,
    list_depth: usize,
    id: Option<String>,
}

fn make_id(file_id: &str, slug: &str, slug_counts: &mut HashMap<String, usize>) -> String {
    let count = slug_counts.entry(slug.to_string()).or_insert(0);
    *count += 1;
    if *count == 1 {
        format!("{file_id}#{slug}")
    } else {
        format!("{file_id}#{slug}-{}", *count)
    }
}

pub(crate) fn parse_markdown(file_id: &str, content: &str, graph: &mut Graph) {
    let parser = Parser::new_ext(content, Options::ENABLE_TASKLISTS);

    let mut heading_stack: Vec<(u8, String)> = Vec::new();
    let mut slug_counts: HashMap<String, usize> = HashMap::new();
    let mut task_stack: Vec<PendingTask> = Vec::new();
    let mut list_depth: usize = 0;

    let mut in_heading = false;
    let mut current_heading_level: u8 = 0;
    let mut heading_text = String::new();
    let mut in_code_block = false;

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
                    source: parent_id,
                    target: section_id.clone(),
                    kind: EdgeKind::Contains,
                });

                heading_stack.push((current_heading_level, section_id));
            }
            Event::Start(Tag::List(_)) => {
                list_depth += 1;
            }
            Event::End(TagEnd::List(_)) => {
                list_depth -= 1;
            }
            Event::TaskListMarker(checked) => {
                task_stack.push(PendingTask {
                    checked,
                    text: String::new(),
                    list_depth,
                    id: None,
                });
            }
            Event::End(TagEnd::Item)
                if !task_stack.is_empty()
                    && task_stack.last().unwrap().list_depth == list_depth =>
            {
                let mut task = task_stack.pop().unwrap();
                let label = task.text.trim().to_string();
                let slug = slugify(&label);

                let task_id = task
                    .id
                    .take()
                    .unwrap_or_else(|| make_id(file_id, &slug, &mut slug_counts));

                let status = if task.checked { "done" } else { "todo" };

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

                graph.nodes.push(Node {
                    id: task_id.clone(),
                    kind: NodeKind::Task,
                    source: "markdown".into(),
                    label,
                    metadata: Some(serde_json::json!({"status": status})),
                });
                graph.edges.push(Edge {
                    source: parent_id,
                    target: task_id,
                    kind: EdgeKind::Contains,
                });
            }
            Event::Text(text) => {
                if in_heading {
                    heading_text.push_str(&text);
                } else if !in_code_block {
                    if let Some(top) = task_stack.last_mut() {
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
                }
            }
            Event::Start(Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                in_code_block = false;
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
        parse_markdown("test.md", md, &mut g);
        g
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
        let g = scanner.scan(&file).unwrap();
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
    fn deterministic_output() {
        let md = "# B\n## A\n- [ ] Z\n- [ ] Y\n# C\n- [ ] X\n";
        let a = serde_json::to_string(&parse(md)).unwrap();
        let b = serde_json::to_string(&parse(md)).unwrap();
        assert_eq!(a, b);
    }
}
