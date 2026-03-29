// This module wraps ops::add for backwards compatibility with CLI
// The core logic is in ops::add.rs

use anyhow::Result;
use std::path::Path;

use crate::ops::{self, AddTaskInput};

/// Legacy interface for CLI compatibility
pub fn run(
    description: &str,
    section: usize,
    discuss: bool,
    parent: Option<&str>,
    task_description: Option<&str>,
    root: &Path,
) -> Result<()> {
    let input = AddTaskInput {
        description: description.to_string(),
        section,
        parent: parent.map(|s| s.to_string()),
        task_description: task_description.map(|s| s.to_string()),
    };

    let task_id = ops::add::run(&input, discuss, root)?;

    if discuss {
        let slug = crate::scan::markdown::slugify(description);
        println!("Added task {} with detail doc roadmap/{}.md", task_id, slug);
    } else {
        println!("Added task {}", task_id);
    }

    Ok(())
}