use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::graph::{Graph, NodeKind};
use crate::scan::markdown::{parse_markdown};

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

/// Find the line number of a task by its ID
fn find_task_line(content: &str, target_id: &str) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    let mut slug_counts: HashMap<String, usize> = HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        if !is_task_line(line) {
            continue;
        }

        // Extract label after checkbox marker
        let trimmed = line.trim_start();
        let after_marker = trimmed.split("] ").nth(1)?;
        let raw_label = after_marker.trim();

        // Strip numeric ID prefix if present
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

    // Find the canonical ID for this task
    let canonical_id = node.map(|n| n.id.clone()).unwrap_or(id.to_string());

    // Find the line number of the task
    let line_idx = find_task_line(&content, &canonical_id)
        .ok_or_else(|| anyhow::anyhow!("task line for '{}' not found", id))?;

    let lines: Vec<&str> = content.lines().collect();
    let base_indent = indent_level(lines[line_idx]);

    // Collect all lines to delete (task + its subtasks)
    let mut delete_indices: Vec<usize> = vec![line_idx];
    let mut i = line_idx + 1;

    while i < lines.len() {
        let line = lines[i];
        // Delete all deeper indented task lines (subtasks)
        if is_task_line(line) && indent_level(line) > base_indent {
            delete_indices.push(i);
            i += 1;
        } else if line.trim().is_empty() && i + 1 < lines.len() && is_task_line(lines[i + 1]) && indent_level(lines[i + 1]) > base_indent {
            // Empty line before subtask - delete it too
            delete_indices.push(i);
            i += 1;
        } else if indent_level(line) > base_indent {
            // Non-task content indented under task (like notes)
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

    println!("deleted task {}", id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("topo_delete_{}_{}", std::process::id(), n));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn delete_single_task() {
        let dir = temp_dir();
        std::fs::write(dir.join("ROADMAP.md"), "# S\n- [ ] Alpha\n- [ ] Beta\n").unwrap();
        run("ROADMAP.md#alpha", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert!(!result.contains("Alpha"));
        assert!(result.contains("Beta"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_task_with_subtasks() {
        let dir = temp_dir();
        std::fs::write(
            dir.join("ROADMAP.md"),
            "# S\n- [ ] Parent\n  - [ ] Child\n- [ ] Other\n",
        )
        .unwrap();
        run("ROADMAP.md#parent", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert!(!result.contains("Parent"));
        assert!(!result.contains("Child"));
        assert!(result.contains("Other"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_not_found() {
        let dir = temp_dir();
        std::fs::write(dir.join("ROADMAP.md"), "# S\n- [ ] Alpha\n").unwrap();
        let err = run("ROADMAP.md#beta", &dir).unwrap_err();
        assert!(err.to_string().contains("not found"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}