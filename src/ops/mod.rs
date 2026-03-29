pub mod add;
pub mod archive;
pub mod delete;
pub mod unarchive;
pub mod update;

use serde::{Deserialize, Serialize};

/// Common request format for adding a task
#[derive(Deserialize, Serialize, Clone)]
pub struct AddTaskInput {
    /// Task description/title
    pub description: String,
    /// Section number to add task to
    pub section: usize,
    /// Optional parent task ID for subtasks
    #[serde(default)]
    pub parent: Option<String>,
    /// Optional task description text
    /// Accepts both snake_case (task_description) and camelCase (taskDescription)
    #[serde(default, alias = "taskDescription")]
    pub task_description: Option<String>,
}

/// Common request format for updating a task
#[derive(Deserialize, Serialize, Clone)]
pub struct UpdateTaskInput {
    /// Task status: todo, done, in-progress, dropped
    #[serde(default)]
    pub status: Option<String>,
}

/// Common request format for unarchiving a task
#[derive(Deserialize, Serialize, Clone)]
pub struct UnarchiveInput {
    /// Task ID to unarchive (e.g., "1.1")
    #[serde(default, alias = "taskId")]
    pub task_id: Option<String>,
}

/// Common response format for operations
#[derive(Serialize)]
pub struct OpResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl OpResult {
    pub fn ok() -> Self {
        Self { success: true, id: None, error: None }
    }

    pub fn ok_with_id(id: String) -> Self {
        Self { success: true, id: Some(id), error: None }
    }

    pub fn err(msg: String) -> Self {
        Self { success: false, id: None, error: Some(msg) }
    }
}