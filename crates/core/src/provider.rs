use async_trait::async_trait;
use time::OffsetDateTime;

use crate::domain::TaskStatus;

/// A task as it exists on a provider (GitHub issue, Codeberg issue, ClickUp task).
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteTask {
    pub remote_id: String,
    pub title: String,
    pub body: String,
    pub status: TaskStatus,
    pub labels: Vec<String>,
    pub html_url: Option<String>,
    pub remote_updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteDraft {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

/// A repository as exposed by a provider (e.g. a GitHub repo).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteRepo {
    pub id: String,
    pub full_name: String, // "owner/name"
    pub description: Option<String>,
    pub html_url: String,
    pub stars: u64,
    pub open_issues: u64,
    pub language: Option<String>,
    pub is_private: bool,
    pub is_fork: bool,
    pub updated_at: time::OffsetDateTime,
}

/// A label on a GitHub issue or PR (name + hex color, no leading `#`).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteLabel {
    pub name: String,
    pub color: String,
}

/// An issue as surfaced by the dashboard (via the GitHub Search API).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteIssue {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String, // "open" | "closed"
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_name_with_owner: String, // "owner/name"
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub comments_count: u64,
    pub labels: Vec<RemoteLabel>,
    pub assignees: Vec<String>, // logins
}

/// A pull request as surfaced by the dashboard (via the GitHub Search API).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemotePullRequest {
    pub number: u64,
    pub title: String,
    pub html_url: String,
    pub state: String,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_name_with_owner: String,
    pub created_at: time::OffsetDateTime,
    pub updated_at: time::OffsetDateTime,
    pub comments_count: u64,
    pub labels: Vec<RemoteLabel>,
    pub assignees: Vec<String>,
    pub is_draft: bool,
}

/// A GitHub user reference (login + optional avatar). Used for assignees,
/// requested reviewers, and comment/review authors in the detail views.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteUser {
    pub login: String,
    pub avatar_url: Option<String>,
}

/// A single comment on an issue or PR (the issue-comment timeline).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteComment {
    pub id: u64,
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub body: String,
    pub created_at: String,
    pub updated_at: String,
    pub html_url: String,
}

/// A single PR review (approval/changes/comment) with its overall state.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteReview {
    pub id: u64,
    pub reviewer_login: String,
    pub reviewer_avatar_url: Option<String>,
    /// "APPROVED" | "CHANGES_REQUESTED" | "COMMENTED" | "DISMISSED" | "PENDING"
    pub state: String,
    pub body: Option<String>,
    pub submitted_at: Option<String>,
    pub html_url: String,
}

/// Full detail for a single issue (in-app issue detail view).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteIssueDetail {
    pub number: u64,
    pub title: String,
    pub body: Option<String>,
    pub html_url: String,
    pub state: String, // "open" | "closed"
    pub author_login: String,
    pub author_avatar_url: Option<String>,
    pub repo_name_with_owner: String, // "owner/name"
    pub created_at: String,
    pub updated_at: String,
    pub closed_at: Option<String>,
    pub comments_count: u64,
    pub labels: Vec<RemoteLabel>,
    pub assignees: Vec<RemoteUser>,
    pub milestone_title: Option<String>,
}

/// Full detail for a single pull request (in-app PR detail view). Carries all
/// of [`RemoteIssueDetail`] plus PR-specific fields (merge state, diffstat, refs,
/// requested reviewers).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemotePullRequestDetail {
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
    pub labels: Vec<RemoteLabel>,
    pub assignees: Vec<RemoteUser>,
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
    pub requested_reviewers: Vec<RemoteUser>,
    pub requested_teams: Vec<String>, // team slugs
}

/// A single code-search hit (gitdeck repo modal "Mentions" tab, code search).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteCodeHit {
    pub repo_name_with_owner: String, // "owner/name"
    pub path: String,
    pub html_url: String,
    pub name: String,
}

/// A GitHub notification thread (for the Inbox view).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteNotification {
    pub id: String, // thread id
    pub repo_name_with_owner: String,
    pub subject_title: String,
    pub subject_type: String, // "Issue" | "PullRequest" | "Release" | ...
    pub subject_url: Option<String>, // api url; may need converting to html later — keep raw
    pub reason: String,
    pub unread: bool,
    pub updated_at: String,
}

/// A GitHub Actions workflow run (for the CI Health view).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteWorkflowRun {
    pub id: u64,
    pub name: Option<String>,
    pub head_branch: Option<String>,
    pub status: String, // "queued" | "in_progress" | "completed"
    pub conclusion: Option<String>, // "success" | "failure" | "cancelled" | ...
    pub html_url: String,
    pub created_at: String,
    pub updated_at: String,
    pub event: Option<String>,
}

