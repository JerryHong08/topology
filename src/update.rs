use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::graph::{Graph, NodeKind};
use crate::scan::markdown::{make_id, parse_markdown, slugify};

pub fn run(id: &str, assignment: &str, root: &Path) -> Result<()> {
    let (field, value) = assignment
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("invalid assignment, expected field=value"))?;

    if field != "status" {
        bail!("unsupported field '{}', only 'status' is supported", field);
    }
    if value != "done" && value != "todo" {
        bail!("unsupported value '{}', expected 'done' or 'todo'", value);
    }

    let (file_id, _slug) = id
        .split_once('#')
        .ok_or_else(|| anyhow::anyhow!("invalid ID, expected file#slug"))?;

    let file_path = root.join(file_id);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|_| anyhow::anyhow!("cannot read {}", file_path.display()))?;

    // Parse to verify the node exists and is a task
    let mut graph = Graph::default();
    parse_markdown(file_id, &content, &mut graph, &mut Vec::new());

    let node = graph.nodes.iter().find(|n| n.id == id);
    match node {
        None => bail!("node '{}' not found", id),
        Some(n) if n.kind != NodeKind::Task => bail!("node '{}' is not a task", id),
        _ => {}
    }

    // Reconstruct IDs line-by-line to find the target checkbox
    let lines: Vec<&str> = content.lines().collect();
    let mut slug_counts: HashMap<String, usize> = HashMap::new();
    let mut target_line: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        let is_checked = trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ");
        let is_unchecked = trimmed.starts_with("- [ ] ");
        if !is_checked && !is_unchecked {
            continue;
        }
        let prefix_len = "- [x] ".len();
        let label = trimmed[prefix_len..].trim();
        let slug = slugify(label);
        let reconstructed_id = make_id(file_id, &slug, &mut slug_counts);
        if reconstructed_id == id {
            target_line = Some(i);
            break;
        }
    }

    let line_idx = target_line.ok_or_else(|| anyhow::anyhow!("checkbox for '{}' not found", id))?;

    let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    let line = &new_lines[line_idx];
    let new_line = if value == "done" {
        line.replacen("- [ ] ", "- [x] ", 1)
    } else {
        line.replacen("- [x] ", "- [ ] ", 1)
            .replacen("- [X] ", "- [ ] ", 1)
    };
    new_lines[line_idx] = new_line;

    let mut output = new_lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }
    std::fs::write(&file_path, output)?;
    println!("updated {} → {}={}", id, field, value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("topo_update_{}_{}", std::process::id(), n));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn write_md(dir: &Path, name: &str, content: &str) -> std::path::PathBuf {
        let p = dir.join(name);
        let mut f = std::fs::File::create(&p).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        p
    }

    #[test]
    fn toggle_todo_to_done() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] Alpha\n- [ ] Beta\n");
        run("T.md#alpha", "status=done", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("- [x] Alpha"));
        assert!(result.contains("- [ ] Beta"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn toggle_done_to_todo() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [x] Alpha\n");
        run("T.md#alpha", "status=todo", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("- [ ] Alpha"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dedup_second_occurrence() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] Dup\n- [ ] Dup\n");
        run("T.md#dup-2", "status=done", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[1], "- [ ] Dup");
        assert_eq!(lines[2], "- [x] Dup");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nested_indented_task() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] Parent\n  - [ ] Child\n");
        run("T.md#child", "status=done", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("  - [x] Child"));
        assert!(result.contains("- [ ] Parent"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_node_not_found() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] Alpha\n");
        let err = run("T.md#nope", "status=done", &dir).unwrap_err();
        assert!(err.to_string().contains("not found"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_non_task_node() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# MySection\n- [ ] Alpha\n");
        let err = run("T.md#mysection", "status=done", &dir).unwrap_err();
        assert!(err.to_string().contains("not a task"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn error_unsupported_field() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] Alpha\n");
        let err = run("T.md#alpha", "priority=high", &dir).unwrap_err();
        assert!(err.to_string().contains("unsupported field"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
