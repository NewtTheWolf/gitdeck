use newt_todo_core::{
    Board, BoardCard, BoardColumn, RemoteCodeHit, RemoteComment, RemoteContributor, RemoteFork,
    RemoteIssueDetail, RemoteLanguage, RemoteNotification, RemotePath, RemotePullRequestDetail,
    RemoteReferrer, RemoteRelease, RemoteRepoDetail, RemoteReview, RemoteTraffic, RemoteTrafficDay,
    RemoteUser, RemoteWorkflowRun, RepoSnapshot, Task, TaskStatus,
};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;

/// Agent-facing view of a task (timestamps as RFC3339 strings).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub labels: Vec<String>,
    pub due_at: Option<String>,
    pub updated_at: String,
    /// Provider URL if this task mirrors a remote issue; null for local todos.
    pub source_url: Option<String>,
}

impl From<Task> for TaskDto {
    fn from(t: Task) -> Self {
        let fmt = |d: time::OffsetDateTime| d.format(&Rfc3339).unwrap_or_default();
        TaskDto {
            id: t.id.to_string(),
            title: t.title,
            body: t.body,
            status: t.status.as_str().to_string(),
            labels: t.labels,
            due_at: t.due_at.map(fmt),
            updated_at: fmt(t.local_updated_at),
            source_url: t.source.and_then(|s| s.html_url),
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateTaskParams {
    pub title: String,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// Optional RFC3339 due date.
    #[serde(default)]
    pub due_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ListTasksParams {
    /// Filter by status: "open" or "done".
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    /// Free-text match on title/body.
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TaskIdParam {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UpdateTaskParams {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    /// "open" or "done".
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub labels: Option<Vec<String>>,
    /// RFC3339 string to set a due date, or explicit null handling via `clear_due`.
    #[serde(default)]
    pub due_at: Option<String>,
    /// When true, clears the due date (overrides `due_at`).
    #[serde(default)]
    pub clear_due: bool,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SyncAccountParams {
    /// UUID of the account to sync.
    pub account_id: String,
}

pub(crate) fn parse_status(s: &str) -> anyhow::Result<TaskStatus> {
    TaskStatus::from_str(s).ok_or_else(|| anyhow::anyhow!("invalid status `{s}` (use open|done)"))
}

// --- boards -----------------------------------------------------------------

fn fmt_dt(d: time::OffsetDateTime) -> String {
    d.format(&Rfc3339).unwrap_or_default()
}

/// A board with its ordered columns (each carrying its ordered manual cards).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct BoardDto {
    pub id: String,
    pub name: String,
    pub position: i64,
    pub created_at: String,
    pub updated_at: String,
    pub columns: Vec<ColumnDto>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ColumnDto {
    pub id: String,
    pub name: String,
    pub position: i64,
    /// Opaque smart-filter JSON; interpreted by the frontend.
    pub filter: serde_json::Value,
    pub created_at: String,
    pub cards: Vec<CardDto>,
}

#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CardDto {
    pub id: String,
    pub item_key: String,
    pub position: i64,
}

impl From<BoardCard> for CardDto {
    fn from(c: BoardCard) -> Self {
        CardDto {
            id: c.id.to_string(),
            item_key: c.item_key,
            position: c.position,
        }
    }
}

impl ColumnDto {
    /// Build a column DTO from a column plus its manual cards (already filtered/ordered).
    pub(crate) fn from_parts(column: BoardColumn, cards: Vec<CardDto>) -> Self {
        ColumnDto {
            id: column.id.to_string(),
            name: column.name,
            position: column.position,
            filter: column.filter,
            created_at: fmt_dt(column.created_at),
            cards,
        }
    }
}

impl BoardDto {
    /// Build a board DTO from a board plus its assembled columns (ordered).
    pub(crate) fn from_parts(board: Board, columns: Vec<ColumnDto>) -> Self {
        BoardDto {
            id: board.id.to_string(),
            name: board.name,
            position: board.position,
            created_at: fmt_dt(board.created_at),
            updated_at: fmt_dt(board.updated_at),
            columns,
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateBoardParams {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RenameBoardParams {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct BoardIdParam {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReorderBoardsParams {
    /// Board ids in the desired order.
    pub ordered_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateColumnParams {
    pub board_id: String,
    pub name: String,
    /// Opaque smart-filter JSON. Defaults to `{}` (manual-only) when omitted.
    #[serde(default)]
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UpdateColumnParams {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub filter: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ColumnIdParam {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReorderColumnsParams {
    pub board_id: String,
    /// Column ids in the desired order.
    pub ordered_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PlaceCardParams {
    pub board_id: String,
    pub column_id: String,
    pub item_key: String,
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RemoveCardParams {
    pub board_id: String,
    pub item_key: String,
}

// --- notifications ----------------------------------------------------------

/// A GitHub notification thread (Inbox view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct NotificationDto {
    pub id: String,
    pub repo_name_with_owner: String,
    pub subject_title: String,
    pub subject_type: String,
    pub subject_url: Option<String>,
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
}

impl From<RemoteNotification> for NotificationDto {
    fn from(n: RemoteNotification) -> Self {
        NotificationDto {
            id: n.id,
            repo_name_with_owner: n.repo_name_with_owner,
            subject_title: n.subject_title,
            subject_type: n.subject_type,
            subject_url: n.subject_url,
            reason: n.reason,
            unread: n.unread,
            updated_at: n.updated_at,
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct MarkNotificationReadParams {
    /// UUID of the account the notification belongs to.
    pub id: String,
    /// GitHub notification thread id.
    pub thread_id: String,
}

// --- workflow runs ----------------------------------------------------------

/// A GitHub Actions workflow run (CI Health view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct WorkflowRunDto {
    pub id: u64,
    pub name: Option<String>,
    pub head_branch: Option<String>,
    pub status: String,
    pub conclusion: Option<String>,
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub event: Option<String>,
}

impl From<RemoteWorkflowRun> for WorkflowRunDto {
    fn from(r: RemoteWorkflowRun) -> Self {
        WorkflowRunDto {
            id: r.id,
            name: r.name,
            head_branch: r.head_branch,
            status: r.status,
            conclusion: r.conclusion,
            html_url: r.html_url,
            created_at: r.created_at,
            updated_at: r.updated_at,
            event: r.event,
        }
    }
}

#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ListWorkflowRunsParams {
    /// UUID of the account.
    pub id: String,
    pub owner: String,
    pub repo: String,
    /// Number of runs to fetch (capped at 20 by the provider).
    #[serde(default)]
    pub per_page: Option<u32>,
}

// --- repository details -----------------------------------------------------

/// Full detail for a single repository (Repository Details view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RepoDetailDto {
    pub full_name: String,
    pub description: Option<String>,
    pub html_url: String,
    pub homepage: Option<String>,
    pub language: Option<String>,
    pub stars: u64,
    pub forks: u64,
    pub open_issues: u64,
    pub watchers: u64,
    pub default_branch: String,
    pub license: Option<String>,
    pub topics: Vec<String>,
    pub owner_login: String,
    pub owner_avatar_url: Option<String>,
    pub is_private: bool,
    pub is_fork: bool,
    pub is_archived: bool,
    pub size: u64,
    pub pushed_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

impl From<RemoteRepoDetail> for RepoDetailDto {
    fn from(r: RemoteRepoDetail) -> Self {
        RepoDetailDto {
            full_name: r.full_name,
            description: r.description,
            html_url: r.html_url,
            homepage: r.homepage,
            language: r.language,
            stars: r.stars,
            forks: r.forks,
            open_issues: r.open_issues,
            watchers: r.watchers,
            default_branch: r.default_branch,
            license: r.license,
            topics: r.topics,
            owner_login: r.owner_login,
            owner_avatar_url: r.owner_avatar_url,
            is_private: r.is_private,
            is_fork: r.is_fork,
            is_archived: r.is_archived,
            size: r.size,
            pushed_at: r.pushed_at,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

/// A GitHub release (Repository Details view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReleaseDto {
    pub id: u64,
    pub tag_name: String,
    pub name: Option<String>,
    pub html_url: String,
    pub body: Option<String>,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: Option<String>,
    pub author_login: Option<String>,
}

impl From<RemoteRelease> for ReleaseDto {
    fn from(r: RemoteRelease) -> Self {
        ReleaseDto {
            id: r.id,
            tag_name: r.tag_name,
            name: r.name,
            html_url: r.html_url,
            body: r.body,
            draft: r.draft,
            prerelease: r.prerelease,
            published_at: r.published_at,
            author_login: r.author_login,
        }
    }
}

/// A fork of a repository (Repository Details view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ForkDto {
    pub full_name: String,
    pub html_url: String,
    pub stars: u64,
    pub pushed_at: Option<String>,
    pub owner_login: String,
}

impl From<RemoteFork> for ForkDto {
    fn from(f: RemoteFork) -> Self {
        ForkDto {
            full_name: f.full_name,
            html_url: f.html_url,
            stars: f.stars,
            pushed_at: f.pushed_at,
            owner_login: f.owner_login,
        }
    }
}

/// A contributor to a repository (Repository Details view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ContributorDto {
    pub login: String,
    pub avatar_url: Option<String>,
    pub html_url: String,
    pub contributions: u64,
}

impl From<RemoteContributor> for ContributorDto {
    fn from(c: RemoteContributor) -> Self {
        ContributorDto {
            login: c.login,
            avatar_url: c.avatar_url,
            html_url: c.html_url,
            contributions: c.contributions,
        }
    }
}

/// A language used in a repository (Repository Details view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LanguageDto {
    pub name: String,
    pub bytes: u64,
}

impl From<RemoteLanguage> for LanguageDto {
    fn from(l: RemoteLanguage) -> Self {
        LanguageDto {
            name: l.name,
            bytes: l.bytes,
        }
    }
}

// --- traffic ----------------------------------------------------------------

/// A single day of repository traffic (views or clones).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TrafficDayDto {
    pub timestamp: String,
    pub count: u64,
    pub uniques: u64,
}

impl From<RemoteTrafficDay> for TrafficDayDto {
    fn from(d: RemoteTrafficDay) -> Self {
        TrafficDayDto {
            timestamp: d.timestamp,
            count: d.count,
            uniques: d.uniques,
        }
    }
}

/// Repository traffic totals plus a per-day breakdown (Traffic view).
/// Used for both views and clones.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct TrafficDto {
    pub count: u64,
    pub uniques: u64,
    pub days: Vec<TrafficDayDto>,
}

impl From<RemoteTraffic> for TrafficDto {
    fn from(t: RemoteTraffic) -> Self {
        TrafficDto {
            count: t.count,
            uniques: t.uniques,
            days: t.days.into_iter().map(Into::into).collect(),
        }
    }
}

/// A popular referrer for a repository (Traffic view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReferrerDto {
    pub referrer: String,
    pub count: u64,
    pub uniques: u64,
}

impl From<RemoteReferrer> for ReferrerDto {
    fn from(r: RemoteReferrer) -> Self {
        ReferrerDto {
            referrer: r.referrer,
            count: r.count,
            uniques: r.uniques,
        }
    }
}

/// A popular content path for a repository (Traffic view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PathDto {
    pub path: String,
    pub title: String,
    pub count: u64,
    pub uniques: u64,
}

impl From<RemotePath> for PathDto {
    fn from(p: RemotePath) -> Self {
        PathDto {
            path: p.path,
            title: p.title,
            count: p.count,
            uniques: p.uniques,
        }
    }
}

// --- search (gitdeck repo modal "Mentions" tab) -----------------------------

/// A single code-search hit (Mentions tab, code search).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CodeHitDto {
    pub repo_name_with_owner: String,
    pub path: String,
    pub html_url: String,
    pub name: String,
}

impl From<RemoteCodeHit> for CodeHitDto {
    fn from(h: RemoteCodeHit) -> Self {
        CodeHitDto {
            repo_name_with_owner: h.repo_name_with_owner,
            path: h.path,
            html_url: h.html_url,
            name: h.name,
        }
    }
}

/// Params for the search endpoints (`search_code` / `search_issues`).
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SearchParams {
    /// UUID of the account whose token authorizes the search.
    pub id: String,
    /// The GitHub search query (forwarded verbatim).
    pub query: String,
    /// Number of results to fetch (capped at 30 by the provider).
    #[serde(default)]
    pub per_page: Option<u32>,
}

/// Shared params for the repository-detail endpoints. `per_page` is ignored by
/// `get_repo_detail` and `get_languages`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RepoRefParams {
    /// UUID of the account.
    pub id: String,
    pub owner: String,
    pub repo: String,
    #[serde(default)]
    pub per_page: Option<u32>,
}

// --- in-app issue/PR detail -------------------------------------------------

/// A GitHub user reference (login + optional avatar).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct UserDto {
    pub login: String,
    pub avatar_url: Option<String>,
}

impl From<RemoteUser> for UserDto {
    fn from(u: RemoteUser) -> Self {
        UserDto {
            login: u.login,
            avatar_url: u.avatar_url,
        }
    }
}

/// A single comment on an issue or PR (issue-comment timeline).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CommentDto {
    pub id: u64,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

impl From<RemoteComment> for CommentDto {
    fn from(c: RemoteComment) -> Self {
        CommentDto {
            id: c.id,
            author_login: c.author_login,
            author_avatar_url: c.author_avatar_url,
            body: c.body,
            created_at: c.created_at,
            updated_at: c.updated_at,
            html_url: c.html_url,
        }
    }
}

/// A single PR review with its overall state.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ReviewDto {
    pub id: u64,
    pub reviewer_login: String,
    pub reviewer_avatar_url: Option<String>,
    /// "APPROVED" | "CHANGES_REQUESTED" | "COMMENTED" | "DISMISSED" | "PENDING"
    pub state: String,
    pub body: Option<String>,
    pub submitted_at: Option<String>,
    pub html_url: String,
}

impl From<RemoteReview> for ReviewDto {
    fn from(r: RemoteReview) -> Self {
        ReviewDto {
            id: r.id,
            reviewer_login: r.reviewer_login,
            reviewer_avatar_url: r.reviewer_avatar_url,
            state: r.state,
            body: r.body,
            submitted_at: r.submitted_at,
            html_url: r.html_url,
        }
    }
}

/// Full detail for a single issue (in-app issue detail view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IssueDetailDto {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub state: String,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_name_with_owner: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub comments_count: u64,
    pub labels: Vec<LabelDto>,
    pub assignees: Vec<UserDto>,
    pub milestone_title: Option<String>,
}

/// A label on an issue/PR (name + hex color).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct LabelDto {
    pub name: String,
    pub color: String,
}

impl From<newt_todo_core::RemoteLabel> for LabelDto {
    fn from(l: newt_todo_core::RemoteLabel) -> Self {
        LabelDto {
            name: l.name,
            color: l.color,
        }
    }
}

impl From<RemoteIssueDetail> for IssueDetailDto {
    fn from(i: RemoteIssueDetail) -> Self {
        IssueDetailDto {
            number: i.number,
            title: i.title,
            body: i.body,
            html_url: i.html_url,
            state: i.state,
            author_login: i.author_login,
            author_avatar_url: i.author_avatar_url,
            repo_name_with_owner: i.repo_name_with_owner,
            created_at: i.created_at,
            updated_at: i.updated_at,
            closed_at: i.closed_at,
            comments_count: i.comments_count,
            labels: i.labels.into_iter().map(Into::into).collect(),
            assignees: i.assignees.into_iter().map(Into::into).collect(),
            milestone_title: i.milestone_title,
        }
    }
}

/// Full detail for a single pull request (in-app PR detail view).
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct PullRequestDetailDto {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub state: String,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_name_with_owner: String,
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub comments_count: u64,
    pub labels: Vec<LabelDto>,
    pub assignees: Vec<UserDto>,
    pub milestone_title: Option<String>,
    pub is_draft: bool,
    pub merged: bool,
    pub mergeable_state: Option<String>,
    pub base_ref: String,
    pub head_ref: String,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub commits: Option<u64>,
    pub requested_reviewers: Vec<UserDto>,
    pub requested_teams: Vec<String>,
}

impl From<RemotePullRequestDetail> for PullRequestDetailDto {
    fn from(p: RemotePullRequestDetail) -> Self {
        PullRequestDetailDto {
            number: p.number,
            title: p.title,
            body: p.body,
            html_url: p.html_url,
            state: p.state,
            author_login: p.author_login,
            author_avatar_url: p.author_avatar_url,
            repo_name_with_owner: p.repo_name_with_owner,
            created_at: p.created_at,
            updated_at: p.updated_at,
            closed_at: p.closed_at,
            comments_count: p.comments_count,
            labels: p.labels.into_iter().map(Into::into).collect(),
            assignees: p.assignees.into_iter().map(Into::into).collect(),
            milestone_title: p.milestone_title,
            is_draft: p.is_draft,
            merged: p.merged,
            mergeable_state: p.mergeable_state,
            base_ref: p.base_ref,
            head_ref: p.head_ref,
            additions: p.additions,
            deletions: p.deletions,
            changed_files: p.changed_files,
            commits: p.commits,
            requested_reviewers: p.requested_reviewers.into_iter().map(Into::into).collect(),
            requested_teams: p.requested_teams,
        }
    }
}

/// Shared params for the issue/PR detail endpoints (carries a `number`).
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IssueRefParams {
    /// UUID of the account.
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    /// Used by the comment/review listing endpoints (capped at 100).
    #[serde(default)]
    pub per_page: Option<u32>,
}

// --- triage write actions ---------------------------------------------------

/// Set an issue's state ("open" | "closed").
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SetIssueStateParams {
    /// UUID of the account whose token authorizes the write.
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    /// "open" or "closed".
    pub state: String,
}

/// Add labels to an issue (additive; does not replace existing labels).
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IssueLabelsParams {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub labels: Vec<String>,
}

/// Remove a single label from an issue.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct RemoveLabelParams {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub label: String,
}

/// Add assignees to an issue.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IssueAssigneesParams {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub assignees: Vec<String>,
}

/// Create a comment on an issue.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct IssueCommentParams {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub number: u64,
    pub body: String,
}

// --- daily repo snapshots ---------------------------------------------------

/// One persisted daily metric capture for a repository.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct SnapshotDto {
    pub id: String,
    pub account_id: String,
    pub repo_full_name: String,
    /// Calendar day in `YYYY-MM-DD`.
    pub day: String,
    pub stars: i64,
    pub forks: i64,
    pub open_issues: i64,
    /// RFC3339 timestamp of when this snapshot was written.
    pub captured_at: String,
}

impl From<RepoSnapshot> for SnapshotDto {
    fn from(s: RepoSnapshot) -> Self {
        SnapshotDto {
            id: s.id,
            account_id: s.account_id,
            repo_full_name: s.repo_full_name,
            day: s.day,
            stars: s.stars,
            forks: s.forks,
            open_issues: s.open_issues,
            captured_at: s.captured_at,
        }
    }
}

/// Params for `list_snapshots`.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct ListSnapshotsParams {
    /// UUID of the account.
    pub id: String,
    /// Optional inclusive lower bound (`YYYY-MM-DD`); snapshots with `day >= since_day`.
    #[serde(default)]
    pub since_day: Option<String>,
}

/// A per-repo entry in a daily digest: the latest day's absolute numbers plus
/// the delta versus the previous captured day. Deltas are 0 when there is no
/// previous day for that repo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DigestEntryDto {
    pub repo_full_name: String,
    pub stars_delta: i64,
    pub forks_delta: i64,
    pub open_issues_delta: i64,
    pub stars: i64,
    pub forks: i64,
    pub open_issues: i64,
}

/// A daily digest: the latest captured `day`, the previous captured `day` (if
/// any), and one entry per repo present on the latest day.
///
/// Semantics: `entries` contains ALL repos captured on `day` (not just changed
/// ones), each carrying its current absolute numbers and the delta against the
/// same repo on `previous_day`. If a repo had no row on `previous_day` (or
/// there is no previous day at all), its deltas equal its current values' diff
/// against 0 — i.e. when `previous_day` is None, all deltas are 0. The UI can
/// filter to nonzero deltas itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct DigestDto {
    /// Latest captured day, or None when the account has no snapshots at all.
    pub day: Option<String>,
    pub previous_day: Option<String>,
    pub entries: Vec<DigestEntryDto>,
}

