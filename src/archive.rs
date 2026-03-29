// This module wraps ops::archive for backwards compatibility with CLI
// The core logic is in ops::archive.rs

use anyhow::Result;
use std::path::Path;

use crate::ops;

/// Archive done/dropped tasks from ROADMAP.md to ARCHIVE.md
pub fn run(root: &Path, dry_run: bool) -> Result<()> {
    let count = ops::archive::run(root, dry_run)?;

    if count == 0 {
        println!("nothing to archive");
    } else if dry_run {
        println!("would archive {} task block(s)", count);
    } else {
        println!("archived {} task block(s) to ARCHIVE.md", count);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("topo_archive_{}_{}", std::process::id(), n));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn archive_done_blocks() {
        let dir = temp_dir();
        std::fs::write(
            dir.join("ROADMAP.md"),
            "# Roadmap\n\n## 1. Core\n\n- [x] 1.1 Scan\n  - [x] 1.1.1 Parse\n- [ ] 1.2 Query\n",
        )
        .unwrap();

        run(&dir, false).unwrap();

        let roadmap = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert!(!roadmap.contains("1.1 Scan"));
        assert!(roadmap.contains("1.2 Query"));

        let archive = std::fs::read_to_string(dir.join("ARCHIVE.md")).unwrap();
        assert!(archive.contains("1.1 Scan"));
        assert!(archive.contains("1.1.1 Parse"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn block_with_incomplete_subtask_stays() {
        let dir = temp_dir();
        std::fs::write(
            dir.join("ROADMAP.md"),
            "# R\n\n## S\n\n- [x] Parent\n  - [ ] Child\n",
        )
        .unwrap();

        run(&dir, false).unwrap();

        let roadmap = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert!(roadmap.contains("Parent"));
        assert!(roadmap.contains("Child"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dropped_tasks_are_archivable() {
        let dir = temp_dir();
        std::fs::write(
            dir.join("ROADMAP.md"),
            "# R\n\n## S\n\n- [~] Dropped task\n- [ ] Active\n",
        )
        .unwrap();

        run(&dir, false).unwrap();

        let roadmap = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert!(!roadmap.contains("Dropped task"));
        assert!(roadmap.contains("Active"));

        let archive = std::fs::read_to_string(dir.join("ARCHIVE.md")).unwrap();
        assert!(archive.contains("Dropped task"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn nothing_to_archive() {
        let dir = temp_dir();
        std::fs::write(
            dir.join("ROADMAP.md"),
            "# R\n\n## S\n\n- [ ] Todo\n",
        )
        .unwrap();

        run(&dir, false).unwrap();

        let roadmap = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert!(roadmap.contains("Todo"));
        assert!(!dir.join("ARCHIVE.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dry_run_does_not_modify() {
        let dir = temp_dir();
        let original = "# R\n\n## S\n\n- [x] Done task\n- [ ] Todo\n";
        std::fs::write(dir.join("ROADMAP.md"), original).unwrap();

        run(&dir, true).unwrap();

        let roadmap = std::fs::read_to_string(dir.join("ROADMAP.md")).unwrap();
        assert_eq!(roadmap, original);
        assert!(!dir.join("ARCHIVE.md").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_to_existing_archive() {
        let dir = temp_dir();
        std::fs::write(
            dir.join("ARCHIVE.md"),
            "# Archive\n\n## S\n- [x] Old task\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("ROADMAP.md"),
            "# R\n\n## S\n\n- [x] New task\n",
        )
        .unwrap();

        run(&dir, false).unwrap();

        let archive = std::fs::read_to_string(dir.join("ARCHIVE.md")).unwrap();
        assert!(archive.contains("Old task"));
        assert!(archive.contains("New task"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}