/// Full detail for a single repository (gitdeck "Repository Details" view).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteRepoDetail {
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
    pub license: Option<String>, // spdx id or name
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

/// A GitHub release for a repository.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteRelease {
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

/// A fork of a repository.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteFork {
    pub full_name: String,
    pub html_url: String,
    pub stars: u64,
    pub pushed_at: Option<String>,
    pub owner_login: String,
}

/// A contributor to a repository.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteContributor {
    pub login: String,
    pub avatar_url: Option<String>,
    pub html_url: String,
    pub contributions: u64,
}

/// A language used in a repository (name + bytes of code).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteLanguage {
    pub name: String,
    pub bytes: u64,
}

/// A single day of repository traffic (views or clones).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteTrafficDay {
    pub timestamp: String,
    pub count: u64,
    pub uniques: u64,
}

/// Repository traffic totals plus a per-day breakdown. Used for BOTH the
/// views and clones endpoints (they share the same shape).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteTraffic {
    pub count: u64,
    pub uniques: u64,
    pub days: Vec<RemoteTrafficDay>,
}

/// A popular referrer for a repository (traffic view).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemoteReferrer {
    pub referrer: String,
    pub count: u64,
    pub uniques: u64,
}

/// A popular content path for a repository (traffic view).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RemotePath {
    pub path: String,
    pub title: String,
    pub count: u64,
    pub uniques: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RemotePatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub status: Option<TaskStatus>,
    pub labels: Option<Vec<String>>,
}

#[derive(Debug, thiserror::Error)]
#[error("provider error: {0}")]
pub struct ProviderError(pub String);

#[async_trait]
pub trait Provider: Send + Sync {
    async fn list_tasks(&self) -> Result<Vec<RemoteTask>, ProviderError>;
    async fn create_task(&self, draft: RemoteDraft) -> Result<RemoteTask, ProviderError>;
    async fn update_task(
        &self,
        remote_id: &str,
        patch: RemotePatch,
    ) -> Result<RemoteTask, ProviderError>;

    /// List repositories visible to the authenticated account.
    ///
    /// Defaulted so existing providers that don't support repo listing
    /// (e.g. the test `FakeProvider`) keep compiling without changes.
    async fn list_repos(&self) -> Result<Vec<RemoteRepo>, ProviderError> {
        Ok(Vec::new())
    }

    /// List issues that involve the authenticated account.
    ///
    /// Defaulted so providers that don't support issue search keep compiling.
    async fn list_issues(&self) -> Result<Vec<RemoteIssue>, ProviderError> {
        Ok(Vec::new())
    }

    /// List pull requests that involve the authenticated account.
    ///
    /// Defaulted so providers that don't support PR search keep compiling.
    async fn list_pull_requests(&self) -> Result<Vec<RemotePullRequest>, ProviderError> {
        Ok(Vec::new())
    }

    /// Search code across GitHub (gitdeck repo modal "Mentions" tab).
    ///
    /// Defaulted so providers that don't support code search keep compiling.
    async fn search_code(
        &self,
        _query: &str,
        _per_page: u32,
    ) -> Result<Vec<RemoteCodeHit>, ProviderError> {
        Ok(Vec::new())
    }

    /// Search issues/PRs across GitHub (gitdeck repo modal "Mentions" tab).
    ///
    /// Defaulted so providers that don't support issue search keep compiling.
    async fn search_issues(
        &self,
        _query: &str,
        _per_page: u32,
    ) -> Result<Vec<RemoteIssue>, ProviderError> {
        Ok(Vec::new())
    }

    /// List notification threads for the authenticated account (Inbox view).
    ///
    /// Defaulted so providers that don't support notifications keep compiling.
    async fn list_notifications(&self) -> Result<Vec<RemoteNotification>, ProviderError> {
        Ok(Vec::new())
    }

    /// Mark a single notification thread as read.
    ///
    /// Defaulted so providers that don't support notifications keep compiling.
    async fn mark_notification_read(&self, _thread_id: &str) -> Result<(), ProviderError> {
        Ok(())
    }

    /// List recent workflow runs for a repo (CI Health view).
    ///
    /// Defaulted so providers that don't support workflow runs keep compiling.
    async fn list_workflow_runs(
        &self,
        _owner: &str,
        _repo: &str,
        _per_page: u32,
    ) -> Result<Vec<RemoteWorkflowRun>, ProviderError> {
        Ok(Vec::new())
    }

