use anyhow::{bail, Result};
use std::collections::HashMap;
use std::path::Path;

use crate::graph::{Graph, NodeKind};
use crate::scan::markdown::{extract_numeric_id, make_id, parse_markdown, slugify};

use super::UpdateTaskInput;

/// Update a task in ROADMAP.md
/// Core operation used by both CLI and API
pub fn run(id: &str, input: &UpdateTaskInput, root: &Path) -> Result<()> {
    let status = input.status.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no status provided"))?;

    let valid = ["done", "todo", "in-progress", "dropped"];
    if !valid.contains(&status.as_str()) {
        bail!("unsupported value '{}', expected one of: {}", status, valid.join(", "));
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
        let is_task = trimmed.starts_with("- [x] ")
            || trimmed.starts_with("- [X] ")
            || trimmed.starts_with("- [ ] ")
            || trimmed.starts_with("- [-] ")
            || trimmed.starts_with("- [~] ");
        if !is_task {
            continue;
        }
        let prefix_len = "- [x] ".len();
        let raw_label = trimmed[prefix_len..].trim();
        let label = match extract_numeric_id(raw_label) {
            Some((_, rest)) => rest,
            None => raw_label,
        };
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
    let marker = match status.as_str() {
        "done" => "[x]",
        "todo" => "[ ]",
        "in-progress" => "[-]",
        "dropped" => "[~]",
        _ => bail!("unsupported value '{}'", status),
    };
    let new_line = line
        .replacen("- [ ] ", &format!("- {} ", marker), 1)
        .replacen("- [x] ", &format!("- {} ", marker), 1)
        .replacen("- [X] ", &format!("- {} ", marker), 1)
        .replacen("- [-] ", &format!("- {} ", marker), 1)
        .replacen("- [~] ", &format!("- {} ", marker), 1);
    new_lines[line_idx] = new_line;

    let mut output = new_lines.join("\n");
    if content.ends_with('\n') {
        output.push('\n');
    }
    std::fs::write(&file_path, output)?;
    Ok(())
}