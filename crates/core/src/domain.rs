use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Open,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Open => "open",
            TaskStatus::Done => "done",
        }
    }
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(TaskStatus::Open),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub account_id: Uuid,
    pub remote_id: String,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub source: Option<SourceRef>,
    pub title: String,
    pub body: String,
    pub status: TaskStatus,
    pub labels: Vec<String>,
    pub due_at: Option<OffsetDateTime>,
    pub local_updated_at: OffsetDateTime,
    pub dirty: bool,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDraft {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub due_at: Option<OffsetDateTime>,
}

impl TaskDraft {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            labels: Vec::new(),
            due_at: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub status: Option<TaskStatus>,
    pub labels: Option<Vec<String>>,
    /// Outer Option = "field present in patch"; inner Option = "set to None".
    pub due_at: Option<Option<OffsetDateTime>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub label: Option<String>,
    pub query: Option<String>,
    pub include_deleted: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_defaults_are_empty() {
        let d = TaskDraft::new("Buy milk");
        assert_eq!(d.title, "Buy milk");
        assert_eq!(d.body, "");
        assert!(d.labels.is_empty());
        assert!(d.due_at.is_none());
    }

    #[test]
    fn status_roundtrips_as_str() {
        assert_eq!(TaskStatus::Open.as_str(), "open");
        assert_eq!(TaskStatus::Done.as_str(), "done");
        assert_eq!(TaskStatus::from_str("done"), Some(TaskStatus::Done));
        assert_eq!(TaskStatus::from_str("nope"), None);
    }
}
