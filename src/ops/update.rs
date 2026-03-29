use anyhow::Result;
use std::path::Path;

/// Update a task in ROADMAP.md
/// This is the core operation used by both CLI and API
pub fn run(task_id: &str, assignment: &str, root: &Path) -> Result<()> {
    crate::update::run(task_id, assignment, root)
}