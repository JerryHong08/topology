use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;

use crate::graph::{Graph, NodeKind};
use crate::scan::markdown::{parse_markdown, slugify};

pub fn run(query: &str, root: &Path) -> Result<()> {
    let slug = slugify(query);
    let detail_path = root.join("roadmap").join(format!("{slug}.md"));

    if detail_path.exists() {
        let content = fs::read_to_string(&detail_path)
            .with_context(|| format!("cannot read {}", detail_path.display()))?;
        print!("{content}");
        return Ok(());
    }

    // Fallback: scan ROADMAP.md and find matching task
    let roadmap_path = root.join("ROADMAP.md");
    let content = fs::read_to_string(&roadmap_path)
        .with_context(|| "cannot read ROADMAP.md")?;
    let mut graph = Graph::default();
    parse_markdown("ROADMAP.md", &content, &mut graph);

    // Find the task whose slug matches the query
    let matching_task = graph.nodes.iter().find(|n| {
        if n.kind != NodeKind::Task {
            return false;
        }
        let Some(node_slug) = n.id.rsplit_once('#').map(|(_, s)| s) else {
            return false;
        };
        node_slug == slug || node_slug.starts_with(&format!("{slug}-"))
    });

    let Some(task) = matching_task else {
        bail!("no task matching '{}' found in ROADMAP.md", query);
    };

    // Collect subtask labels from edges
    let subtask_ids: Vec<&str> = graph
        .edges
        .iter()
        .filter(|e| e.source == task.id)
        .map(|e| e.target.as_str())
        .collect();

    println!("# {}", task.label);
    let status = task
        .metadata
        .as_ref()
        .and_then(|m| m.get("status"))
        .and_then(|v| v.as_str())
        .unwrap_or("todo");
    println!("Status: {status}");

    if !subtask_ids.is_empty() {
        println!("\n## Subtasks");
        for sid in &subtask_ids {
            if let Some(sub) = graph.nodes.iter().find(|n| n.id == *sid) {
                let sub_status = sub
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("todo");
                let mark = if sub_status == "done" { "x" } else { " " };
                println!("- [{mark}] {}", sub.label);
            }
        }
    }

    Ok(())
}