    /// Get full detail for a single repository (Repository Details view).
    ///
    /// Defaulted so providers that don't support repo detail keep compiling.
    /// Returns `Ok(None)` when the repo doesn't exist.
    async fn get_repo_detail(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Option<RemoteRepoDetail>, ProviderError> {
        Ok(None)
    }

    /// List releases for a repository.
    ///
    /// Defaulted so providers that don't support releases keep compiling.
    async fn list_releases(
        &self,
        _owner: &str,
        _repo: &str,
        _per_page: u32,
    ) -> Result<Vec<RemoteRelease>, ProviderError> {
        Ok(Vec::new())
    }

    /// List forks for a repository.
    ///
    /// Defaulted so providers that don't support forks keep compiling.
    async fn list_forks(
        &self,
        _owner: &str,
        _repo: &str,
        _per_page: u32,
    ) -> Result<Vec<RemoteFork>, ProviderError> {
        Ok(Vec::new())
    }

    /// List contributors for a repository.
    ///
    /// Defaulted so providers that don't support contributors keep compiling.
    async fn list_contributors(
        &self,
        _owner: &str,
        _repo: &str,
        _per_page: u32,
    ) -> Result<Vec<RemoteContributor>, ProviderError> {
        Ok(Vec::new())
    }

    /// Get language breakdown for a repository (sorted by bytes desc).
    ///
    /// Defaulted so providers that don't support languages keep compiling.
    async fn get_languages(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<RemoteLanguage>, ProviderError> {
        Ok(Vec::new())
    }

    /// Get the daily view traffic for a repository (Traffic view).
    ///
    /// Requires push access; for repos the user can't push to GitHub returns
    /// 403 (and 404 when missing) — both map to `Ok(None)` so the UI degrades.
    /// Defaulted so providers that don't support traffic keep compiling.
    async fn get_traffic_views(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Option<RemoteTraffic>, ProviderError> {
        Ok(None)
    }

    /// Get the daily clone traffic for a repository (Traffic view).
    ///
    /// Requires push access; 403/404 map to `Ok(None)`.
    /// Defaulted so providers that don't support traffic keep compiling.
    async fn get_traffic_clones(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Option<RemoteTraffic>, ProviderError> {
        Ok(None)
    }

    /// List the popular referrers for a repository (Traffic view).
    ///
    /// Requires push access; 403/404 map to `Ok(vec![])`.
    /// Defaulted so providers that don't support traffic keep compiling.
    async fn list_referrers(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<RemoteReferrer>, ProviderError> {
        Ok(Vec::new())
    }

    /// List the popular content paths for a repository (Traffic view).
    ///
    /// Requires push access; 403/404 map to `Ok(vec![])`.
    /// Defaulted so providers that don't support traffic keep compiling.
    async fn list_paths(
        &self,
        _owner: &str,
        _repo: &str,
    ) -> Result<Vec<RemotePath>, ProviderError> {
        Ok(Vec::new())
    }

    // --- triage write actions -------------------------------------------------
    //
    // These mutate an issue/PR identified by (owner, repo, number). Defaulted to
    // a no-op `Ok(())` so providers that don't implement writes (e.g. the test
    // `FakeProvider`) keep compiling; only `GitHubProvider` implements them.

    /// Set an issue's state ("open" or "closed").
    async fn set_issue_state(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _state: &str,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    /// Add labels to an issue (additive; does not replace existing labels).
    async fn add_issue_labels(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _labels: &[String],
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    /// Remove a single label from an issue. A 404 (label not present) is a no-op.
    async fn remove_issue_label(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _label: &str,
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    /// Add assignees to an issue.
    async fn add_issue_assignees(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _assignees: &[String],
    ) -> Result<(), ProviderError> {
        Ok(())
    }

    /// Create a comment on an issue; returns the new comment's `html_url`.
    async fn create_issue_comment(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _body: &str,
    ) -> Result<String, ProviderError> {
        Ok(String::new())
    }

    // --- in-app issue/PR detail ----------------------------------------------
    //
    // Defaulted so providers that don't support detail views keep compiling.

    /// Get full detail for a single issue. `Ok(None)` when it doesn't exist.
    async fn get_issue(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
    ) -> Result<Option<RemoteIssueDetail>, ProviderError> {
        Ok(None)
    }

    /// Get full detail for a single pull request. `Ok(None)` when it doesn't exist.
    async fn get_pull_request(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
    ) -> Result<Option<RemotePullRequestDetail>, ProviderError> {
        Ok(None)
    }

    /// List the issue-comment timeline for an issue or PR.
    async fn list_issue_comments(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _per_page: u32,
    ) -> Result<Vec<RemoteComment>, ProviderError> {
        Ok(Vec::new())
    }

    /// List the reviews on a pull request.
    async fn list_pull_reviews(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _per_page: u32,
    ) -> Result<Vec<RemoteReview>, ProviderError> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_draft_builds() {
        let d = RemoteDraft {
            title: "x".into(),
            body: "b".into(),
            labels: vec!["l".into()],
        };
        assert_eq!(d.title, "x");
    }
}
