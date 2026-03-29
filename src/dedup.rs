use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::graph::{Graph, NodeKind};
use crate::scan::markdown::parse_markdown;

/// Renumber tasks in ROADMAP.md to ensure unique sequential IDs
pub fn run(root: &Path, dry_run: bool) -> Result<()> {
    let roadmap_path = root.join("ROADMAP.md");
    let content = fs::read_to_string(&roadmap_path)
        .with_context(|| format!("cannot read {}", roadmap_path.display()))?;

    // Parse to get current structure
    let mut graph = Graph::default();
    parse_markdown("ROADMAP.md", &content, &mut graph, &mut Vec::new());

    // Build section -> tasks mapping
    let mut section_tasks: HashMap<String, Vec<(String, String, String)>> = HashMap::new(); // section_id -> [(old_id, old_stable_id, label)]

    // Find all sections and their tasks
    for node in &graph.nodes {
        if node.kind == NodeKind::Section {
            section_tasks.entry(node.id.clone()).or_default();
        }
    }

    // Find tasks and their parent sections
    let mut task_section_map: HashMap<String, String> = HashMap::new();
    for edge in &graph.edges {
        if edge.kind == crate::graph::EdgeKind::Contains {
            let source_node = graph.nodes.iter().find(|n| n.id == edge.source);
            let target_node = graph.nodes.iter().find(|n| n.id == edge.target);

            if let (Some(source), Some(target)) = (source_node, target_node) {
                if source.kind == NodeKind::Section && target.kind == NodeKind::Task {
                    task_section_map.insert(target.id.clone(), source.id.clone());
                    section_tasks.entry(source.id.clone()).or_default().push((
                        target.id.clone(),
                        target.metadata.as_ref()
                            .and_then(|m| m.get("stable_id"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        target.label.clone(),
                    ));
                } else if source.kind == NodeKind::Task && target.kind == NodeKind::Task {
                    // Subtask - handle later
                }
            }
        }
    }

    // Extract section numbers from section labels
    let get_section_num = |section_id: &str| -> Option<usize> {
        let section = graph.nodes.iter().find(|n| n.id == section_id)?;
        let label = &section.label;
        let parts: Vec<&str> = label.split('.').collect();
        parts.first().and_then(|s| s.parse().ok())
    };

    // Build renumbering map
    let mut renumber_map: HashMap<String, String> = HashMap::new(); // old_stable_id -> new_stable_id

    for (section_id, tasks) in &section_tasks {
        let section_num = match get_section_num(section_id) {
            Some(n) => n,
            None => continue,
        };

        let mut task_counter = 0;
        for (_, old_stable_id, _) in tasks {
            if old_stable_id.is_empty() {
                continue;
            }
            task_counter += 1;
            let new_id = format!("{}.{}", section_num, task_counter);
            if old_stable_id != &new_id {
                renumber_map.insert(old_stable_id.clone(), new_id);
            }
        }
    }

    if renumber_map.is_empty() {
        println!("No renumbering needed");
        return Ok(());
    }

    if dry_run {
        println!("Would renumber:");
        for (old, new) in &renumber_map {
            println!("  {} -> {}", old, new);
        }
        return Ok(());
    }

    // Apply renumbering to file
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<String> = Vec::new();

    for line in &lines {
        let mut new_line = line.to_string();
        for (old, new) in &renumber_map {
            // Replace task ID in the format "X.X " at the start of task label
            let pattern = format!("{} ", old);
            let replacement = format!("{} ", new);
            if line.contains(&pattern) {
                new_line = new_line.replace(&pattern, &replacement);
            }
        }
        new_lines.push(new_line);
    }

    let mut output = new_lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }

    fs::write(&roadmap_path, output)
        .with_context(|| format!("cannot write {}", roadmap_path.display()))?;

    // Update cache
    let graph = crate::scan::run_all(root)?;
    crate::scan::write_cache_for(root, &graph);

    println!("Renumbered {} tasks", renumber_map.len());
    Ok(())
}