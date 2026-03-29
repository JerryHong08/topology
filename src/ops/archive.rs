use anyhow::Result;
use std::path::Path;

/// Archive done/dropped tasks from ROADMAP.md to ARCHIVE.md
/// This is the core operation used by both CLI and API
pub fn run(root: &Path, dry_run: bool) -> Result<()> {
    crate::archive::run(root, dry_run)
}