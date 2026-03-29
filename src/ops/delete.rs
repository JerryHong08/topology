use anyhow::Result;
use std::path::Path;

/// Delete a task from ROADMAP.md by ID
/// This is the core operation used by both CLI and API
pub fn run(task_id: &str, root: &Path) -> Result<()> {
    crate::delete::run(task_id, root)
}