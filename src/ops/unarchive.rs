use anyhow::Result;
use std::path::Path;

use super::UnarchiveInput;

/// Restore archived tasks from ARCHIVE.md back to ROADMAP.md
/// This is the core operation used by both CLI and API
pub fn run(root: &Path, input: &UnarchiveInput, dry_run: bool) -> Result<()> {
    crate::unarchive::run(root, input.task_id.as_deref(), dry_run)
}