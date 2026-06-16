pub mod account;
pub mod board;
pub mod domain;
pub mod provider;
pub mod store;
pub mod sync;

pub use account::{Account, AccountDraft, ProviderKind};
pub use board::{Board, BoardCard, BoardColumn};
pub use domain::{SourceRef, Task, TaskDraft, TaskFilter, TaskPatch, TaskStatus};
pub use provider::{
    Provider, ProviderError, RemoteCodeHit, RemoteComment, RemoteContributor, RemoteDraft,
    RemoteFork, RemoteIssue, RemoteIssueDetail, RemoteLabel, RemoteLanguage, RemoteNotification,
    RemotePatch, RemotePath, RemotePullRequest, RemotePullRequestDetail, RemoteReferrer,
    RemoteRelease, RemoteRepo, RemoteRepoDetail, RemoteReview, RemoteTask, RemoteTraffic,
    RemoteTrafficDay, RemoteUser, RemoteWorkflowRun,
};
pub use store::{today_ymd, RepoSnapshot, Store};
pub use sync::{sync_account, SyncReport};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("invalid data in field `{0}`")]
    DataFormat(&'static str),
    #[error("task not found: {0}")]
    NotFound(uuid::Uuid),
    #[error("provider failure: {0}")]
    Provider(String),
}