/// Pure digest-diff: compute a [`DigestDto`] from a flat list of snapshots for a
/// single account. Independent of the database so it can be unit-tested directly.
///
/// - Picks the two most recent distinct days present in `snapshots`.
/// - `day` = the most recent; `previous_day` = the second-most-recent (or None).
/// - One entry per repo present on `day`, ordered by `repo_full_name`.
/// - Deltas are `latest - previous` per repo; if a repo is absent on
///   `previous_day` (or there is no previous day), its deltas are 0.
pub fn compute_digest(snapshots: &[RepoSnapshot]) -> DigestDto {
    // Distinct days, descending.
    let mut days: Vec<&str> = snapshots.iter().map(|s| s.day.as_str()).collect();
    days.sort_unstable();
    days.dedup();
    let latest = days.last().copied();
    let previous = if days.len() >= 2 {
        Some(days[days.len() - 2])
    } else {
        None
    };

    let Some(latest_day) = latest else {
        return DigestDto {
            day: None,
            previous_day: None,
            entries: Vec::new(),
        };
    };

    // Index previous-day numbers by repo for O(1) lookup.
    let prev_by_repo: BTreeMap<&str, &RepoSnapshot> = match previous {
        Some(prev_day) => snapshots
            .iter()
            .filter(|s| s.day == prev_day)
            .map(|s| (s.repo_full_name.as_str(), s))
            .collect(),
        None => BTreeMap::new(),
    };

    // Entries for the latest day, ordered by repo name (BTreeMap keeps them sorted).
    let latest_by_repo: BTreeMap<&str, &RepoSnapshot> = snapshots
        .iter()
        .filter(|s| s.day == latest_day)
        .map(|s| (s.repo_full_name.as_str(), s))
        .collect();

    let entries = latest_by_repo
        .into_values()
        .map(|cur| {
            let prev = prev_by_repo.get(cur.repo_full_name.as_str());
            let (ps, pf, pi) = prev.map(|p| (p.stars, p.forks, p.open_issues)).unwrap_or((
                cur.stars,
                cur.forks,
                cur.open_issues,
            ));
            DigestEntryDto {
                repo_full_name: cur.repo_full_name.clone(),
                stars_delta: cur.stars - ps,
                forks_delta: cur.forks - pf,
                open_issues_delta: cur.open_issues - pi,
                stars: cur.stars,
                forks: cur.forks,
                open_issues: cur.open_issues,
            }
        })
        .collect();

    DigestDto {
        day: Some(latest_day.to_string()),
        previous_day: previous.map(|p| p.to_string()),
        entries,
    }
}

