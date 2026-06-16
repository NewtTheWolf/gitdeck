use async_trait::async_trait;
use newt_todo_core::{
    Provider, ProviderError, RemoteCodeHit, RemoteComment, RemoteContributor, RemoteDraft,
    RemoteFork, RemoteIssue, RemoteIssueDetail, RemoteLabel, RemoteLanguage, RemoteNotification,
    RemotePatch, RemotePath, RemotePullRequest, RemotePullRequestDetail, RemoteReferrer,
    RemoteRelease, RemoteRepo, RemoteRepoDetail, RemoteReview, RemoteTask, RemoteTraffic,
    RemoteTrafficDay, RemoteUser, RemoteWorkflowRun, TaskStatus,
};
use serde::Deserialize;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

pub struct GitHubProvider {
    client: reqwest::Client,
    base_url: String,
    token: String,
    owner: String,
    repo: String,
}

impl GitHubProvider {
    pub fn new(
        client: reqwest::Client,
        base_url: impl Into<String>,
        token: impl Into<String>,
        owner: impl Into<String>,
        repo: impl Into<String>,
    ) -> Self {
        Self {
            client,
            base_url: base_url.into(),
            token: token.into(),
            owner: owner.into(),
            repo: repo.into(),
        }
    }
    fn issues_url(&self) -> String {
        format!(
            "{}/repos/{}/{}/issues",
            self.base_url, self.owner, self.repo
        )
    }
    fn repos_url(&self) -> String {
        format!("{}/user/repos", self.base_url)
    }
    fn search_issues_url(&self) -> String {
        format!("{}/search/issues", self.base_url)
    }
    fn search_code_url(&self) -> String {
        format!("{}/search/code", self.base_url)
    }
    fn notifications_url(&self) -> String {
        format!("{}/notifications", self.base_url)
    }
    fn notification_thread_url(&self, thread_id: &str) -> String {
        format!("{}/notifications/threads/{}", self.base_url, thread_id)
    }
    fn workflow_runs_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/actions/runs", self.base_url, owner, repo)
    }
    fn repo_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}", self.base_url, owner, repo)
    }
    fn releases_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/releases", self.base_url, owner, repo)
    }
    fn forks_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/forks", self.base_url, owner, repo)
    }
    fn contributors_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/contributors", self.base_url, owner, repo)
    }
    fn languages_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/languages", self.base_url, owner, repo)
    }
    fn traffic_views_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/traffic/views", self.base_url, owner, repo)
    }
    fn traffic_clones_url(&self, owner: &str, repo: &str) -> String {
        format!("{}/repos/{}/{}/traffic/clones", self.base_url, owner, repo)
    }
    fn referrers_url(&self, owner: &str, repo: &str) -> String {
        format!(
            "{}/repos/{}/{}/traffic/popular/referrers",
            self.base_url, owner, repo
        )
    }
    fn paths_url(&self, owner: &str, repo: &str) -> String {
        format!(
            "{}/repos/{}/{}/traffic/popular/paths",
            self.base_url, owner, repo
        )
    }
    fn issue_url(&self, owner: &str, repo: &str, number: u64) -> String {
        format!("{}/repos/{}/{}/issues/{}", self.base_url, owner, repo, number)
    }
    fn issue_labels_url(&self, owner: &str, repo: &str, number: u64) -> String {
        format!(
            "{}/repos/{}/{}/issues/{}/labels",
            self.base_url, owner, repo, number
        )
    }
    fn issue_assignees_url(&self, owner: &str, repo: &str, number: u64) -> String {
        format!(
            "{}/repos/{}/{}/issues/{}/assignees",
            self.base_url, owner, repo, number
        )
    }
    fn issue_comments_url(&self, owner: &str, repo: &str, number: u64) -> String {
        format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, owner, repo, number
        )
    }
    fn pull_url(&self, owner: &str, repo: &str, number: u64) -> String {
        format!("{}/repos/{}/{}/pulls/{}", self.base_url, owner, repo, number)
    }
    fn pull_reviews_url(&self, owner: &str, repo: &str, number: u64) -> String {
        format!(
            "{}/repos/{}/{}/pulls/{}/reviews",
            self.base_url, owner, repo, number
        )
    }
}

