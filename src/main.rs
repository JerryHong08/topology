mod graph;
mod output;
mod query;
mod scan;
mod status;
mod context;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "topology", version, about = "Project file structures into a unified graph")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a directory and output its topology as JSON
    Scan {
        /// Path to scan
        #[arg(default_value = ".")]
        path: PathBuf,

        /// Filter output to a specific layer (e.g. "filesystem", "markdown")
        #[arg(long)]
        layer: Option<String>,
    },
    /// Show task status summary from ROADMAP.md
    Status {
        /// Path to ROADMAP.md
        #[arg(long, default_value = "ROADMAP.md")]
        roadmap: PathBuf,
    },
    /// Load context for a task by name
    Context {
        /// Task name or slug to look up
        query: String,

        /// Project root directory
        #[arg(long, default_value = ".")]
        root: PathBuf,
    },
    /// Query the topology graph with traversal and filters
    Query {
        /// Filter expressions (e.g. type=task, status=todo, label~keyword)
        #[arg(trailing_var_arg = true)]
        filters: Vec<String>,

        /// Path to scan
        #[arg(long, default_value = ".")]
        path: PathBuf,

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
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Scan { path, layer } => {
            let graph = scan::run_all(&path, layer.as_deref())?;
            output::print_json(&graph)?;
        }
        Commands::Status { roadmap } => {
            status::run(&roadmap)?;
        }
        Commands::Context { query, root } => {
            context::run(&query, &root)?;
        }
        Commands::Query { filters, path, roots, children, descendants, ancestors } => {
            let graph = scan::run_all(&path, None)?;
            let traversal = if roots {
                query::Traversal::Roots
            } else if let Some(id) = children {
                query::Traversal::Children(id)
            } else if let Some(id) = descendants {
                query::Traversal::Descendants(id)
            } else if let Some(id) = ancestors {
                query::Traversal::Ancestors(id)
            } else {
                query::Traversal::None
            };
            let parsed: Vec<query::Filter> = filters
                .iter()
                .filter_map(|s| query::Filter::parse(s))
                .collect();
            let result = query::execute(&graph, &traversal, &parsed);
            output::print_json(&result)?;
        }
    }
    Ok(())
}
