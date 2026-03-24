use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::graph::{EdgeKind, Graph, NodeKind};
use crate::scan::markdown::parse_markdown;

pub struct StatusOutput {
    pub total: usize,
    pub done: usize,
    pub todo: usize,
    pub stages: Vec<Stage>,
}

pub struct Stage {
    pub name: String,
    pub total: usize,
    pub done: usize,
    pub tasks: Vec<TaskSummary>,
}

pub struct TaskSummary {
    pub id: String,
    pub label: String,
    pub status: String,
    pub subtasks: Option<SubtaskCount>,
}

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

    // Print agent-native format
    println!("Progress: {}/{} tasks done", output.done, output.total);
    if output.todo > 0 {
        println!("Remaining: {}", output.todo);
    }
    println!();

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
            if let Some(sub) = &task.subtasks {
                println!("  {} {} {}/{} subtasks", marker, task.label, sub.done, sub.total);
            } else {
                println!("  {} {}", marker, task.label);
            }
        }
        println!();
    }

    Ok(())
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

        let mut stage_tasks = Vec::new();
        let empty = Vec::new();
        let top_level_ids = children.get(&node.id).unwrap_or(&empty);

        for child_id in top_level_ids {
            let Some(child) = node_map.get(child_id.as_str()) else {
                continue;
            };
            if child.kind != NodeKind::Task {
                continue;
            }

            let sub_ids = children.get(child_id.as_str()).unwrap_or(&empty);
            let subtask_nodes: Vec<_> = sub_ids
                .iter()
                .filter_map(|id| node_map.get(id.as_str()))
                .filter(|n| n.kind == NodeKind::Task)
                .collect();

            let (status, subtasks) = if subtask_nodes.is_empty() {
                let s = child
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

            stage_tasks.push(TaskSummary {
                id: child_id.clone(),
                label: child.label.clone(),
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