#[cfg(test)]
mod digest_tests {
    use super::*;

    fn snap(repo: &str, day: &str, stars: i64, forks: i64, issues: i64) -> RepoSnapshot {
        RepoSnapshot {
            id: format!("{repo}-{day}"),
            account_id: "a".into(),
            repo_full_name: repo.into(),
            day: day.into(),
            stars,
            forks,
            open_issues: issues,
            captured_at: "2026-06-16T00:00:00Z".into(),
        }
    }

    #[test]
    fn empty_yields_no_day() {
        let d = compute_digest(&[]);
        assert_eq!(d.day, None);
        assert_eq!(d.previous_day, None);
        assert!(d.entries.is_empty());
    }

    #[test]
    fn one_day_yields_zero_deltas() {
        let snaps = vec![snap("o/a", "2026-06-16", 10, 2, 4)];
        let d = compute_digest(&snaps);
        assert_eq!(d.day.as_deref(), Some("2026-06-16"));
        assert_eq!(d.previous_day, None);
        assert_eq!(d.entries.len(), 1);
        let e = &d.entries[0];
        assert_eq!((e.stars_delta, e.forks_delta, e.open_issues_delta), (0, 0, 0));
        assert_eq!((e.stars, e.forks, e.open_issues), (10, 2, 4));
    }

