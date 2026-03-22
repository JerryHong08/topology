mod diff;
mod graph;
mod output;
mod query;
mod resolve;
mod scan;
mod status;
mod context;
mod update;
mod archive;

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
    /// Archive done/dropped tasks from ROADMAP.md to ARCHIVE.md
    Archive {
        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,

        /// Preview what would be archived without writing
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
        Commands::Archive { root, dry_run } => {
            archive::run(&root, dry_run)?;
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
                println!("{}", serde_json::to_string_pretty(&s)?);
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
    }
    Ok(())
}
