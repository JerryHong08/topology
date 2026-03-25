use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

fn is_task_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- [x] ")
        || t.starts_with("- [X] ")
        || t.starts_with("- [ ] ")
        || t.starts_with("- [-] ")
        || t.starts_with("- [~] ")
}

fn is_done_or_dropped(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("- [x] ")
        || t.starts_with("- [X] ")
        || t.starts_with("- [~] ")
}

fn indent_level(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// A contiguous block: a top-level task line + all deeper-indented task lines below it.
struct Block {
    lines: Vec<String>,
    archivable: bool,
    /// True if this is a section header (non-task content before first task)
    is_header: bool,
}

fn collect_blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0;
    let mut seen_first_task = false;

    while i < lines.len() {
        if !is_task_line(lines[i]) {
            // Non-task line
            // Check if this is a header (content before first task in section)
            let is_header = !seen_first_task;
            blocks.push(Block {
                lines: vec![lines[i].to_string()],
                archivable: false,
                is_header,
            });
            i += 1;
            continue;
        }

        seen_first_task = true;
        let base_indent = indent_level(lines[i]);
        let mut block_lines = vec![lines[i].to_string()];
        let mut all_done = is_done_or_dropped(lines[i]);
        i += 1;

        while i < lines.len() {
            if is_task_line(lines[i]) && indent_level(lines[i]) > base_indent {
                if !is_done_or_dropped(lines[i]) {
                    all_done = false;
                }
                block_lines.push(lines[i].to_string());
                i += 1;
            } else {
                break;
            }
        }

        blocks.push(Block {
            lines: block_lines,
            archivable: all_done,
            is_header: false,
        });
    }

    blocks
}

pub fn run(root: &Path, dry_run: bool) -> Result<()> {
    let roadmap_path = root.join("ROADMAP.md");
    let content = std::fs::read_to_string(&roadmap_path)
        .map_err(|_| anyhow::anyhow!("cannot read {}", roadmap_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();

    // Group lines by H2 section
    let mut sections: Vec<(Option<String>, Vec<&str>)> = Vec::new();

    for line in &lines {
        if line.starts_with("## ") {
            let title = line.trim_start_matches("## ").to_string();
            sections.push((Some(title), Vec::new()));
        } else if sections.is_empty() {
            // Lines before first H2
            if sections.is_empty() {
                sections.push((None, Vec::new()));
            }
            sections.last_mut().unwrap().1.push(line);
        } else {
            sections.last_mut().unwrap().1.push(line);
        }
    }

    // For each section, collect blocks, separate archivable vs remaining
    // archived: section_name -> (header_lines, Vec<task_blocks>)
    let mut archived: BTreeMap<String, (Vec<String>, Vec<Vec<String>>)> = BTreeMap::new();
    let mut remaining_lines: Vec<String> = Vec::new();

    for (section_name, section_lines) in &sections {
        if let Some(name) = section_name {
            remaining_lines.push(format!("## {}", name));
        }

        let blocks = collect_blocks(section_lines);
        let mut current_header: Vec<String> = Vec::new();

        for block in &blocks {
            if block.is_header {
                // Collect header lines
                current_header.extend(block.lines.clone());
                remaining_lines.extend(block.lines.clone());
            } else if block.archivable {
                if let Some(name) = section_name {
                    let entry = archived.entry(name.clone()).or_default();
                    // Store header if not already stored
                    if entry.0.is_empty() && !current_header.is_empty() {
                        entry.0 = current_header.clone();
                    }
                    entry.1.push(block.lines.clone());
                }
            } else {
                for l in &block.lines {
                    remaining_lines.push(l.clone());
                }
            }
        }
    }

    if archived.is_empty() {
        println!("nothing to archive");
        return Ok(());
    }

    if dry_run {
        println!("would archive:");
        for (section, (header, blocks)) in &archived {
            println!("\n## {}", section);
            for line in header {
                println!("{}", line);
            }
            for block in blocks {
                for line in block {
                    println!("{}", line);
                }
            }
        }
        return Ok(());
    }

    // Write remaining ROADMAP.md
    let mut roadmap_out = remaining_lines.join("\n");
    if content.ends_with('\n') {
        roadmap_out.push('\n');
    }
    std::fs::write(&roadmap_path, roadmap_out)?;

    // Read or create archive.md, append under section headers
    let archive_path = root.join("ARCHIVE.md");
    let existing = std::fs::read_to_string(&archive_path).unwrap_or_default();

    let mut archive_sections: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_archive_section: Option<String> = None;

    for line in existing.lines() {
        if line.starts_with("## ") {
            let title = line.trim_start_matches("## ").to_string();
            current_archive_section = Some(title.clone());
            archive_sections.entry(title).or_default();
        } else if line.starts_with("# ") {
            // Skip top-level header
        } else if let Some(ref sec) = current_archive_section {
            archive_sections.get_mut(sec).unwrap().push(line.to_string());
        }
    }

    // Merge new archived blocks
    for (section, (header, blocks)) in &archived {
        let entry = archive_sections.entry(section.clone()).or_default();
        // Add header first (for context)
        for line in header {
            entry.push(line.clone());
        }
        // Add archived tasks
        for block in blocks {
            for line in block {
                entry.push(line.clone());
            }
        }
    }

    // Write archive.md
    let mut out = String::from("# Archive\n");
    for (section, lines) in &archive_sections {
        out.push_str(&format!("\n## {}\n", section));
        for line in lines {
            if !line.trim().is_empty() {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    std::fs::write(&archive_path, &out)?;

    // Summary
    let total: usize = archived.values().map(|(_, blocks)| blocks.len()).sum();
    println!("archived {} task block(s) to ARCHIVE.md", total);

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