/// Percent-encode a single label name for use as a URL path segment. GitHub
/// label names may contain spaces and other characters that must be escaped.
fn encode_label(label: &str) -> String {
    // Encode every byte that isn't an unreserved URL character.
    let mut out = String::with_capacity(label.len());
    for b in label.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Traffic endpoints require push access. For repos the user can't push to,
/// GitHub returns 403 (and 404 when missing). Treat both as "no data" so the
/// Traffic UI degrades gracefully instead of erroring.
fn is_no_traffic_access(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::FORBIDDEN || status == reqwest::StatusCode::NOT_FOUND
}

/// Derive `owner/name` from a GitHub `repository_url`
/// (e.g. `https://api.github.com/repos/owner/name`) by joining the last two
/// path segments. Falls back to the raw input if it can't be parsed.
fn repo_name_with_owner(repository_url: &str) -> String {
    let segments: Vec<&str> = repository_url
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();
    match segments.as_slice() {
        [.., owner, name] => format!("{owner}/{name}"),
        _ => repository_url.to_string(),
    }
}

#[derive(Deserialize)]
struct GhRepo {
    id: u64,
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    html_url: String,
    #[serde(default)]
    stargazers_count: u64,
    #[serde(default)]
    open_issues_count: u64,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    fork: bool,
    updated_at: String,
}

impl GhRepo {
    fn into_remote(self) -> Result<RemoteRepo, ProviderError> {
        Ok(RemoteRepo {
            id: self.id.to_string(),
            full_name: self.full_name,
            description: self.description,
            html_url: self.html_url,
            stars: self.stargazers_count,
            open_issues: self.open_issues_count,
            language: self.language,
            is_private: self.private,
            is_fork: self.fork,
            updated_at: OffsetDateTime::parse(&self.updated_at, &Rfc3339)
                .map_err(|e| ProviderError(format!("bad updated_at: {e}")))?,
        })
    }
}

#[derive(Deserialize)]
struct GhLabel {
    name: String,
}
#[derive(Deserialize)]
struct GhIssue {
    number: u64,
    title: String,
    #[serde(default)]
    body: Option<String>,
    state: String, // "open" | "closed"
    html_url: String,
    updated_at: String,
    #[serde(default)]
    labels: Vec<GhLabel>,
    #[serde(default)]
    pull_request: Option<serde_json::Value>, // present → it's a PR, skip
}

impl GhIssue {
    fn into_remote(self) -> Result<RemoteTask, ProviderError> {
        Ok(RemoteTask {
            remote_id: self.number.to_string(),
            title: self.title,
            body: self.body.unwrap_or_default(),
            status: if self.state == "closed" {
                TaskStatus::Done
            } else {
                TaskStatus::Open
            },
            labels: self.labels.into_iter().map(|l| l.name).collect(),
            html_url: Some(self.html_url),
            remote_updated_at: OffsetDateTime::parse(&self.updated_at, &Rfc3339)
                .map_err(|e| ProviderError(format!("bad updated_at: {e}")))?,
        })
    }
}

#[derive(Deserialize)]
struct GhSearchUser {
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GhSearchLabel {
    name: String,
    #[serde(default)]
    color: String,
}

#[derive(Deserialize)]
struct GhSearchAssignee {
    login: String,
}

#[derive(Deserialize)]
struct GhSearchItem {
    number: u64,
    title: String,
    html_url: String,
    state: String,
    #[serde(default)]
    user: Option<GhSearchUser>,
    #[serde(default)]
    comments: u64,
    #[serde(default)]
    labels: Vec<GhSearchLabel>,
    #[serde(default)]
    assignees: Vec<GhSearchAssignee>,
    created_at: String,
    updated_at: String,
    repository_url: String,
    #[serde(default)]
    draft: bool,
}

impl GhSearchItem {
    fn labels(&self) -> Vec<RemoteLabel> {
        self.labels
            .iter()
            .map(|l| RemoteLabel {
                name: l.name.clone(),
                color: l.color.clone(),
            })
            .collect()
    }

    fn assignees(&self) -> Vec<String> {
        self.assignees.iter().map(|a| a.login.clone()).collect()
    }

    fn parsed_dates(&self) -> Result<(OffsetDateTime, OffsetDateTime), ProviderError> {
        let created = OffsetDateTime::parse(&self.created_at, &Rfc3339)
            .map_err(|e| ProviderError(format!("bad created_at: {e}")))?;
        let updated = OffsetDateTime::parse(&self.updated_at, &Rfc3339)
            .map_err(|e| ProviderError(format!("bad updated_at: {e}")))?;
        Ok((created, updated))
    }

    fn into_issue(self) -> Result<RemoteIssue, ProviderError> {
        let (created_at, updated_at) = self.parsed_dates()?;
        let labels = self.labels();
        let assignees = self.assignees();
        let (author_login, author_avatar_url) = match self.user {
            Some(u) => (u.login, u.avatar_url),
            None => (String::new(), None),
        };
        Ok(RemoteIssue {
            number: self.number,
            title: self.title,
            html_url: self.html_url,
            state: self.state,
            author_login,
            author_avatar_url,
            repo_name_with_owner: repo_name_with_owner(&self.repository_url),
            created_at,
            updated_at,
            comments_count: self.comments,
            labels,
            assignees,
        })
    }

    fn into_pull_request(self) -> Result<RemotePullRequest, ProviderError> {
        let (created_at, updated_at) = self.parsed_dates()?;
        let labels = self.labels();
        let assignees = self.assignees();
        let is_draft = self.draft;
        let (author_login, author_avatar_url) = match self.user {
            Some(u) => (u.login, u.avatar_url),
            None => (String::new(), None),
        };
        Ok(RemotePullRequest {
            number: self.number,
            title: self.title,
            html_url: self.html_url,
            state: self.state,
            author_login,
            author_avatar_url,
            repo_name_with_owner: repo_name_with_owner(&self.repository_url),
            created_at,
            updated_at,
            comments_count: self.comments,
            labels,
            assignees,
            is_draft,
        })
    }
}

#[derive(Deserialize)]
struct GhSearchResponse {
    #[serde(default)]
    items: Vec<GhSearchItem>,
}

#[derive(Deserialize)]
struct GhCodeRepository {
    #[serde(default)]
    full_name: String,
}

#[derive(Deserialize)]
struct GhCodeSearchItem {
    name: String,
    path: String,
    html_url: String,
    repository: GhCodeRepository,
}

impl GhCodeSearchItem {
    fn into_remote(self) -> RemoteCodeHit {
        RemoteCodeHit {
            repo_name_with_owner: self.repository.full_name,
            path: self.path,
            html_url: self.html_url,
            name: self.name,
        }
    }
}

#[derive(Deserialize)]
struct GhCodeSearchResponse {
    #[serde(default)]
    items: Vec<GhCodeSearchItem>,
}

#[derive(Deserialize)]
struct GhNotificationRepo {
    #[serde(default)]
    full_name: String,
}

#[derive(Deserialize)]
struct GhNotificationSubject {
    #[serde(default)]
    title: String,
    #[serde(rename = "type", default)]
    subject_type: String,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Deserialize)]
struct GhNotification {
    id: String,
    #[serde(default)]
    repository: Option<GhNotificationRepo>,
    subject: GhNotificationSubject,
    reason: String,
    #[serde(default)]
    unread: bool,
    updated_at: String,
}

impl GhNotification {
    fn into_remote(self) -> RemoteNotification {
        RemoteNotification {
            id: self.id,
            repo_name_with_owner: self.repository.map(|r| r.full_name).unwrap_or_default(),
            subject_title: self.subject.title,
            subject_type: self.subject.subject_type,
            subject_url: self.subject.url,
            reason: self.reason,
            unread: self.unread,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Deserialize)]
struct GhWorkflowRun {
    id: u64,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    head_branch: Option<String>,
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    html_url: String,
    created_at: String,
    updated_at: String,
    #[serde(default)]
    event: Option<String>,
}

impl GhWorkflowRun {
    fn into_remote(self) -> RemoteWorkflowRun {
        RemoteWorkflowRun {
            id: self.id,
            name: self.name,
            head_branch: self.head_branch,
            status: self.status,
            conclusion: self.conclusion,
            html_url: self.html_url,
            created_at: self.created_at,
            updated_at: self.updated_at,
            event: self.event,
        }
    }
}

#[derive(Deserialize)]
struct GhWorkflowRunsResponse {
    #[serde(default)]
    workflow_runs: Vec<GhWorkflowRun>,
}

#[derive(Deserialize)]
struct GhOwner {
    #[serde(default)]
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GhLicense {
    #[serde(default)]
    spdx_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct GhRepoDetail {
    full_name: String,
    #[serde(default)]
    description: Option<String>,
    html_url: String,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    stargazers_count: u64,
    #[serde(default)]
    forks_count: u64,
    #[serde(default)]
    open_issues_count: u64,
    #[serde(default)]
    watchers_count: u64,
    #[serde(default)]
    default_branch: String,
    #[serde(default)]
    license: Option<GhLicense>,
    #[serde(default)]
    topics: Vec<String>,
    #[serde(default)]
    owner: Option<GhOwner>,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    fork: bool,
    #[serde(default)]
    archived: bool,
    #[serde(default)]
    size: u64,
    #[serde(default)]
    pushed_at: Option<String>,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    updated_at: Option<String>,
}

impl GhRepoDetail {
    fn into_remote(self) -> RemoteRepoDetail {
        let license = self.license.and_then(|l| {
            // Prefer the SPDX id (e.g. "MIT"); GitHub uses "NOASSERTION" for
            // licenses it couldn't identify, so fall back to the human name.
            match l.spdx_id {
                Some(id) if !id.is_empty() && id != "NOASSERTION" => Some(id),
                _ => l.name,
            }
        });
        let (owner_login, owner_avatar_url) = match self.owner {
            Some(o) => (o.login, o.avatar_url),
            None => (String::new(), None),
        };
        RemoteRepoDetail {
            full_name: self.full_name,
            description: self.description,
            html_url: self.html_url,
            homepage: self.homepage.filter(|h| !h.is_empty()),
            language: self.language,
            stars: self.stargazers_count,
            forks: self.forks_count,
            open_issues: self.open_issues_count,
            watchers: self.watchers_count,
            default_branch: self.default_branch,
            license,
            topics: self.topics,
            owner_login,
            owner_avatar_url,
            is_private: self.private,
            is_fork: self.fork,
            is_archived: self.archived,
            size: self.size,
            pushed_at: self.pushed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Deserialize)]
struct GhReleaseAuthor {
    #[serde(default)]
    login: String,
}

#[derive(Deserialize)]
struct GhRelease {
    id: u64,
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    author: Option<GhReleaseAuthor>,
}

impl GhRelease {
    fn into_remote(self) -> RemoteRelease {
        RemoteRelease {
            id: self.id,
            tag_name: self.tag_name,
            name: self.name,
            html_url: self.html_url,
            body: self.body,
            draft: self.draft,
            prerelease: self.prerelease,
            published_at: self.published_at,
            author_login: self.author.map(|a| a.login),
        }
    }
}

#[derive(Deserialize)]
struct GhFork {
    full_name: String,
    html_url: String,
    #[serde(default)]
    stargazers_count: u64,
    #[serde(default)]
    pushed_at: Option<String>,
    #[serde(default)]
    owner: Option<GhOwner>,
}

impl GhFork {
    fn into_remote(self) -> RemoteFork {
        RemoteFork {
            full_name: self.full_name,
            html_url: self.html_url,
            stars: self.stargazers_count,
            pushed_at: self.pushed_at,
            owner_login: self.owner.map(|o| o.login).unwrap_or_default(),
        }
    }
}

#[derive(Deserialize)]
struct GhContributor {
    #[serde(default)]
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    contributions: u64,
}

impl GhContributor {
    fn into_remote(self) -> RemoteContributor {
        RemoteContributor {
            login: self.login,
            avatar_url: self.avatar_url,
            html_url: self.html_url,
            contributions: self.contributions,
        }
    }
}

#[derive(Deserialize)]
struct GhTrafficDay {
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    count: u64,
    #[serde(default)]
    uniques: u64,
}

impl GhTrafficDay {
    fn into_remote(self) -> RemoteTrafficDay {
        RemoteTrafficDay {
            timestamp: self.timestamp,
            count: self.count,
            uniques: self.uniques,
        }
    }
}

#[derive(Deserialize)]
struct GhTrafficViews {
    #[serde(default)]
    count: u64,
    #[serde(default)]
    uniques: u64,
    #[serde(default)]
    views: Vec<GhTrafficDay>,
}

impl GhTrafficViews {
    fn into_remote(self) -> RemoteTraffic {
        RemoteTraffic {
            count: self.count,
            uniques: self.uniques,
            days: self.views.into_iter().map(GhTrafficDay::into_remote).collect(),
        }
    }
}

#[derive(Deserialize)]
struct GhTrafficClones {
    #[serde(default)]
    count: u64,
    #[serde(default)]
    uniques: u64,
    #[serde(default)]
    clones: Vec<GhTrafficDay>,
}

impl GhTrafficClones {
    fn into_remote(self) -> RemoteTraffic {
        RemoteTraffic {
            count: self.count,
            uniques: self.uniques,
            days: self.clones.into_iter().map(GhTrafficDay::into_remote).collect(),
        }
    }
}

#[derive(Deserialize)]
struct GhReferrer {
    #[serde(default)]
    referrer: String,
    #[serde(default)]
    count: u64,
    #[serde(default)]
    uniques: u64,
}

impl GhReferrer {
    fn into_remote(self) -> RemoteReferrer {
        RemoteReferrer {
            referrer: self.referrer,
            count: self.count,
            uniques: self.uniques,
        }
    }
}

#[derive(Deserialize)]
struct GhPath {
    #[serde(default)]
    path: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    count: u64,
    #[serde(default)]
    uniques: u64,
}

/// The subset of a created issue comment we care about (its web URL).
#[derive(Deserialize)]
struct GhComment {
    #[serde(default)]
    html_url: String,
}

impl GhPath {
    fn into_remote(self) -> RemotePath {
        RemotePath {
            path: self.path,
            title: self.title,
            count: self.count,
            uniques: self.uniques,
        }
    }
}

// --- in-app issue/PR detail -------------------------------------------------

#[derive(Deserialize)]
struct GhUser {
    #[serde(default)]
    login: String,
    #[serde(default)]
    avatar_url: Option<String>,
}

impl GhUser {
    fn into_remote(self) -> RemoteUser {
        RemoteUser {
            login: self.login,
            avatar_url: self.avatar_url,
        }
    }
}

#[derive(Deserialize)]
struct GhDetailLabel {
    #[serde(default)]
    name: String,
    #[serde(default)]
    color: String,
}

#[derive(Deserialize)]
struct GhMilestone {
    #[serde(default)]
    title: Option<String>,
}

/// The shared issue-ish fields present on both the issues and pulls endpoints.
#[derive(Deserialize)]
struct GhIssueDetail {
    number: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    state: String,
    #[serde(default)]
    user: Option<GhUser>,
    #[serde(default)]
    repository_url: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    closed_at: Option<String>,
    #[serde(default)]
    comments: u64,
    #[serde(default)]
    labels: Vec<GhDetailLabel>,
    #[serde(default)]
    assignees: Vec<GhUser>,
    #[serde(default)]
    milestone: Option<GhMilestone>,
}

fn detail_labels(labels: Vec<GhDetailLabel>) -> Vec<RemoteLabel> {
    labels
        .into_iter()
        .map(|l| RemoteLabel {
            name: l.name,
            color: l.color,
        })
        .collect()
}

impl GhIssueDetail {
    fn into_remote(self, owner: &str, repo: &str) -> RemoteIssueDetail {
        let (author_login, author_avatar_url) = match self.user {
            Some(u) => (u.login, u.avatar_url),
            None => (String::new(), None),
        };
        let repo_name_with_owner = if self.repository_url.is_empty() {
            format!("{owner}/{repo}")
        } else {
            repo_name_with_owner(&self.repository_url)
        };
        RemoteIssueDetail {
            number: self.number,
            title: self.title,
            body: self.body,
            html_url: self.html_url,
            state: self.state,
            author_login,
            author_avatar_url,
            repo_name_with_owner,
            created_at: self.created_at,
            updated_at: self.updated_at,
            closed_at: self.closed_at,
            comments_count: self.comments,
            labels: detail_labels(self.labels),
            assignees: self.assignees.into_iter().map(GhUser::into_remote).collect(),
            milestone_title: self.milestone.and_then(|m| m.title),
        }
    }
}

#[derive(Deserialize)]
struct GhRef {
    #[serde(default, rename = "ref")]
    ref_name: String,
}

#[derive(Deserialize)]
struct GhTeam {
    #[serde(default)]
    slug: String,
}

/// The pulls endpoint shares the issue-ish fields plus PR-specific ones.
#[derive(Deserialize)]
struct GhPullDetail {
    #[serde(flatten)]
    issue: GhIssueDetail,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    merged: bool,
    #[serde(default)]
    mergeable_state: Option<String>,
    #[serde(default)]
    base: Option<GhRef>,
    #[serde(default)]
    head: Option<GhRef>,
    #[serde(default)]
    additions: Option<u64>,
    #[serde(default)]
    deletions: Option<u64>,
    #[serde(default)]
    changed_files: Option<u64>,
    #[serde(default)]
    commits: Option<u64>,
    #[serde(default)]
    requested_reviewers: Vec<GhUser>,
    #[serde(default)]
    requested_teams: Vec<GhTeam>,
}

impl GhPullDetail {
    fn into_remote(self, owner: &str, repo: &str) -> RemotePullRequestDetail {
        let base_ref = self.base.map(|r| r.ref_name).unwrap_or_default();
        let head_ref = self.head.map(|r| r.ref_name).unwrap_or_default();
        let issue = self.issue.into_remote(owner, repo);
        RemotePullRequestDetail {
            number: issue.number,
            title: issue.title,
            body: issue.body,
            html_url: issue.html_url,
            state: issue.state,
            author_login: issue.author_login,
            author_avatar_url: issue.author_avatar_url,
            repo_name_with_owner: issue.repo_name_with_owner,
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            comments_count: issue.comments_count,
            labels: issue.labels,
            assignees: issue.assignees,
            milestone_title: issue.milestone_title,
            is_draft: self.draft,
            merged: self.merged,
            mergeable_state: self.mergeable_state,
            base_ref,
            head_ref,
            additions: self.additions,
            deletions: self.deletions,
            changed_files: self.changed_files,
            commits: self.commits,
            requested_reviewers: self
                .requested_reviewers
                .into_iter()
                .map(GhUser::into_remote)
                .collect(),
            requested_teams: self.requested_teams.into_iter().map(|t| t.slug).collect(),
        }
    }
}

#[derive(Deserialize)]
struct GhIssueComment {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    user: Option<GhUser>,
    #[serde(default)]
    body: String,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    html_url: String,
}

impl GhIssueComment {
    fn into_remote(self) -> RemoteComment {
        let (author_login, author_avatar_url) = match self.user {
            Some(u) => (u.login, u.avatar_url),
            None => (String::new(), None),
        };
        RemoteComment {
            id: self.id,
            author_login,
            author_avatar_url,
            body: self.body,
            created_at: self.created_at,
            updated_at: self.updated_at,
            html_url: self.html_url,
        }
    }
}

#[derive(Deserialize)]
struct GhReview {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    user: Option<GhUser>,
    #[serde(default)]
    state: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    submitted_at: Option<String>,
    #[serde(default)]
    html_url: String,
}

impl GhReview {
    fn into_remote(self) -> RemoteReview {
        let (reviewer_login, reviewer_avatar_url) = match self.user {
            Some(u) => (u.login, u.avatar_url),
            None => (String::new(), None),
        };
        // GitHub returns an empty-string body for reviews without a comment;
        // normalize that to None.
        let body = self.body.filter(|b| !b.is_empty());
        RemoteReview {
            id: self.id,
            reviewer_login,
            reviewer_avatar_url,
            state: self.state,
            body,
            submitted_at: self.submitted_at,
            html_url: self.html_url,
        }
    }
}

#[async_trait]
impl Provider for GitHubProvider {
    async fn list_tasks(&self) -> Result<Vec<RemoteTask>, ProviderError> {
        let resp = self
            .client
            .get(self.issues_url())
            .query(&[("state", "all"), ("per_page", "100")])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let issues: Vec<GhIssue> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        issues
            .into_iter()
            .filter(|i| i.pull_request.is_none()) // exclude PRs
            .map(GhIssue::into_remote)
            .collect()
    }

    async fn create_task(&self, draft: RemoteDraft) -> Result<RemoteTask, ProviderError> {
        let resp = self
            .client
            .post(self.issues_url())
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::json!({ "title": draft.title, "body": draft.body, "labels": draft.labels }))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let issue: GhIssue = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        issue.into_remote()
    }

    async fn update_task(
        &self,
        remote_id: &str,
        patch: RemotePatch,
    ) -> Result<RemoteTask, ProviderError> {
        let mut body = serde_json::Map::new();
        if let Some(t) = patch.title {
            body.insert("title".into(), t.into());
        }
        if let Some(b) = patch.body {
            body.insert("body".into(), b.into());
        }
        if let Some(s) = patch.status {
            body.insert(
                "state".into(),
                (if s == TaskStatus::Done {
                    "closed"
                } else {
                    "open"
                })
                .into(),
            );
        }
        if let Some(l) = patch.labels {
            body.insert("labels".into(), serde_json::json!(l));
        }
        let url = format!("{}/{}", self.issues_url(), remote_id);
        let resp = self
            .client
            .patch(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::Value::Object(body))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let issue: GhIssue = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        issue.into_remote()
    }

    async fn list_repos(&self) -> Result<Vec<RemoteRepo>, ProviderError> {
        let resp = self
            .client
            .get(self.repos_url())
            .query(&[
                ("per_page", "100"),
                ("sort", "updated"),
                ("affiliation", "owner,collaborator,organization_member"),
            ])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let repos: Vec<GhRepo> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        repos.into_iter().map(GhRepo::into_remote).collect()
    }

    async fn list_issues(&self) -> Result<Vec<RemoteIssue>, ProviderError> {
        let resp = self
            .client
            .get(self.search_issues_url())
            .query(&[
                ("q", "is:issue involves:@me"),
                ("sort", "updated"),
                ("per_page", "50"),
            ])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let search: GhSearchResponse = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        search
            .items
            .into_iter()
            .map(GhSearchItem::into_issue)
            .collect()
    }

    async fn list_pull_requests(&self) -> Result<Vec<RemotePullRequest>, ProviderError> {
        let resp = self
            .client
            .get(self.search_issues_url())
            .query(&[
                ("q", "is:pr involves:@me"),
                ("sort", "updated"),
                ("per_page", "50"),
            ])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let search: GhSearchResponse = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        search
            .items
            .into_iter()
            .map(GhSearchItem::into_pull_request)
            .collect()
    }

    async fn search_code(
        &self,
        query: &str,
        per_page: u32,
    ) -> Result<Vec<RemoteCodeHit>, ProviderError> {
        let per_page = per_page.clamp(1, 30).to_string();
        let resp = self
            .client
            .get(self.search_code_url())
            .query(&[("q", query), ("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let search: GhCodeSearchResponse = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(search
            .items
            .into_iter()
            .map(GhCodeSearchItem::into_remote)
            .collect())
    }

    async fn search_issues(
        &self,
        query: &str,
        per_page: u32,
    ) -> Result<Vec<RemoteIssue>, ProviderError> {
        let per_page = per_page.clamp(1, 30).to_string();
        let resp = self
            .client
            .get(self.search_issues_url())
            .query(&[("q", query), ("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let search: GhSearchResponse = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        // Reuse the exact issue mapping `list_issues` uses.
        search
            .items
            .into_iter()
            .map(GhSearchItem::into_issue)
            .collect()
    }

    async fn list_notifications(&self) -> Result<Vec<RemoteNotification>, ProviderError> {
        let resp = self
            .client
            .get(self.notifications_url())
            .query(&[("all", "true"), ("per_page", "50")])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let notifications: Vec<GhNotification> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(notifications
            .into_iter()
            .map(GhNotification::into_remote)
            .collect())
    }

    async fn mark_notification_read(&self, thread_id: &str) -> Result<(), ProviderError> {
        self.client
            .patch(self.notification_thread_url(thread_id))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(())
    }

    async fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> Result<Vec<RemoteWorkflowRun>, ProviderError> {
        let per_page = per_page.clamp(1, 20).to_string();
        let resp = self
            .client
            .get(self.workflow_runs_url(owner, repo))
            .query(&[("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let runs: GhWorkflowRunsResponse = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(runs
            .workflow_runs
            .into_iter()
            .map(GhWorkflowRun::into_remote)
            .collect())
    }

    async fn get_repo_detail(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<RemoteRepoDetail>, ProviderError> {
        let resp = self
            .client
            .get(self.repo_url(owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let detail: GhRepoDetail = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(Some(detail.into_remote()))
    }

    async fn list_releases(
        &self,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> Result<Vec<RemoteRelease>, ProviderError> {
        let per_page = per_page.clamp(1, 30).to_string();
        let resp = self
            .client
            .get(self.releases_url(owner, repo))
            .query(&[("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let releases: Vec<GhRelease> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(releases.into_iter().map(GhRelease::into_remote).collect())
    }

    async fn list_forks(
        &self,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> Result<Vec<RemoteFork>, ProviderError> {
        let per_page = per_page.clamp(1, 30).to_string();
        let resp = self
            .client
            .get(self.forks_url(owner, repo))
            .query(&[("sort", "stargazers"), ("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let forks: Vec<GhFork> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(forks.into_iter().map(GhFork::into_remote).collect())
    }

    async fn list_contributors(
        &self,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> Result<Vec<RemoteContributor>, ProviderError> {
        let per_page = per_page.clamp(1, 30).to_string();
        let resp = self
            .client
            .get(self.contributors_url(owner, repo))
            .query(&[("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let contributors: Vec<GhContributor> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(contributors
            .into_iter()
            .map(GhContributor::into_remote)
            .collect())
    }

    async fn get_languages(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<RemoteLanguage>, ProviderError> {
        let resp = self
            .client
            .get(self.languages_url(owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        // The languages endpoint returns a JSON object {name: bytes}.
        let map: std::collections::HashMap<String, u64> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let mut langs: Vec<RemoteLanguage> = map
            .into_iter()
            .map(|(name, bytes)| RemoteLanguage { name, bytes })
            .collect();
        // Sort by bytes desc; tie-break on name for stable ordering.
        langs.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));
        Ok(langs)
    }

    async fn get_traffic_views(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<RemoteTraffic>, ProviderError> {
        let resp = self
            .client
            .get(self.traffic_views_url(owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if is_no_traffic_access(resp.status()) {
            return Ok(None);
        }
        let views: GhTrafficViews = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(Some(views.into_remote()))
    }

    async fn get_traffic_clones(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Option<RemoteTraffic>, ProviderError> {
        let resp = self
            .client
            .get(self.traffic_clones_url(owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if is_no_traffic_access(resp.status()) {
            return Ok(None);
        }
        let clones: GhTrafficClones = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(Some(clones.into_remote()))
    }

    async fn list_referrers(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<RemoteReferrer>, ProviderError> {
        let resp = self
            .client
            .get(self.referrers_url(owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if is_no_traffic_access(resp.status()) {
            return Ok(Vec::new());
        }
        let referrers: Vec<GhReferrer> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(referrers.into_iter().map(GhReferrer::into_remote).collect())
    }

    async fn list_paths(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<RemotePath>, ProviderError> {
        let resp = self
            .client
            .get(self.paths_url(owner, repo))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if is_no_traffic_access(resp.status()) {
            return Ok(Vec::new());
        }
        let paths: Vec<GhPath> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(paths.into_iter().map(GhPath::into_remote).collect())
    }

    // --- triage write actions -------------------------------------------------

    async fn set_issue_state(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        state: &str,
    ) -> Result<(), ProviderError> {
        self.client
            .patch(self.issue_url(owner, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::json!({ "state": state }))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(())
    }

    async fn add_issue_labels(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        labels: &[String],
    ) -> Result<(), ProviderError> {
        self.client
            .post(self.issue_labels_url(owner, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::json!({ "labels": labels }))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(())
    }

    async fn remove_issue_label(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        label: &str,
    ) -> Result<(), ProviderError> {
        let url = format!(
            "{}/{}",
            self.issue_labels_url(owner, repo, number),
            encode_label(label)
        );
        let resp = self
            .client
            .delete(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        // A 404 means the label wasn't on the issue — treat removing it as a no-op.
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        resp.error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(())
    }

    async fn add_issue_assignees(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        assignees: &[String],
    ) -> Result<(), ProviderError> {
        self.client
            .post(self.issue_assignees_url(owner, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::json!({ "assignees": assignees }))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(())
    }

    async fn create_issue_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        body: &str,
    ) -> Result<String, ProviderError> {
        let comment: GhComment = self
            .client
            .post(self.issue_comments_url(owner, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::json!({ "body": body }))
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(comment.html_url)
    }

    // --- in-app issue/PR detail ----------------------------------------------

    async fn get_issue(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<Option<RemoteIssueDetail>, ProviderError> {
        let resp = self
            .client
            .get(self.issue_url(owner, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let detail: GhIssueDetail = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(Some(detail.into_remote(owner, repo)))
    }

    async fn get_pull_request(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> Result<Option<RemotePullRequestDetail>, ProviderError> {
        let resp = self
            .client
            .get(self.pull_url(owner, repo, number))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let detail: GhPullDetail = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(Some(detail.into_remote(owner, repo)))
    }

    async fn list_issue_comments(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        per_page: u32,
    ) -> Result<Vec<RemoteComment>, ProviderError> {
        let per_page = per_page.clamp(1, 100).to_string();
        let resp = self
            .client
            .get(self.issue_comments_url(owner, repo, number))
            .query(&[("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let comments: Vec<GhIssueComment> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(comments
            .into_iter()
            .map(GhIssueComment::into_remote)
            .collect())
    }

    async fn list_pull_reviews(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        per_page: u32,
    ) -> Result<Vec<RemoteReview>, ProviderError> {
        let per_page = per_page.clamp(1, 100).to_string();
        let resp = self
            .client
            .get(self.pull_reviews_url(owner, repo, number))
            .query(&[("per_page", per_page.as_str())])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        let reviews: Vec<GhReview> = resp
            .error_for_status()
            .map_err(|e| ProviderError(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError(e.to_string()))?;
        Ok(reviews.into_iter().map(GhReview::into_remote).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_partial_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn provider(uri: String) -> GitHubProvider {
        GitHubProvider::new(reqwest::Client::new(), uri, "t", "o", "r")
    }

    #[tokio::test]
    async fn list_tasks_filters_pull_requests() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "number": 1,
                "title": "hi",
                "body": "b",
                "state": "open",
                "html_url": "https://github.com/o/r/issues/1",
                "updated_at": "2026-06-15T10:00:00Z",
                "labels": [{ "name": "bug" }]
            },
            {
                "number": 2,
                "title": "a pr",
                "body": "pr body",
                "state": "open",
                "html_url": "https://github.com/o/r/pull/2",
                "updated_at": "2026-06-15T10:00:00Z",
                "labels": [],
                "pull_request": { "url": "https://github.com/o/r/pulls/2" }
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let tasks = p.list_tasks().await.unwrap();

        assert_eq!(tasks.len(), 1, "PR must be filtered out");
        assert_eq!(tasks[0].remote_id, "1");
        assert_eq!(tasks[0].status, TaskStatus::Open);
        assert_eq!(tasks[0].labels, vec!["bug".to_string()]);
    }

    #[tokio::test]
    async fn update_task_maps_closed_to_done() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "number": 1,
            "title": "hi",
            "body": "b",
            "state": "closed",
            "html_url": "https://github.com/o/r/issues/1",
            "updated_at": "2026-06-15T11:00:00Z",
            "labels": []
        });
        Mock::given(method("PATCH"))
            .and(path("/repos/o/r/issues/1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let task = p
            .update_task(
                "1",
                RemotePatch {
                    status: Some(TaskStatus::Done),
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_eq!(task.remote_id, "1");
        assert_eq!(task.status, TaskStatus::Done);
    }

    #[tokio::test]
    async fn create_task_posts_and_maps() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "number": 7,
            "title": "new",
            "body": "made",
            "state": "open",
            "html_url": "https://github.com/o/r/issues/7",
            "updated_at": "2026-06-15T12:00:00Z",
            "labels": [{ "name": "feature" }]
        });
        Mock::given(method("POST"))
            .and(path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(201).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let task = p
            .create_task(newt_todo_core::RemoteDraft {
                title: "new".into(),
                body: "made".into(),
                labels: vec!["feature".into()],
            })
            .await
            .unwrap();

        assert_eq!(task.remote_id, "7");
        assert_eq!(task.title, "new");
        assert_eq!(task.labels, vec!["feature".to_string()]);
    }

    #[tokio::test]
    async fn list_repos_maps_github_repos() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "id": 100,
                "full_name": "o/alpha",
                "description": "the alpha repo",
                "html_url": "https://github.com/o/alpha",
                "stargazers_count": 42,
                "open_issues_count": 3,
                "language": "Rust",
                "private": false,
                "fork": false,
                "updated_at": "2026-06-15T10:00:00Z"
            },
            {
                "id": 200,
                "full_name": "o/beta-fork",
                "description": null,
                "html_url": "https://github.com/o/beta-fork",
                "stargazers_count": 1,
                "open_issues_count": 0,
                "language": null,
                "private": true,
                "fork": true,
                "updated_at": "2026-06-14T10:00:00Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let repos = p.list_repos().await.unwrap();

        assert_eq!(repos.len(), 2);

        assert_eq!(repos[0].full_name, "o/alpha");
        assert_eq!(repos[0].stars, 42);
        assert_eq!(repos[0].open_issues, 3);
        assert_eq!(repos[0].language.as_deref(), Some("Rust"));
        assert!(!repos[0].is_fork);
        assert!(!repos[0].is_private);

        assert_eq!(repos[1].full_name, "o/beta-fork");
        assert_eq!(repos[1].stars, 1);
        assert!(repos[1].is_fork);
        assert!(repos[1].is_private);
        assert_eq!(repos[1].language, None);
        assert_eq!(repos[1].description, None);
    }

    #[test]
    fn repo_name_with_owner_derives_last_two_segments() {
        assert_eq!(
            repo_name_with_owner("https://api.github.com/repos/o/r"),
            "o/r"
        );
        assert_eq!(
            repo_name_with_owner("https://api.github.com/repos/octo-org/some-repo/"),
            "octo-org/some-repo"
        );
    }

    #[tokio::test]
    async fn list_issues_maps_search_items() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "items": [{
                "number": 42,
                "title": "a bug",
                "html_url": "https://github.com/o/r/issues/42",
                "state": "open",
                "user": { "login": "alice", "avatar_url": "https://avatars/alice.png" },
                "comments": 3,
                "labels": [{ "name": "bug", "color": "d73a4a" }],
                "assignees": [{ "login": "bob" }],
                "created_at": "2026-06-10T10:00:00Z",
                "updated_at": "2026-06-15T10:00:00Z",
                "repository_url": "https://api.github.com/repos/o/r"
            }]
        });
        Mock::given(method("GET"))
            .and(path("/search/issues"))
            .and(query_param("q", "is:issue involves:@me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let issues = p.list_issues().await.unwrap();

        assert_eq!(issues.len(), 1);
        let i = &issues[0];
        assert_eq!(i.number, 42);
        assert_eq!(i.repo_name_with_owner, "o/r");
        assert_eq!(i.state, "open");
        assert_eq!(i.author_login, "alice");
        assert_eq!(
            i.author_avatar_url.as_deref(),
            Some("https://avatars/alice.png")
        );
        assert_eq!(i.comments_count, 3);
        assert_eq!(i.labels.len(), 1);
        assert_eq!(i.labels[0].name, "bug");
        assert_eq!(i.labels[0].color, "d73a4a");
        assert_eq!(i.assignees, vec!["bob".to_string()]);
    }

    #[tokio::test]
    async fn list_pull_requests_maps_draft_and_repo() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "items": [{
                "number": 7,
                "title": "a draft pr",
                "html_url": "https://github.com/o/r/pull/7",
                "state": "open",
                "user": { "login": "carol", "avatar_url": null },
                "comments": 1,
                "labels": [],
                "assignees": [],
                "created_at": "2026-06-12T10:00:00Z",
                "updated_at": "2026-06-15T11:00:00Z",
                "repository_url": "https://api.github.com/repos/octo/widgets",
                "draft": true,
                "pull_request": { "url": "https://api.github.com/repos/octo/widgets/pulls/7" }
            }]
        });
        Mock::given(method("GET"))
            .and(path("/search/issues"))
            .and(query_param("q", "is:pr involves:@me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let prs = p.list_pull_requests().await.unwrap();

        assert_eq!(prs.len(), 1);
        let pr = &prs[0];
        assert_eq!(pr.number, 7);
        assert!(pr.is_draft);
        assert_eq!(pr.repo_name_with_owner, "octo/widgets");
        assert_eq!(pr.author_login, "carol");
        assert_eq!(pr.author_avatar_url, None);
        assert_eq!(pr.comments_count, 1);
    }

    #[tokio::test]
    async fn search_code_forwards_query_and_maps_items() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "items": [{
                "name": "github.rs",
                "path": "crates/providers/src/github.rs",
                "html_url": "https://github.com/o/r/blob/main/crates/providers/src/github.rs",
                "repository": { "full_name": "o/r" }
            }]
        });
        Mock::given(method("GET"))
            .and(path("/search/code"))
            .and(query_param("q", "search_code in:file"))
            .and(query_param("per_page", "15"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let hits = p.search_code("search_code in:file", 15).await.unwrap();

        assert_eq!(hits.len(), 1);
        let h = &hits[0];
        assert_eq!(h.repo_name_with_owner, "o/r");
        assert_eq!(h.path, "crates/providers/src/github.rs");
        assert_eq!(h.name, "github.rs");
        assert_eq!(
            h.html_url,
            "https://github.com/o/r/blob/main/crates/providers/src/github.rs"
        );
    }

    #[tokio::test]
    async fn search_code_clamps_per_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/search/code"))
            .and(query_param("per_page", "30"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({ "items": [] })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let hits = p.search_code("anything", 999).await.unwrap();
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn search_issues_forwards_query_and_reuses_issue_mapping() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "items": [{
                "number": 99,
                "title": "found via search",
                "html_url": "https://github.com/octo/widgets/issues/99",
                "state": "open",
                "user": { "login": "dave", "avatar_url": "https://avatars/dave.png" },
                "comments": 2,
                "labels": [{ "name": "enhancement", "color": "a2eeef" }],
                "assignees": [{ "login": "erin" }],
                "created_at": "2026-06-10T10:00:00Z",
                "updated_at": "2026-06-15T10:00:00Z",
                "repository_url": "https://api.github.com/repos/octo/widgets"
            }]
        });
        Mock::given(method("GET"))
            .and(path("/search/issues"))
            .and(query_param("q", "mentions:@me repo:octo/widgets"))
            .and(query_param("per_page", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let issues = p
            .search_issues("mentions:@me repo:octo/widgets", 20)
            .await
            .unwrap();

        assert_eq!(issues.len(), 1);
        let i = &issues[0];
        assert_eq!(i.number, 99);
        // repo_name_with_owner derived from repository_url via the shared mapping.
        assert_eq!(i.repo_name_with_owner, "octo/widgets");
        assert_eq!(i.author_login, "dave");
        assert_eq!(i.labels[0].name, "enhancement");
        assert_eq!(i.assignees, vec!["erin".to_string()]);
    }

    #[tokio::test]
    async fn list_notifications_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "id": "1234",
                "unread": true,
                "reason": "mention",
                "updated_at": "2026-06-15T10:00:00Z",
                "subject": {
                    "title": "an issue needs you",
                    "type": "Issue",
                    "url": "https://api.github.com/repos/o/r/issues/42"
                },
                "repository": { "full_name": "o/r" }
            },
            {
                "id": "5678",
                "unread": false,
                "reason": "review_requested",
                "updated_at": "2026-06-14T10:00:00Z",
                "subject": {
                    "title": "a pr to review",
                    "type": "PullRequest",
                    "url": null
                },
                "repository": { "full_name": "octo/widgets" }
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/notifications"))
            .and(query_param("all", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let notifications = p.list_notifications().await.unwrap();

        assert_eq!(notifications.len(), 2);

        let n = &notifications[0];
        assert_eq!(n.id, "1234");
        assert!(n.unread);
        assert_eq!(n.reason, "mention");
        assert_eq!(n.repo_name_with_owner, "o/r");
        assert_eq!(n.subject_title, "an issue needs you");
        assert_eq!(n.subject_type, "Issue");
        assert_eq!(
            n.subject_url.as_deref(),
            Some("https://api.github.com/repos/o/r/issues/42")
        );

        let n2 = &notifications[1];
        assert_eq!(n2.id, "5678");
        assert!(!n2.unread);
        assert_eq!(n2.subject_type, "PullRequest");
        assert_eq!(n2.subject_url, None);
        assert_eq!(n2.repo_name_with_owner, "octo/widgets");
    }

    #[tokio::test]
    async fn mark_notification_read_patches_thread() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/notifications/threads/1234"))
            .respond_with(ResponseTemplate::new(205))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        p.mark_notification_read("1234").await.unwrap();
    }

    #[tokio::test]
    async fn list_workflow_runs_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "total_count": 2,
            "workflow_runs": [
                {
                    "id": 111,
                    "name": "CI",
                    "head_branch": "main",
                    "status": "completed",
                    "conclusion": "success",
                    "html_url": "https://github.com/o/r/actions/runs/111",
                    "created_at": "2026-06-15T10:00:00Z",
                    "updated_at": "2026-06-15T10:05:00Z",
                    "event": "push"
                },
                {
                    "id": 222,
                    "name": "CI",
                    "head_branch": "feature",
                    "status": "in_progress",
                    "conclusion": null,
                    "html_url": "https://github.com/o/r/actions/runs/222",
                    "created_at": "2026-06-15T11:00:00Z",
                    "updated_at": "2026-06-15T11:01:00Z",
                    "event": "pull_request"
                }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/r/actions/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let runs = p.list_workflow_runs("o", "r", 20).await.unwrap();

        assert_eq!(runs.len(), 2);

        let r0 = &runs[0];
        assert_eq!(r0.id, 111);
        assert_eq!(r0.name.as_deref(), Some("CI"));
        assert_eq!(r0.head_branch.as_deref(), Some("main"));
        assert_eq!(r0.status, "completed");
        assert_eq!(r0.conclusion.as_deref(), Some("success"));
        assert_eq!(r0.event.as_deref(), Some("push"));

        let r1 = &runs[1];
        assert_eq!(r1.id, 222);
        assert_eq!(r1.status, "in_progress");
        assert_eq!(r1.conclusion, None, "in_progress run has null conclusion");
    }

    #[tokio::test]
    async fn get_repo_detail_maps_payload_null_safe() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "full_name": "o/alpha",
            "description": "the alpha repo",
            "html_url": "https://github.com/o/alpha",
            "homepage": null,
            "language": "Rust",
            "stargazers_count": 42,
            "forks_count": 7,
            "open_issues_count": 3,
            "watchers_count": 42,
            "default_branch": "main",
            "license": null,
            "topics": ["rust", "cli"],
            "owner": { "login": "o", "avatar_url": "https://avatars/o.png" },
            "private": false,
            "fork": false,
            "archived": false,
            "size": 1234,
            "pushed_at": "2026-06-15T10:00:00Z",
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2026-06-16T10:00:00Z"
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/alpha"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let detail = p.get_repo_detail("o", "alpha").await.unwrap().unwrap();

        assert_eq!(detail.full_name, "o/alpha");
        assert_eq!(detail.stars, 42);
        assert_eq!(detail.forks, 7);
        assert_eq!(detail.open_issues, 3);
        assert_eq!(detail.watchers, 42);
        assert_eq!(detail.default_branch, "main");
        assert_eq!(detail.homepage, None, "empty/null homepage maps to None");
        assert_eq!(detail.license, None, "null license maps to None");
        assert_eq!(detail.topics, vec!["rust".to_string(), "cli".to_string()]);
        assert_eq!(detail.owner_login, "o");
        assert_eq!(detail.owner_avatar_url.as_deref(), Some("https://avatars/o.png"));
        assert_eq!(detail.size, 1234);
        assert!(!detail.is_archived);
    }

    #[tokio::test]
    async fn get_repo_detail_maps_license_spdx() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "full_name": "o/lic",
            "html_url": "https://github.com/o/lic",
            "default_branch": "main",
            "license": { "spdx_id": "MIT", "name": "MIT License" },
            "owner": { "login": "o" }
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/lic"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let detail = p.get_repo_detail("o", "lic").await.unwrap().unwrap();
        assert_eq!(detail.license.as_deref(), Some("MIT"));
    }

    #[tokio::test]
    async fn get_repo_detail_license_noassertion_falls_back_to_name() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "full_name": "o/na",
            "html_url": "https://github.com/o/na",
            "default_branch": "main",
            "license": { "spdx_id": "NOASSERTION", "name": "Custom License" },
            "owner": { "login": "o" }
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/na"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let detail = p.get_repo_detail("o", "na").await.unwrap().unwrap();
        assert_eq!(detail.license.as_deref(), Some("Custom License"));
    }

    #[tokio::test]
    async fn get_repo_detail_404_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/o/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Not Found"
            })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let detail = p.get_repo_detail("o", "missing").await.unwrap();
        assert!(detail.is_none(), "404 maps to Ok(None)");
    }

    #[tokio::test]
    async fn list_releases_maps_payload_null_name() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "id": 1,
                "tag_name": "v1.0.0",
                "name": "First release",
                "html_url": "https://github.com/o/r/releases/v1.0.0",
                "body": "notes",
                "draft": false,
                "prerelease": false,
                "published_at": "2026-06-15T10:00:00Z",
                "author": { "login": "alice" }
            },
            {
                "id": 2,
                "tag_name": "v1.1.0-rc1",
                "name": null,
                "html_url": "https://github.com/o/r/releases/v1.1.0-rc1",
                "body": null,
                "draft": true,
                "prerelease": true,
                "published_at": null,
                "author": null
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/releases"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let releases = p.list_releases("o", "r", 30).await.unwrap();

        assert_eq!(releases.len(), 2);
        assert_eq!(releases[0].tag_name, "v1.0.0");
        assert_eq!(releases[0].name.as_deref(), Some("First release"));
        assert_eq!(releases[0].author_login.as_deref(), Some("alice"));
        assert!(!releases[0].draft);

        assert_eq!(releases[1].name, None, "null name maps to None");
        assert_eq!(releases[1].body, None);
        assert_eq!(releases[1].published_at, None);
        assert_eq!(releases[1].author_login, None);
        assert!(releases[1].draft);
        assert!(releases[1].prerelease);
    }

    #[tokio::test]
    async fn list_forks_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "full_name": "fork-owner/r",
                "html_url": "https://github.com/fork-owner/r",
                "stargazers_count": 9,
                "pushed_at": "2026-06-14T10:00:00Z",
                "owner": { "login": "fork-owner" }
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/forks"))
            .and(query_param("sort", "stargazers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let forks = p.list_forks("o", "r", 30).await.unwrap();

        assert_eq!(forks.len(), 1);
        assert_eq!(forks[0].full_name, "fork-owner/r");
        assert_eq!(forks[0].stars, 9);
        assert_eq!(forks[0].owner_login, "fork-owner");
        assert_eq!(forks[0].pushed_at.as_deref(), Some("2026-06-14T10:00:00Z"));
    }

    #[tokio::test]
    async fn list_contributors_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "login": "alice",
                "avatar_url": "https://avatars/alice.png",
                "html_url": "https://github.com/alice",
                "contributions": 120
            },
            {
                "login": "bob",
                "avatar_url": null,
                "html_url": "https://github.com/bob",
                "contributions": 5
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/contributors"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let contributors = p.list_contributors("o", "r", 30).await.unwrap();

        assert_eq!(contributors.len(), 2);
        assert_eq!(contributors[0].login, "alice");
        assert_eq!(contributors[0].contributions, 120);
        assert_eq!(
            contributors[0].avatar_url.as_deref(),
            Some("https://avatars/alice.png")
        );
        assert_eq!(contributors[1].login, "bob");
        assert_eq!(contributors[1].avatar_url, None);
    }

    #[tokio::test]
    async fn get_languages_maps_object_to_sorted_vec() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "Rust": 50000,
            "TypeScript": 12000,
            "CSS": 30000
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/r/languages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let langs = p.get_languages("o", "r").await.unwrap();

        assert_eq!(langs.len(), 3);
        // Sorted by bytes desc: Rust (50000), CSS (30000), TypeScript (12000).
        assert_eq!(langs[0].name, "Rust");
        assert_eq!(langs[0].bytes, 50000);
        assert_eq!(langs[1].name, "CSS");
        assert_eq!(langs[1].bytes, 30000);
        assert_eq!(langs[2].name, "TypeScript");
        assert_eq!(langs[2].bytes, 12000);
    }

    #[tokio::test]
    async fn get_traffic_views_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "count": 1400,
            "uniques": 506,
            "views": [
                { "timestamp": "2026-06-14T00:00:00Z", "count": 100, "uniques": 40 },
                { "timestamp": "2026-06-15T00:00:00Z", "count": 80, "uniques": 30 }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/r/traffic/views"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let traffic = p.get_traffic_views("o", "r").await.unwrap().unwrap();

        assert_eq!(traffic.count, 1400);
        assert_eq!(traffic.uniques, 506);
        assert_eq!(traffic.days.len(), 2);
        assert_eq!(traffic.days[0].timestamp, "2026-06-14T00:00:00Z");
        assert_eq!(traffic.days[0].count, 100);
        assert_eq!(traffic.days[0].uniques, 40);
        assert_eq!(traffic.days[1].count, 80);
    }

    #[tokio::test]
    async fn get_traffic_clones_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "count": 173,
            "uniques": 128,
            "clones": [
                { "timestamp": "2026-06-14T00:00:00Z", "count": 10, "uniques": 8 }
            ]
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/r/traffic/clones"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let traffic = p.get_traffic_clones("o", "r").await.unwrap().unwrap();

        assert_eq!(traffic.count, 173);
        assert_eq!(traffic.uniques, 128);
        assert_eq!(traffic.days.len(), 1);
        assert_eq!(traffic.days[0].count, 10);
        assert_eq!(traffic.days[0].uniques, 8);
    }

    #[tokio::test]
    async fn list_referrers_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            { "referrer": "Google", "count": 4, "uniques": 3 },
            { "referrer": "github.com", "count": 2, "uniques": 2 }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/traffic/popular/referrers"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let referrers = p.list_referrers("o", "r").await.unwrap();

        assert_eq!(referrers.len(), 2);
        assert_eq!(referrers[0].referrer, "Google");
        assert_eq!(referrers[0].count, 4);
        assert_eq!(referrers[0].uniques, 3);
        assert_eq!(referrers[1].referrer, "github.com");
    }

    #[tokio::test]
    async fn list_paths_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            { "path": "/o/r", "title": "o/r: the repo", "count": 30, "uniques": 12 },
            { "path": "/o/r/issues", "title": "Issues", "count": 5, "uniques": 4 }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/traffic/popular/paths"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let paths = p.list_paths("o", "r").await.unwrap();

        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0].path, "/o/r");
        assert_eq!(paths[0].title, "o/r: the repo");
        assert_eq!(paths[0].count, 30);
        assert_eq!(paths[0].uniques, 12);
        assert_eq!(paths[1].path, "/o/r/issues");
    }

    #[tokio::test]
    async fn get_traffic_views_403_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/traffic/views"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "Must have push access to repository"
            })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let traffic = p.get_traffic_views("o", "r").await.unwrap();
        assert!(traffic.is_none(), "403 (no push access) maps to Ok(None)");
    }

    #[tokio::test]
    async fn list_referrers_403_returns_empty() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/traffic/popular/referrers"))
            .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
                "message": "Must have push access to repository"
            })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let referrers = p.list_referrers("o", "r").await.unwrap();
        assert!(referrers.is_empty(), "403 maps to empty list");
    }

    // --- triage write actions -------------------------------------------------

    #[tokio::test]
    async fn set_issue_state_patches_state() {
        let server = MockServer::start().await;
        Mock::given(method("PATCH"))
            .and(path("/repos/o/r/issues/42"))
            .and(body_partial_json(serde_json::json!({ "state": "closed" })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        p.set_issue_state("o", "r", 42, "closed").await.unwrap();
    }

    #[tokio::test]
    async fn add_issue_labels_posts_labels() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/o/r/issues/42/labels"))
            .and(body_partial_json(
                serde_json::json!({ "labels": ["bug", "p1"] }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        p.add_issue_labels("o", "r", 42, &["bug".into(), "p1".into()])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn remove_issue_label_deletes_label() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/repos/o/r/issues/42/labels/bug"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        p.remove_issue_label("o", "r", 42, "bug").await.unwrap();
    }

    #[tokio::test]
    async fn remove_issue_label_404_is_ok() {
        let server = MockServer::start().await;
        Mock::given(method("DELETE"))
            .and(path("/repos/o/r/issues/42/labels/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Label does not exist"
            })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        // A 404 (label not present) is treated as a successful no-op.
        p.remove_issue_label("o", "r", 42, "missing").await.unwrap();
    }

    #[tokio::test]
    async fn add_issue_assignees_posts_assignees() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/o/r/issues/42/assignees"))
            .and(body_partial_json(
                serde_json::json!({ "assignees": ["alice", "bob"] }),
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({})))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        p.add_issue_assignees("o", "r", 42, &["alice".into(), "bob".into()])
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn create_issue_comment_posts_body_and_returns_url() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/repos/o/r/issues/42/comments"))
            .and(body_partial_json(
                serde_json::json!({ "body": "looks good to me" }),
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": 1001,
                "html_url": "https://github.com/o/r/issues/42#issuecomment-1001"
            })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let url = p
            .create_issue_comment("o", "r", 42, "looks good to me")
            .await
            .unwrap();
        assert_eq!(url, "https://github.com/o/r/issues/42#issuecomment-1001");
    }

    // --- in-app issue/PR detail ----------------------------------------------

    #[tokio::test]
    async fn get_issue_maps_detail() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "number": 42,
            "title": "a detailed bug",
            "body": null,
            "html_url": "https://github.com/o/r/issues/42",
            "state": "open",
            "user": { "login": "alice", "avatar_url": "https://avatars/alice.png" },
            "repository_url": "https://api.github.com/repos/o/r",
            "created_at": "2026-06-10T10:00:00Z",
            "updated_at": "2026-06-15T10:00:00Z",
            "closed_at": null,
            "comments": 4,
            "labels": [{ "name": "bug", "color": "d73a4a" }],
            "assignees": [
                { "login": "bob", "avatar_url": "https://avatars/bob.png" },
                { "login": "carol", "avatar_url": null }
            ],
            "milestone": { "title": "v1.0" }
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/r/issues/42"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let issue = p.get_issue("o", "r", 42).await.unwrap().unwrap();

        assert_eq!(issue.number, 42);
        assert_eq!(issue.repo_name_with_owner, "o/r");
        assert_eq!(issue.body, None, "null body maps to None");
        assert_eq!(issue.author_login, "alice");
        assert_eq!(issue.comments_count, 4);
        assert_eq!(issue.labels.len(), 1);
        assert_eq!(issue.labels[0].name, "bug");
        assert_eq!(issue.labels[0].color, "d73a4a");
        assert_eq!(issue.assignees.len(), 2);
        assert_eq!(issue.assignees[0].login, "bob");
        assert_eq!(
            issue.assignees[0].avatar_url.as_deref(),
            Some("https://avatars/bob.png")
        );
        assert_eq!(issue.assignees[1].login, "carol");
        assert_eq!(issue.assignees[1].avatar_url, None);
        assert_eq!(issue.milestone_title.as_deref(), Some("v1.0"));
        assert_eq!(issue.closed_at, None);
    }

    #[tokio::test]
    async fn get_issue_404_returns_none() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/issues/999"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Not Found"
            })))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let issue = p.get_issue("o", "r", 999).await.unwrap();
        assert!(issue.is_none(), "404 maps to Ok(None)");
    }

    #[tokio::test]
    async fn get_pull_request_maps_detail() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "number": 7,
            "title": "a pr",
            "body": "fixes things",
            "html_url": "https://github.com/o/r/pull/7",
            "state": "open",
            "user": { "login": "carol", "avatar_url": null },
            "repository_url": "https://api.github.com/repos/o/r",
            "created_at": "2026-06-12T10:00:00Z",
            "updated_at": "2026-06-15T11:00:00Z",
            "closed_at": null,
            "comments": 2,
            "labels": [{ "name": "feature", "color": "a2eeef" }],
            "assignees": [{ "login": "dave", "avatar_url": null }],
            "milestone": null,
            "draft": true,
            "merged": false,
            "mergeable_state": null,
            "base": { "ref": "main" },
            "head": { "ref": "feature-x" },
            "additions": 120,
            "deletions": 7,
            "changed_files": 3,
            "commits": 5,
            "requested_reviewers": [
                { "login": "erin", "avatar_url": "https://avatars/erin.png" }
            ],
            "requested_teams": [{ "slug": "core-team" }]
        });
        Mock::given(method("GET"))
            .and(path("/repos/o/r/pulls/7"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let pr = p.get_pull_request("o", "r", 7).await.unwrap().unwrap();

        assert_eq!(pr.number, 7);
        assert_eq!(pr.repo_name_with_owner, "o/r");
        assert!(pr.is_draft);
        assert!(!pr.merged);
        assert_eq!(pr.mergeable_state, None, "null mergeable_state maps to None");
        assert_eq!(pr.base_ref, "main");
        assert_eq!(pr.head_ref, "feature-x");
        assert_eq!(pr.additions, Some(120));
        assert_eq!(pr.deletions, Some(7));
        assert_eq!(pr.changed_files, Some(3));
        assert_eq!(pr.commits, Some(5));
        assert_eq!(pr.comments_count, 2);
        assert_eq!(pr.labels[0].name, "feature");
        assert_eq!(pr.assignees.len(), 1);
        assert_eq!(pr.assignees[0].login, "dave");
        assert_eq!(pr.requested_reviewers.len(), 1);
        assert_eq!(pr.requested_reviewers[0].login, "erin");
        assert_eq!(
            pr.requested_reviewers[0].avatar_url.as_deref(),
            Some("https://avatars/erin.png")
        );
        assert_eq!(pr.requested_teams, vec!["core-team".to_string()]);
    }

    #[tokio::test]
    async fn list_issue_comments_maps_payload() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "id": 1001,
                "user": { "login": "alice", "avatar_url": "https://avatars/alice.png" },
                "body": "first comment",
                "created_at": "2026-06-13T10:00:00Z",
                "updated_at": "2026-06-13T10:05:00Z",
                "html_url": "https://github.com/o/r/issues/42#issuecomment-1001"
            },
            {
                "id": 1002,
                "user": { "login": "bob", "avatar_url": null },
                "body": "second comment",
                "created_at": "2026-06-14T10:00:00Z",
                "updated_at": "2026-06-14T10:00:00Z",
                "html_url": "https://github.com/o/r/issues/42#issuecomment-1002"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/issues/42/comments"))
            .and(query_param("per_page", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let comments = p.list_issue_comments("o", "r", 42, 100).await.unwrap();

        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].id, 1001);
        assert_eq!(comments[0].author_login, "alice");
        assert_eq!(
            comments[0].author_avatar_url.as_deref(),
            Some("https://avatars/alice.png")
        );
        assert_eq!(comments[0].body, "first comment");
        assert_eq!(
            comments[0].html_url,
            "https://github.com/o/r/issues/42#issuecomment-1001"
        );
        assert_eq!(comments[1].author_login, "bob");
        assert_eq!(comments[1].author_avatar_url, None);
    }

    #[tokio::test]
    async fn list_issue_comments_clamps_per_page() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/o/r/issues/42/comments"))
            .and(query_param("per_page", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let comments = p.list_issue_comments("o", "r", 42, 999).await.unwrap();
        assert!(comments.is_empty());
    }

    #[tokio::test]
    async fn list_pull_reviews_maps_states_and_null_body() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "id": 2001,
                "user": { "login": "erin", "avatar_url": "https://avatars/erin.png" },
                "state": "APPROVED",
                "body": "LGTM",
                "submitted_at": "2026-06-15T12:00:00Z",
                "html_url": "https://github.com/o/r/pull/7#pullrequestreview-2001"
            },
            {
                "id": 2002,
                "user": { "login": "frank", "avatar_url": null },
                "state": "CHANGES_REQUESTED",
                "body": "please fix",
                "submitted_at": "2026-06-15T13:00:00Z",
                "html_url": "https://github.com/o/r/pull/7#pullrequestreview-2002"
            },
            {
                "id": 2003,
                "user": { "login": "grace", "avatar_url": null },
                "state": "COMMENTED",
                "body": "",
                "submitted_at": "2026-06-15T14:00:00Z",
                "html_url": "https://github.com/o/r/pull/7#pullrequestreview-2003"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/pulls/7/reviews"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let p = provider(server.uri());
        let reviews = p.list_pull_reviews("o", "r", 7, 50).await.unwrap();

        assert_eq!(reviews.len(), 3);
        assert_eq!(reviews[0].id, 2001);
        assert_eq!(reviews[0].reviewer_login, "erin");
        assert_eq!(reviews[0].state, "APPROVED");
        assert_eq!(reviews[0].body.as_deref(), Some("LGTM"));
        assert_eq!(reviews[0].submitted_at.as_deref(), Some("2026-06-15T12:00:00Z"));

        assert_eq!(reviews[1].state, "CHANGES_REQUESTED");
        assert_eq!(reviews[1].body.as_deref(), Some("please fix"));

        assert_eq!(reviews[2].state, "COMMENTED");
        assert_eq!(reviews[2].body, None, "empty review body maps to None");
    }
}
