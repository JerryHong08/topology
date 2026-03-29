use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::graph::{Graph, NodeKind};
use crate::scan::markdown::parse_markdown;

fn is_task_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- [x] ")
        || t.starts_with("- [X] ")
        || t.starts_with("- [ ] ")
        || t.starts_with("- [-] ")
        || t.starts_with("- [~] ")
}

fn indent_level(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

fn find_task_line(content: &str, target_id: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    let mut slug_counts: HashMap<String, usize> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        if !is_task_line(line) {
            continue;
        }

        let trimmed = line.trim_start();
        let after_marker = trimmed.split("] ").nth(1)?;
        let raw_label = after_marker.trim();

        let label = match crate::scan::markdown::extract_numeric_id(raw_label) {
            Some((_, rest)) => rest,
            None => raw_label,
        };

        let slug = crate::scan::markdown::slugify(label);
        let file_id = "ROADMAP.md";
        let reconstructed_id = crate::scan::markdown::make_id(file_id, &slug, &mut slug_counts);

        if reconstructed_id == target_id || target_id.ends_with(&slug) || target_id.contains(&slug) {
            return Some(i);
        }
    }
    None
}

/// Delete a task from ROADMAP.md
/// Core operation used by both CLI and API
pub fn run(id: &str, root: &Path) -> Result<()> {
    let roadmap_path = root.join("ROADMAP.md");
    let content = std::fs::read_to_string(&roadmap_path)
        .with_context(|| format!("cannot read {}", roadmap_path.display()))?;

    // Parse to verify the node exists and is a task
    let mut graph = Graph::default();
    parse_markdown("ROADMAP.md", &content, &mut graph, &mut Vec::new());

    // Try to find the node by ID or stable_id
    let node = graph.nodes.iter().find(|n| {
        n.id == id || n.metadata.as_ref()
            .and_then(|m| m.get("stable_id"))
            .and_then(|v| v.as_str())
            == Some(id.split('#').nth(1).unwrap_or(id))
    });

    match node {
        None => anyhow::bail!("node '{}' not found", id),
        Some(n) if n.kind != NodeKind::Task => anyhow::bail!("node '{}' is not a task", id),
        _ => {}
    }

    let canonical_id = node.map(|n| n.id.clone()).unwrap_or(id.to_string());

    let line_idx = find_task_line(&content, &canonical_id)
        .ok_or_else(|| anyhow::anyhow!("task line for '{}' not found", id))?;

    let lines: Vec<&str> = content.lines().collect();
    let base_indent = indent_level(lines[line_idx]);

    // Collect all lines to delete (task + its subtasks + description)
    let mut delete_indices: Vec<usize> = vec![line_idx];
    let mut i = line_idx + 1;

    while i < lines.len() {
        let line = lines[i];
        let line_indent = indent_level(line);

        if line.trim().is_empty() {
            if i + 1 < lines.len() {
                let next = lines[i + 1];
                let next_indent = indent_level(next);
                if next_indent > base_indent || (is_task_line(next) && next_indent > base_indent) {
                    delete_indices.push(i);
                }
            }
            i += 1;
            continue;
        }

        if is_task_line(line) && line_indent > base_indent {
            delete_indices.push(i);
            i += 1;
        } else if line_indent > base_indent {
            delete_indices.push(i);
            i += 1;
        } else {
            break;
        }
    }

    // Build new content without deleted lines
    let mut new_lines: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if !delete_indices.contains(&i) {
            new_lines.push(line.to_string());
        }
    }

    let mut output = new_lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }

    std::fs::write(&roadmap_path, output)
        .with_context(|| format!("cannot write {}", roadmap_path.display()))?;

    // Update cache
    let graph = crate::scan::run_all(root)?;
    crate::scan::write_cache_for(root, &graph);

    Ok(())
}