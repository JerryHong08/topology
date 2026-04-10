mod add;
mod add_section;
mod archive;
mod context;
mod dedup;
mod delete;
mod diff;
mod graph;
mod layout;
mod move_section;
mod ops;
mod output;
mod query;
mod rename_section;
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
        assignment: Option<String>,

        /// Link to detail document (e.g. "roadmap/slug.md")
        #[arg(long)]
        link: Option<String>,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Add a new task to ROADMAP.md
    Add {
        /// Task description (can include ID prefix like "9.12 Task description")
        description: String,

        /// Section number to add task to (auto-detected if description has ID prefix)
        #[arg(long)]
        section: Option<usize>,

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
    /// Renumber tasks to ensure unique sequential IDs
    Dedup {
        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Preview what would be renumbered without writing
        #[arg(long)]
        dry_run: bool,
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
    /// Section management commands
    Section {
        #[command(subcommand)]
        command: SectionCommands,
    },
}

#[derive(Subcommand)]
enum SectionCommands {
    /// Add a new section to ROADMAP.md
    Add {
        /// Section title
        title: String,

        /// Section number (optional, auto-assigned if not specified)
        #[arg(long)]
        number: Option<usize>,

        /// Insert after this section number
        #[arg(long)]
        after: Option<usize>,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Rename a section
    Rename {
        /// Section number to rename
        number: usize,

        /// New title for the section
        title: String,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Move a section to a new position
    Move {
        /// Section number to move
        number: usize,

        /// Move after this section number
        #[arg(long)]
        after: usize,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
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
        Commands::Update { id, assignment, link, root } => {
            let graph = scan::run_cached(&root)?;
            let canonical = resolve::resolve(&graph, &id)?;
            update::run(&canonical, assignment.as_deref(), link.as_deref(), &root)?;
        }
        Commands::Add { description, section, discuss, parent, task_description, root } => {
            add::run(&description, section, discuss, parent.as_deref(), task_description.as_deref(), &root)?;
        }
        Commands::Archive { root, dry_run } => {
            archive::run(&root, dry_run)?;
        }
        Commands::Delete { id, root } => {
            let graph = scan::run_cached(&root)?;
            let canonical = resolve::resolve(&graph, &id)?;
            delete::run(&canonical, &root)?;
        }
        Commands::Dedup { root, dry_run } => {
            dedup::run(&root, dry_run)?;
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
                        let prefix = task.stable_id.as_deref().map(|sid| format!("{} ", sid)).unwrap_or_default();
                        if let Some(sub) = &task.subtasks {
                            println!("  {} {}{}/{} subtasks", marker, prefix, sub.done, sub.total);
                        } else {
                            println!("  {} {}{}", marker, prefix, task.label);
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
        Commands::Section { command } => {
            match command {
                SectionCommands::Add { title, number, after, root } => {
                    let input = ops::AddSectionInput {
                        title,
                        section_number: number,
                        after,
                    };
                    let id = add_section::run(&input, &root)?;
                    println!("created section {}", id);
                }
                SectionCommands::Rename { number, title, root } => {
                    let input = ops::UpdateSectionInput { title };
                    rename_section::run(number, &input, &root)?;
                    println!("renamed section {} → {}", number, input.title);
                }
                SectionCommands::Move { number, after, root } => {
                    let input = ops::MoveSectionInput { after };
                    move_section::run(number, &input, &root)?;
                    println!("moved section {} → after {}", number, after);
                }
            }
        }
    }
    Ok(())
}