    #[test]
    fn two_days_compute_correct_deltas() {
        let snaps = vec![
            snap("o/a", "2026-06-15", 10, 2, 4),
            snap("o/b", "2026-06-15", 5, 0, 1),
            snap("o/a", "2026-06-16", 13, 2, 1), // +3 stars, 0 forks, -3 issues
            snap("o/b", "2026-06-16", 5, 1, 1),  // 0 stars, +1 fork, 0 issues
            snap("o/c", "2026-06-16", 7, 0, 0),  // new repo: no previous → 0 deltas
        ];
        let d = compute_digest(&snaps);
        assert_eq!(d.day.as_deref(), Some("2026-06-16"));
        assert_eq!(d.previous_day.as_deref(), Some("2026-06-15"));
        // Ordered by repo_full_name: a, b, c.
        assert_eq!(
            d.entries.iter().map(|e| e.repo_full_name.as_str()).collect::<Vec<_>>(),
            vec!["o/a", "o/b", "o/c"]
        );
        assert_eq!(
            (d.entries[0].stars_delta, d.entries[0].open_issues_delta),
            (3, -3)
        );
        assert_eq!(d.entries[1].forks_delta, 1);
        // New repo on the latest day with no previous row → zero deltas.
        assert_eq!(
            (
                d.entries[2].stars_delta,
                d.entries[2].forks_delta,
                d.entries[2].open_issues_delta
            ),
            (0, 0, 0)
        );
        assert_eq!(d.entries[2].stars, 7);
    }
}
