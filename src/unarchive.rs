// This module wraps ops::unarchive for backwards compatibility with CLI
// The core logic is in ops::unarchive.rs

use anyhow::Result;
use std::path::Path;

use crate::ops::{self, UnarchiveInput};

/// Restore archived tasks from ARCHIVE.md back to ROADMAP.md
pub fn run(root: &Path, task_id: Option<&str>, dry_run: bool) -> Result<()> {
    let input = UnarchiveInput {
        task_id: task_id.map(|s| s.to_string()),
    };

    let count = ops::unarchive::run(root, &input, dry_run)?;

    if count == 0 {
        println!("no tasks to unarchive");
    } else if dry_run {
        println!("would unarchive {} task(s)", count);
    } else {
        println!("restored {} task(s) from ARCHIVE.md", count);
    }

    Ok(())
}