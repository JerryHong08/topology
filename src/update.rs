// This module wraps ops::update for backwards compatibility with CLI
// The core logic is in ops::update.rs

use anyhow::{bail, Result};
use std::path::Path;

use crate::ops::{self, UpdateTaskInput};

/// Legacy interface for CLI compatibility (accepts "field=value" string)
pub fn run(id: &str, assignment: &str, root: &Path) -> Result<()> {
    let (field, value) = assignment
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("invalid assignment, expected field=value"))?;

    if field != "status" {
        bail!("unsupported field '{}', only 'status' is supported", field);
    }

    let input = UpdateTaskInput {
        status: Some(value.to_string()),
    };

    ops::update::run(id, &input, root)?;
    println!("updated {} → status={}", id, value);
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
    fn task_with_numeric_id() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] 1.1 Scan project\n- [ ] 1.2 Query engine\n");
        run("T.md#scan-project", "status=done", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("- [x] 1.1 Scan project"));
        assert!(result.contains("- [ ] 1.2 Query engine"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_in_progress() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [ ] Alpha\n");
        run("T.md#alpha", "status=in-progress", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("- [-] Alpha"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn set_dropped() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [-] Alpha\n");
        run("T.md#alpha", "status=dropped", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("- [~] Alpha"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dropped_to_todo() {
        let dir = temp_dir();
        write_md(&dir, "T.md", "# S\n- [~] Alpha\n");
        run("T.md#alpha", "status=todo", &dir).unwrap();
        let result = std::fs::read_to_string(dir.join("T.md")).unwrap();
        assert!(result.contains("- [ ] Alpha"));
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