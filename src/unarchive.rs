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

fn indent_level(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Extract task ID from a task line
fn extract_task_id(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !is_task_line(trimmed) {
        return None;
    }
    // Extract ID like "1.1" or "1.1.1" from "- [ ] 1.1 Task description"
    let after_marker = trimmed.split("] ").nth(1)?;
    let first_word = after_marker.split_whitespace().next()?;
    // Check if first word looks like a task ID (contains dots and digits)
    if first_word.contains('.') && first_word.chars().any(|c| c.is_ascii_digit()) {
        Some(first_word.to_string())
    } else {
        None
    }
}

pub fn run(root: &Path, task_id: Option<&str>, dry_run: bool) -> Result<()> {
    let roadmap_path = root.join("ROADMAP.md");
    let archive_path = root.join("ARCHIVE.md");

    // Check if archive exists
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
            // Save previous section
            if let Some(ref sec) = current_section {
                archive_sections.insert(sec.clone(), current_tasks.clone());
            }
            // Start new section
            let title = line.trim_start_matches("## ").to_string();
            current_section = Some(title);
            current_tasks = Vec::new();
        } else if line.starts_with("# ") {
            // Skip top-level header
            continue;
        } else {
            current_tasks.push(line.to_string());
        }
    }
    // Don't forget the last section
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

                // Collect this task and its subtasks
                let mut task_block = vec![line.clone()];
                i += 1;

                // Collect indented subtasks
                while i < lines.len() {
                    let next_line = &lines[i];
                    if is_task_line(next_line) && indent_level(next_line) > base_indent {
                        task_block.push(next_line.clone());
                        i += 1;
                    } else if next_line.trim().is_empty() {
                        // Skip empty lines between tasks
                        i += 1;
                    } else {
                        break;
                    }
                }

                // Decide whether to restore this task
                let should_restore = match task_id {
                    Some(id) => {
                        // Restore if this task or its parent matches
                        task_id_in_line.as_ref().map(|tid| {
                            tid == id || tid.starts_with(&format!("{}.", id))
                        }).unwrap_or(false)
                    }
                    None => {
                        // No specific ID - restore all tasks (or could prompt)
                        true
                    }
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
        println!("no tasks to unarchive");
        return Ok(());
    }

    if dry_run {
        println!("would unarchive:");
        for (section, tasks) in &tasks_to_restore {
            println!("\n## {}", section);
            for task in tasks {
                println!("{}", task);
            }
        }
        return Ok(());
    }

    // Parse ROADMAP.md into sections with proper structure
    // section -> (header_lines before tasks, task_lines, description_lines after tasks)
    let mut roadmap_sections: BTreeMap<String, SectionContent> = BTreeMap::new();

    struct SectionContent {
        header: Vec<String>,      // ## Section name + description before tasks
        tasks: Vec<String>,       // Task lines
        footer: Vec<String>,      // Description after tasks (if any)
    }

    let mut current_roadmap_section: Option<String> = None;
    let mut current_header: Vec<String> = Vec::new();
    let mut current_tasks: Vec<String> = Vec::new();
    let mut current_footer: Vec<String> = Vec::new();
    let mut pre_section_lines: Vec<String> = Vec::new();
    let mut in_tasks = false;
    let mut after_tasks = false;

    for line in roadmap_content.lines() {
        if line.starts_with("## ") {
            // Save previous section
            if let Some(ref sec) = current_roadmap_section {
                roadmap_sections.insert(sec.clone(), SectionContent {
                    header: current_header.clone(),
                    tasks: current_tasks.clone(),
                    footer: current_footer.clone(),
                });
            } else {
                // Lines before first H2
                pre_section_lines.extend(current_header.clone());
            }
            // Start new section
            let title = line.trim_start_matches("## ").to_string();
            current_roadmap_section = Some(title);
            current_header = vec![line.to_string()];
            current_tasks = Vec::new();
            current_footer = Vec::new();
            in_tasks = false;
            after_tasks = false;
        } else if line.starts_with("# ") {
            // H1 - add to pre_section_lines if before first H2
            if current_roadmap_section.is_none() {
                current_header.push(line.to_string());
            } else {
                current_header.push(line.to_string());
            }
        } else {
            if current_roadmap_section.is_none() {
                pre_section_lines.push(line.to_string());
            } else {
                // Check if this is a task line
                if is_task_line(line) {
                    in_tasks = true;
                    after_tasks = false;
                    current_tasks.push(line.to_string());
                } else if in_tasks && line.trim().is_empty() {
                    // Empty line after tasks - might be end of tasks
                    current_tasks.push(line.to_string());
                } else if in_tasks && !line.trim().is_empty() && !is_task_line(line) {
                    // Non-task, non-empty line after tasks = footer/description
                    after_tasks = true;
                    current_footer.push(line.to_string());
                } else {
                    // Before tasks or in footer
                    if after_tasks {
                        current_footer.push(line.to_string());
                    } else {
                        current_header.push(line.to_string());
                    }
                }
            }
        }
    }
    // Save last section
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
            // Section exists - append tasks
            content.tasks.extend(tasks.clone());
        } else {
            // Section doesn't exist - create it
            roadmap_sections.insert(section.clone(), SectionContent {
                header: vec![format!("## {}", section), "".to_string()],
                tasks: tasks.clone(),
                footer: Vec::new(),
            });
        }
    }

    // Write updated ROADMAP.md
    let mut roadmap_out = String::new();

    // Add pre-section lines (H1 title, intro, etc.)
    for line in &pre_section_lines {
        roadmap_out.push_str(line);
        roadmap_out.push('\n');
    }

    // Add sections with blank line between them
    let mut first_section = true;
    for (_section, content) in &roadmap_sections {
        // Add blank line before section (except first)
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

    // Summary
    let total: usize = tasks_to_restore.values().map(|v| v.len()).sum();
    println!("restored {} task(s) from ARCHIVE.md", total);

    Ok(())
}
