use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use crate::scan::markdown::{slugify, parse_markdown};
use crate::graph::{Graph, NodeKind, EdgeKind};

use super::AddTaskInput;

/// Collect all stable_ids from both ROADMAP.md and ARCHIVE.md
fn collect_all_ids(root: &Path) -> HashSet<String> {
    let mut ids = HashSet::new();

    for filename in ["ROADMAP.md", "ARCHIVE.md"] {
        let path = root.join(filename);
        if !path.exists() {
            continue;
        }
        if let Ok(content) = fs::read_to_string(&path) {
            let mut graph = Graph::default();
            parse_markdown(filename, &content, &mut graph, &mut Vec::new());
            for node in &graph.nodes {
                if node.kind == NodeKind::Task {
                    if let Some(meta) = &node.metadata {
                        if let Some(sid) = meta.get("stable_id").and_then(|v| v.as_str()) {
                            ids.insert(sid.to_string());
                        }
                    }
                }
            }
        }
    }
    ids
}

/// Add a new task to ROADMAP.md
/// Core operation used by both CLI and API
pub fn run(input: &AddTaskInput, discuss: bool, root: &Path) -> Result<String> {
    let roadmap_path = root.join("ROADMAP.md");

    // Read existing ROADMAP.md
    let content = fs::read_to_string(&roadmap_path)
        .with_context(|| format!("cannot read {}", roadmap_path.display()))?;

    // Parse to find the section and determine next task ID
    let mut graph = Graph::default();
    parse_markdown("ROADMAP.md", &content, &mut graph, &mut Vec::new());

    // Find the target section
    let section_prefix = format!("{}.", input.section);
    let section_node = graph
        .nodes
        .iter()
        .find(|n| {
            n.kind == NodeKind::Section
                && n.metadata
                    .as_ref()
                    .and_then(|m| m.get("level"))
                    .and_then(|v| v.as_u64())
                    == Some(2)
                && n.label.starts_with(&section_prefix)
        })
        .ok_or_else(|| anyhow::anyhow!("section {} not found", input.section))?;

    let section_name = &section_node.label;

    // Find existing tasks in this section to determine next ID
    let mut max_task_num = 0u32;

    // Build parent-child relationships
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Contains {
            children
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }
    }

    // Get direct children of section
    let empty_vec = Vec::new();
    let section_children = children.get(&section_node.id).unwrap_or(&empty_vec);

    for child_id in section_children {
        if let Some(node) = graph.nodes.iter().find(|n| &n.id == child_id) {
            if node.kind != NodeKind::Task {
                continue;
            }

            if let Some(sid) = node
                .metadata
                .as_ref()
                .and_then(|m| m.get("stable_id"))
                .and_then(|v| v.as_str())
            {
                let parts: Vec<&str> = sid.split('.').collect();
                if parts.len() == 2 {
                    if let Ok(num) = parts[1].parse::<u32>() {
                        max_task_num = max_task_num.max(num);
                    }
                }
            }
        }
    }

    // Generate new task ID, ensuring uniqueness across ROADMAP.md and ARCHIVE.md
    let all_ids = collect_all_ids(root);

    let new_task_id = if let Some(parent_id) = &input.parent {
        // Find parent task to get its numeric ID
        let parent_node = graph
            .nodes
            .iter()
            .find(|n| {
                n.metadata
                    .as_ref()
                    .and_then(|m| m.get("stable_id"))
                    .and_then(|v| v.as_str())
                    == Some(parent_id.as_str())
            })
            .ok_or_else(|| anyhow::anyhow!("parent task {} not found", parent_id))?;

        // Find next available subtask number
        let mut max_subtask = 0u32;
        let empty_vec2 = Vec::new();
        let parent_children = children.get(&parent_node.id).unwrap_or(&empty_vec2);

        for child_id in parent_children {
            if let Some(node) = graph.nodes.iter().find(|n| &n.id == child_id) {
                if let Some(sid) = node
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("stable_id"))
                    .and_then(|v| v.as_str())
                {
                    let parts: Vec<&str> = sid.split('.').collect();
                    if parts.len() == 3 {
                        if let Ok(num) = parts[2].parse::<u32>() {
                            max_subtask = max_subtask.max(num);
                        }
                    }
                }
            }
        }

        // Find next available ID that doesn't exist
        let mut candidate = max_subtask + 1;
        loop {
            let id = format!("{}.{}", parent_id, candidate);
            if !all_ids.contains(&id) {
                break id;
            }
            candidate += 1;
        }
    } else {
        // Find next available ID for this section
        let mut candidate = max_task_num + 1;
        loop {
            let id = format!("{}.{}", input.section, candidate);
            if !all_ids.contains(&id) {
                break id;
            }
            candidate += 1;
        }
    };

    // Create task line with optional description
    let indent = if input.parent.is_some() { "  " } else { "" };
    let task_line = format!("{}- [ ] {} {}", indent, new_task_id, input.description);

    // Add description line if provided (with blank line separator)
    let task_lines = if let Some(ref desc) = input.task_description {
        let desc_indent = if input.parent.is_some() { "    " } else { "  " };
        format!("{}\n\n{}{}", task_line, desc_indent, desc)
    } else {
        task_line
    };

    // Create detail doc if requested
    let detail_doc_path = if discuss {
        let slug = slugify(&input.description);
        let doc_path = root.join("roadmap").join(format!("{}.md", slug));
        fs::create_dir_all(root.join("roadmap"))?;

        let template = format!(
            r#"# Task: {}

## Context
Why this task exists. Current project state, user requirements.

## Analysis
- Related code/design in the current project
- Similar past decisions (check ARCHIVE.md)
- Risks and considerations

## Decision
Chosen approach and why.

## Rejected
Alternatives considered but discarded.

## Plan
Concrete implementation steps.
"#,
            input.description
        );

        fs::write(&doc_path, template)
            .with_context(|| format!("cannot write {}", doc_path.display()))?;

        Some(format!("roadmap/{}.md", slug))
    } else {
        None
    };

    // Insert task into ROADMAP.md
    let lines: Vec<&str> = content.lines().collect();
    let section_heading = format!("## {}", section_name);

    let mut insert_line = None;
    let mut in_target_section = false;
    let mut last_task_line: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        if line.starts_with(&section_heading) {
            in_target_section = true;
            continue;
        }

        if in_target_section {
            if line.starts_with("## ") || line.starts_with("# ") {
                insert_line = Some(i);
                break;
            }

            if line.trim().starts_with("- [") {
                last_task_line = Some(i);
            }

            if let Some(parent_id) = &input.parent {
                if line.contains(&format!("{} ", parent_id)) && line.trim().starts_with("- [") {
                    let mut j = i + 1;
                    while j < lines.len() {
                        let next = lines[j];
                        if next.trim().starts_with("- [") && !next.starts_with("  ") {
                            break;
                        }
                        if next.starts_with("## ") || next.starts_with("# ") {
                            break;
                        }
                        j += 1;
                    }
                    insert_line = Some(j);
                    break;
                }
            }
        }
    }

    let insert_idx = if insert_line.is_some() {
        insert_line.unwrap()
    } else if let Some(last_idx) = last_task_line {
        let mut end_idx = last_idx + 1;
        while end_idx < lines.len() {
            let next_line = lines[end_idx];
            if next_line.trim().starts_with("- [") {
                break;
            }
            if next_line.starts_with("## ") || next_line.starts_with("# ") {
                break;
            }
            if !next_line.is_empty() && !next_line.starts_with("  ") {
                break;
            }
            end_idx += 1;
        }
        end_idx
    } else {
        let mut start_idx = 0;
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with(&section_heading) {
                start_idx = i + 1;
                break;
            }
        }
        start_idx
    };

    // Build new content
    let mut new_lines: Vec<String> = lines.iter().map(|l| l.to_string()).collect();

    // Add detail doc link if created
    let final_task_lines = if let Some(ref doc_path) = detail_doc_path {
        let base_line = format!("{}- [ ] [{}]({}) {}", indent, new_task_id, doc_path, input.description);
        if let Some(ref desc) = input.task_description {
            let desc_indent = if input.parent.is_some() { "    " } else { "  " };
            format!("{}\n\n{}{}", base_line, desc_indent, desc)
        } else {
            base_line
        }
    } else {
        task_lines.clone()
    };

    // Insert task
    for line in final_task_lines.lines().rev() {
        new_lines.insert(insert_idx, line.to_string());
    }

    // Write back
    let mut output = new_lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }

    fs::write(&roadmap_path, output)
        .with_context(|| format!("cannot write {}", roadmap_path.display()))?;

    // Update cache
    let graph = crate::scan::run_all(root)?;
    crate::scan::write_cache_for(root, &graph);

    Ok(new_task_id)
}