use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

use super::UnarchiveInput;

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

fn extract_task_id(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !is_task_line(trimmed) {
        return None;
    }
    let after_marker = trimmed.split("] ").nth(1)?;
    let first_word = after_marker.split_whitespace().next()?;
    if first_word.contains('.') && first_word.chars().any(|c| c.is_ascii_digit()) {
        Some(first_word.to_string())
    } else {
        None
    }
}

/// Restore archived tasks from ARCHIVE.md back to ROADMAP.md
/// Core operation used by both CLI and API
pub fn run(root: &Path, input: &UnarchiveInput, dry_run: bool) -> Result<usize> {
    let roadmap_path = root.join("ROADMAP.md");
    let archive_path = root.join("ARCHIVE.md");

    let archive_content = std::fs::read_to_string(&archive_path)
        .map_err(|_| anyhow::anyhow!("ARCHIVE.md not found in {}", root.display()))?;

    let roadmap_content = std::fs::read_to_string(&roadmap_path)
        .map_err(|_| anyhow::anyhow!("ROADMAP.md not found in {}", root.display()))?;

    // Parse archive into sections
    let mut archive_sections: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current_section: Option<String> = None;
    let mut current_tasks: Vec<String> = Vec::new();

    for line in archive_content.lines() {
        if line.starts_with("## ") {
            if let Some(ref sec) = current_section {
                archive_sections.insert(sec.clone(), current_tasks.clone());
            }
            let title = line.trim_start_matches("## ").to_string();
            current_section = Some(title);
            current_tasks = Vec::new();
        } else if line.starts_with("# ") {
            continue;
        } else {
            current_tasks.push(line.to_string());
        }
    }
    if let Some(ref sec) = current_section {
        archive_sections.insert(sec.clone(), current_tasks.clone());
    }

    // Find tasks to unarchive
    let mut tasks_to_restore: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut remaining_archive: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (section, lines) in &archive_sections {
        let mut restored_lines: Vec<String> = Vec::new();
        let mut remaining_lines: Vec<String> = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = &lines[i];

            if is_task_line(line) {
                let base_indent = indent_level(line);
                let task_id_in_line = extract_task_id(line);

                let mut task_block = vec![line.clone()];
                i += 1;

                while i < lines.len() {
                    let next_line = &lines[i];
                    if is_task_line(next_line) && indent_level(next_line) > base_indent {
                        task_block.push(next_line.clone());
                        i += 1;
                    } else if next_line.trim().is_empty() {
                        i += 1;
                    } else {
                        break;
                    }
                }

                let should_restore = match &input.task_id {
                    Some(id) => {
                        task_id_in_line.as_ref().map(|tid| {
                            tid == id || tid.starts_with(&format!("{}.", id))
                        }).unwrap_or(false)
                    }
                    None => true,
                };

                if should_restore {
                    restored_lines.extend(task_block);
                } else {
                    remaining_lines.extend(task_block);
                }
            } else {
                remaining_lines.push(line.clone());
                i += 1;
            }
        }

        if !restored_lines.is_empty() {
            tasks_to_restore.insert(section.clone(), restored_lines);
        }
        if !remaining_lines.is_empty() {
            remaining_archive.insert(section.clone(), remaining_lines);
        }
    }

    if tasks_to_restore.is_empty() {
        return Ok(0);
    }

    if dry_run {
        return Ok(tasks_to_restore.values().map(|v| v.len()).sum());
    }

    // Parse ROADMAP.md into sections
    struct SectionContent {
        header: Vec<String>,
        tasks: Vec<String>,
        footer: Vec<String>,
    }

    let mut roadmap_sections: BTreeMap<String, SectionContent> = BTreeMap::new();
    let mut current_roadmap_section: Option<String> = None;
    let mut current_header: Vec<String> = Vec::new();
    let mut current_tasks: Vec<String> = Vec::new();
    let mut current_footer: Vec<String> = Vec::new();
    let mut pre_section_lines: Vec<String> = Vec::new();
    let mut in_tasks = false;
    let mut after_tasks = false;

    for line in roadmap_content.lines() {
        if line.starts_with("## ") {
            if let Some(ref sec) = current_roadmap_section {
                roadmap_sections.insert(sec.clone(), SectionContent {
                    header: current_header.clone(),
                    tasks: current_tasks.clone(),
                    footer: current_footer.clone(),
                });
            } else {
                pre_section_lines.extend(current_header.clone());
            }
            let title = line.trim_start_matches("## ").to_string();
            current_roadmap_section = Some(title);
            current_header = vec![line.to_string()];
            current_tasks = Vec::new();
            current_footer = Vec::new();
            in_tasks = false;
            after_tasks = false;
        } else if line.starts_with("# ") {
            if current_roadmap_section.is_none() {
                current_header.push(line.to_string());
            } else {
                current_header.push(line.to_string());
            }
        } else {
            if current_roadmap_section.is_none() {
                pre_section_lines.push(line.to_string());
            } else {
                if is_task_line(line) {
                    in_tasks = true;
                    after_tasks = false;
                    current_tasks.push(line.to_string());
                } else if in_tasks && line.trim().is_empty() {
                    current_tasks.push(line.to_string());
                } else if in_tasks && !line.trim().is_empty() && !is_task_line(line) {
                    after_tasks = true;
                    current_footer.push(line.to_string());
                } else {
                    if after_tasks {
                        current_footer.push(line.to_string());
                    } else {
                        current_header.push(line.to_string());
                    }
                }
            }
        }
    }
    if let Some(ref sec) = current_roadmap_section {
        roadmap_sections.insert(sec.clone(), SectionContent {
            header: current_header.clone(),
            tasks: current_tasks.clone(),
            footer: current_footer.clone(),
        });
    }

    // Merge restored tasks into roadmap sections
    for (section, tasks) in &tasks_to_restore {
        if let Some(content) = roadmap_sections.get_mut(section) {
            content.tasks.extend(tasks.clone());
        } else {
            roadmap_sections.insert(section.clone(), SectionContent {
                header: vec![format!("## {}", section), "".to_string()],
                tasks: tasks.clone(),
                footer: Vec::new(),
            });
        }
    }

    // Write updated ROADMAP.md
    let mut roadmap_out = String::new();

    for line in &pre_section_lines {
        roadmap_out.push_str(line);
        roadmap_out.push('\n');
    }

    let mut first_section = true;
    for (_section, content) in &roadmap_sections {
        if !first_section {
            roadmap_out.push('\n');
        }
        first_section = false;

        for line in &content.header {
            roadmap_out.push_str(line);
            roadmap_out.push('\n');
        }
        for line in &content.tasks {
            roadmap_out.push_str(line);
            roadmap_out.push('\n');
        }
        for line in &content.footer {
            roadmap_out.push_str(line);
            roadmap_out.push('\n');
        }
    }

    std::fs::write(&roadmap_path, roadmap_out)?;

    // Write updated ARCHIVE.md
    let mut archive_out = String::from("# Archive\n");
    for (section, lines) in &remaining_archive {
        if lines.iter().any(|l| !l.trim().is_empty()) {
            archive_out.push_str(&format!("\n## {}\n", section));
            for line in lines {
                if !line.trim().is_empty() {
                    archive_out.push_str(line);
                    archive_out.push('\n');
                }
            }
        }
    }
    std::fs::write(&archive_path, archive_out)?;

    let total: usize = tasks_to_restore.values().map(|v| v.len()).sum();
    Ok(total)
}