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

struct Block {
    lines: Vec<String>,
    archivable: bool,
    is_header: bool,
}

fn collect_blocks(lines: &[&str]) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut i = 0;
    let mut seen_first_task = false;

    while i < lines.len() {
        if !is_task_line(lines[i]) {
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

/// Archive done/dropped tasks from ROADMAP.md to ARCHIVE.md
/// Core operation used by both CLI and API
pub fn run(root: &Path, dry_run: bool) -> Result<usize> {
    let roadmap_path = root.join("ROADMAP.md");
    let content = std::fs::read_to_string(&roadmap_path)
        .map_err(|_| anyhow::anyhow!("cannot read {}", roadmap_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();

    let mut sections: Vec<(Option<String>, Vec<&str>)> = Vec::new();

    for line in &lines {
        if line.starts_with("## ") {
            let title = line.trim_start_matches("## ").to_string();
            sections.push((Some(title), Vec::new()));
        } else if sections.is_empty() {
            if sections.is_empty() {
                sections.push((None, Vec::new()));
            }
            sections.last_mut().unwrap().1.push(line);
        } else {
            sections.last_mut().unwrap().1.push(line);
        }
    }

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
                current_header.extend(block.lines.clone());
                remaining_lines.extend(block.lines.clone());
            } else if block.archivable {
                if let Some(name) = section_name {
                    let entry = archived.entry(name.clone()).or_default();
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
        return Ok(0);
    }

    if dry_run {
        return Ok(archived.values().map(|(_, blocks)| blocks.len()).sum());
    }

    // Write remaining ROADMAP.md
    let mut roadmap_out = remaining_lines.join("\n");
    if !roadmap_out.ends_with('\n') {
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
        if entry.is_empty() {
            for line in header {
                entry.push(line.clone());
            }
        }
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
        let mut prev_empty = false;
        for line in lines {
            let is_empty = line.trim().is_empty();
            if is_empty && prev_empty {
                continue;
            }
            out.push_str(line);
            out.push('\n');
            prev_empty = is_empty;
        }
    }
    std::fs::write(&archive_path, &out)?;

    let total: usize = archived.values().map(|(_, blocks)| blocks.len()).sum();
    Ok(total)
}