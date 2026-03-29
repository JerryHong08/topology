use anyhow::Result;
use std::path::Path;

use super::UpdateTaskInput;

/// Update a task in ROADMAP.md
/// This is the core operation used by both CLI and API
pub fn run(task_id: &str, input: &UpdateTaskInput, root: &Path) -> Result<()> {
    if let Some(ref status) = input.status {
        let assignment = format!("status={}", status);
        crate::update::run(task_id, &assignment, root)?;
    }
    Ok(())
}