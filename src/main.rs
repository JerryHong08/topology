mod add;
mod archive;
mod context;
mod delete;
mod diff;
mod graph;
mod ops;
mod output;
mod query;
mod resolve;
mod scan;
mod serve;
mod status;
mod unarchive;
mod update;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "topo", version, about = "Roadmap system — shared human-agent interface for task tracking")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory and build the topology graph
    Scan {
        /// Path to scan
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Output full JSON to stdout (default: summary only)
        #[arg(long)]
        json: bool,
    },
    /// Show task status summary from ROADMAP.md
    Status {
        /// Path to ROADMAP.md
        #[arg(long, default_value = "ROADMAP.md")]
        roadmap: PathBuf,
    },
    /// Show context for any node by ID, short hash, or slug
    Context {
        /// Node ID (short hash, slug, or full ID)
        id: String,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    /// Compare current scan against cached .topology.json
    Diff {
        /// Path to scan
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Show summary statistics (like git diff --stat)
        #[arg(long)]
        stat: bool,
    },
    /// Update a node in its source markdown file
    Update {
        /// Node ID (e.g. "ROADMAP.md#some-task")
        id: String,

        /// Field assignment (e.g. "status=done")
        assignment: String,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Add a new task to ROADMAP.md
    Add {
        /// Task description
        description: String,

        /// Section number to add task to (e.g., 1, 2, 3)
        #[arg(long)]
        section: usize,

        /// Create detail doc for discussion
        #[arg(long)]
        discuss: bool,

        /// Parent task ID (for subtasks)
        #[arg(long)]
        parent: Option<String>,

        /// Task description text (displayed under task)
        #[arg(long)]
        task_description: Option<String>,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Archive done/dropped tasks from ROADMAP.md to ARCHIVE.md
    Archive {
        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Preview what would be archived without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Delete a task from ROADMAP.md
    Delete {
        /// Task ID to delete (e.g., "1.1" or stable ID)
        id: String,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Restore archived tasks from ARCHIVE.md back to ROADMAP.md
    Unarchive {
        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Task ID to unarchive (e.g., "1.1"). If not specified, shows available archived tasks.
        #[arg()]
        task_id: Option<String>,

        /// Preview what would be unarchived without writing
        #[arg(long)]
        dry_run: bool,
    },
    /// Query the topology graph with traversal and filters
    Query {
        /// Filter expressions (e.g. -f type=task -f status=todo -f "label~keyword")
        #[arg(short = 'f', long = "filter")]
        filters: Vec<String>,

        /// Path to scan
        #[arg(long, default_value = ".")]
        path: PathBuf,

        /// Output format
        #[arg(long, default_value = "tree")]
        format: output::OutputFormat,

        /// Print only the count of matching nodes
        #[arg(long)]
        count: bool,

        /// Show only root nodes (no incoming edges)
        #[arg(long, group = "traversal")]
        roots: bool,

        /// Show direct children of a node
        #[arg(long, group = "traversal")]
        children: Option<String>,

        /// Show all descendants of a node
        #[arg(long, group = "traversal")]
        descendants: Option<String>,

        /// Show ancestors of a node
        #[arg(long, group = "traversal")]
        ancestors: Option<String>,

        /// Show nodes that a node references (outgoing reference edges)
        #[arg(long, group = "traversal")]
        references: Option<String>,

        /// Show nodes that reference a node (incoming reference edges)
        #[arg(long, group = "traversal")]
        referenced_by: Option<String>,

        /// Show the next sibling in document order (sequence edge)
        #[arg(long, group = "traversal")]
        next: Option<String>,

        /// Show status summary (stages, task counts, progress)
        #[arg(long)]
        status: bool,
    },
    /// Start web server with WebSocket real-time updates
    Serve {
        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Port to listen on
        #[arg(long, default_value = "7777")]
        port: u16,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan { path, json } => {
            let graph = scan::run_all(&path)?;
            scan::write_cache_for(&path, &graph);
            if json {
                output::print_json(&graph)?;
            } else {
                println!("scanned: {} nodes, {} edges", graph.nodes.len(), graph.edges.len());
            }
        }
        Commands::Status { roadmap } => {
            status::run(&roadmap)?;
        }
        Commands::Context { id, root, json } => {
            let graph = scan::run_cached(&root)?;
            let canonical = resolve::resolve(&graph, &id)?;
            context::run(&canonical, &graph, &root, json)?;
        }
        Commands::Diff { path, stat } => {
            if stat {
                diff::run_stat(&path)?;
            } else {
                diff::run(&path)?;
            }
        }
        Commands::Update { id, assignment, root } => {
            let graph = scan::run_cached(&root)?;
            let canonical = resolve::resolve(&graph, &id)?;
            update::run(&canonical, &assignment, &root)?;
        }
        Commands::Add { description, section, discuss, parent, task_description, root } => {
            let input = ops::AddTaskInput {
                description,
                section,
                parent,
                task_description,
            };
            add::run(&input.description, input.section, discuss, input.parent.as_deref(), input.task_description.as_deref(), &root)?;
        }
        Commands::Archive { root, dry_run } => {
            archive::run(&root, dry_run)?;
        }
        Commands::Delete { id, root } => {
            let graph = scan::run_cached(&root)?;
            let canonical = resolve::resolve(&graph, &id)?;
            delete::run(&canonical, &root)?;
        }
        Commands::Unarchive { root, task_id, dry_run } => {
            unarchive::run(&root, task_id.as_deref(), dry_run)?;
        }
        Commands::Query { filters, path, format, count, roots, children, descendants, ancestors, references, referenced_by, next, status: show_status } => {
            let graph = scan::run_cached(&path)?;

            if show_status {
                use std::collections::HashSet;
                let mut roadmap = graph.clone();
                roadmap.nodes.retain(|n| n.id.starts_with("ROADMAP.md"));
                let valid: HashSet<&str> = roadmap.nodes.iter().map(|n| n.id.as_str()).collect();
                roadmap.edges.retain(|e| valid.contains(e.source.as_str()) && valid.contains(e.target.as_str()));
                let s = status::build(&roadmap);
                // Print agent-native format instead of JSON
                println!("Progress: {}/{} tasks done", s.done, s.total);
                if s.todo > 0 {
                    println!("Remaining: {}", s.todo);
                }
                println!();
                for stage in &s.stages {
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
                return Ok(());
            }

            let resolve_id = |id: String| -> Result<String> {
                resolve::resolve(&graph, &id)
            };

            let traversal = if roots {
                query::Traversal::Roots
            } else if let Some(id) = children {
                query::Traversal::Children(resolve_id(id)?)
            } else if let Some(id) = descendants {
                query::Traversal::Descendants(resolve_id(id)?)
            } else if let Some(id) = ancestors {
                query::Traversal::Ancestors(resolve_id(id)?)
            } else if let Some(id) = references {
                query::Traversal::References(resolve_id(id)?)
            } else if let Some(id) = referenced_by {
                query::Traversal::ReferencedBy(resolve_id(id)?)
            } else if let Some(id) = next {
                query::Traversal::Next(resolve_id(id)?)
            } else {
                query::Traversal::None
            };
            let parsed: Vec<query::Filter> = filters
                .iter()
                .filter_map(|s| query::Filter::parse(s))
                .collect();
            let result = query::execute(&graph, &traversal, &parsed);
            if count {
                output::print_count(&result);
            } else {
                output::print_graph(&result, &format)?;
            }
        }
        Commands::Serve { root, port } => {
            tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime")
                .block_on(serve::run(root, port))?;
        }
    }
    Ok(())
}
