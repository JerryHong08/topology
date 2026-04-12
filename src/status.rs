use anyhow::{Context, Result};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::graph::{EdgeKind, Graph, NodeKind};
use crate::scan::markdown::parse_markdown;

#[derive(Serialize)]
pub struct StatusOutput {
    pub total: usize,
    pub done: usize,
    pub todo: usize,
    pub stages: Vec<Stage>,
}

#[derive(Serialize)]
pub struct Stage {
    pub name: String,
    pub total: usize,
    pub done: usize,
    pub tasks: Vec<TaskSummary>,
}

#[derive(Serialize)]
pub struct TaskSummary {
    pub id: String,
    pub stable_id: Option<String>,
    pub label: String,
    pub status: String,
    pub subtasks: Option<SubtaskCount>,
}

#[derive(Serialize)]
pub struct SubtaskCount {
    pub total: usize,
    pub done: usize,
}

pub fn run(roadmap_path: &Path) -> Result<()> {
    let content = fs::read_to_string(roadmap_path)
        .with_context(|| format!("cannot read {}", roadmap_path.display()))?;
    let file_id = roadmap_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "ROADMAP.md".into());

    let mut graph = Graph::default();
    parse_markdown(&file_id, &content, &mut graph, &mut Vec::new());

    let output = build(&graph);
    print(&output);
    Ok(())
}

pub fn print(output: &StatusOutput) {
    println!("Progress: {}/{} tasks done", output.done, output.total);
    if output.todo > 0 {
        println!("Remaining: {}", output.todo);
    }
    println!();

    let mut archivable_sections: Vec<&str> = Vec::new();

    for stage in &output.stages {
        if stage.total == 0 {
            continue;
        }
        let pct = if stage.total > 0 {
            (stage.done * 100) / stage.total
        } else {
            0
        };
        println!("{} — {}/{} ({}%)", stage.name, stage.done, stage.total, pct);
        for task in &stage.tasks {
            let marker = match task.status.as_str() {
                "done" => "[x]",
                "in-progress" => "[-]",
                "dropped" => "[~]",
                _ => "[ ]",
            };
            let prefix = task.stable_id.as_deref().map(|sid| format!("{} ", sid)).unwrap_or_default();
            if let Some(sub) = &task.subtasks {
                println!("  {} {}{}/{} subtasks", marker, prefix, sub.done, sub.total);
            } else {
                println!("  {} {}{}", marker, prefix, task.label);
            }
        }
        println!();

        // Check if all tasks in this section are done or dropped
        if stage.total > 0 && stage.tasks.iter().all(|t| t.status == "done" || t.status == "dropped") {
            archivable_sections.push(&stage.name);
        }
    }

    if !archivable_sections.is_empty() {
        println!("Hint: {} — all tasks done/dropped. If no more tasks are planned for these sections, run `topo archive` to clean up.", archivable_sections.join(", "));
    }
}

pub fn build(graph: &Graph) -> StatusOutput {
    let mut children: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &graph.edges {
        if edge.kind == EdgeKind::Contains {
            children
                .entry(edge.source.clone())
                .or_default()
                .push(edge.target.clone());
        }
    }

    let node_map: HashMap<&str, &crate::graph::Node> =
        graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();

    // Helper function to recursively collect all descendant tasks
    fn collect_all_tasks<'a>(
        node_id: &str,
        children: &HashMap<String, Vec<String>>,
        node_map: &HashMap<&str, &'a crate::graph::Node>,
        tasks: &mut Vec<&'a crate::graph::Node>,
    ) {
        let empty = Vec::new();
        let child_ids = children.get(node_id).unwrap_or(&empty);

        for child_id in child_ids {
            if let Some(child) = node_map.get(child_id.as_str()) {
                if child.kind == NodeKind::Task {
                    tasks.push(child);
                } else if child.kind == NodeKind::Section {
                    // Recurse into subsections (H3, H4, etc.)
                    collect_all_tasks(child_id, children, node_map, tasks);
                }
            }
        }
    }

    let mut stages = Vec::new();
    let mut total_all = 0usize;
    let mut done_all = 0usize;

    for node in &graph.nodes {
        if node.kind != NodeKind::Section {
            continue;
        }
        let level = node
            .metadata
            .as_ref()
            .and_then(|m| m.get("level"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if level != 2 {
            continue;
        }

        // Collect all tasks from this section and its subsections
        let mut all_tasks: Vec<&crate::graph::Node> = Vec::new();
        collect_all_tasks(&node.id, &children, &node_map, &mut all_tasks);

        let mut stage_tasks = Vec::new();

        for task in all_tasks {
            // Get direct subtasks of this task
            let empty = Vec::new();
            let sub_ids = children.get(&task.id).unwrap_or(&empty);
            let subtask_nodes: Vec<_> = sub_ids
                .iter()
                .filter_map(|id| node_map.get(id.as_str()))
                .filter(|n| n.kind == NodeKind::Task)
                .collect();

            let (status, subtasks) = if subtask_nodes.is_empty() {
                let s = task
                    .metadata
                    .as_ref()
                    .and_then(|m| m.get("status"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("todo");
                (s.to_string(), None)
            } else {
                let sub_total = subtask_nodes.len();
                let sub_done = subtask_nodes
                    .iter()
                    .filter(|n| {
                        n.metadata
                            .as_ref()
                            .and_then(|m| m.get("status"))
                            .and_then(|v| v.as_str())
                            == Some("done")
                    })
                    .count();
                let s = if sub_done == sub_total { "done" } else { "todo" };
                (
                    s.to_string(),
                    Some(SubtaskCount {
                        total: sub_total,
                        done: sub_done,
                    }),
                )
            };

            let stable_id = task
                .metadata
                .as_ref()
                .and_then(|m| m.get("stable_id"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            stage_tasks.push(TaskSummary {
                id: task.id.clone(),
                stable_id,
                label: task.label.clone(),
                status,
                subtasks,
            });
        }

        let stage_done: usize = stage_tasks.iter().filter(|t| t.status == "done").count();
        let stage_total = stage_tasks.len();
        total_all += stage_total;
        done_all += stage_done;

        stages.push(Stage {
            name: node.label.clone(),
            total: stage_total,
            done: stage_done,
            tasks: stage_tasks,
        });
    }

    StatusOutput {
        total: total_all,
        done: done_all,
        todo: total_all - done_all,
        stages,
    }
}
