use anyhow::Result;
use std::path::Path;

use super::AddTaskInput;

/// Add a new task to ROADMAP.md
/// This is the core operation used by both CLI and API
pub fn run(input: &AddTaskInput, root: &Path) -> Result<()> {
    crate::add::run(
        &input.description,
        input.section,
        false, // discuss - not needed for basic add
        input.parent.as_deref(),
        input.task_description.as_deref(),
        root,
    )
}