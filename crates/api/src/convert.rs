//! Conversions between our DTOs/core types and the generated proto messages.
//!
//! These are mechanical field-for-field maps. Service DTOs are the source of
//! truth; proto messages mirror them (snake_case fields, `optional` for
//! `Option`, `repeated` for `Vec`, JSON `serde_json::Value` carried as strings).

use crate::proto;
use newt_todo_core::{
    Account, RemoteIssue, RemotePullRequest, RemoteRepo, SyncChange, SyncReport,
};
use newt_todo_service::{
    BoardDto, CardDto, CodeHitDto, ColumnDto, CommentDto, ContributorDto, DigestDto, ForkDto,
    IssueDetailDto, LanguageDto, NotificationDto, PathDto, PullRequestDetailDto, ReferrerDto,
    ReleaseDto, RepoDetailDto, ReviewDto, SnapshotDto, TaskDto, TrafficDto, UserDto, WorkflowRunDto,
};

// --- cross-device sync (Phase K3) -------------------------------------------

impl From<SyncChange> for proto::Change {
    fn from(c: SyncChange) -> Self {
        proto::Change {
            kind: c.kind,
            id: c.id,
            updated_at: c.updated_at,
            deleted: c.deleted,
            data_json: c.data_json,
            board_id: c.board_id,
            column_id: c.column_id,
        }
    }
}

impl From<proto::Change> for SyncChange {
    fn from(c: proto::Change) -> Self {
        SyncChange {
            kind: c.kind,
            id: c.id,
            updated_at: c.updated_at,
            deleted: c.deleted,
            data_json: c.data_json,
            board_id: c.board_id,
            column_id: c.column_id,
        }
    }
}

// --- local todos ------------------------------------------------------------

impl From<TaskDto> for proto::Task {
    fn from(t: TaskDto) -> Self {
        proto::Task {
            id: t.id,
            title: t.title,
            body: t.body,
            status: t.status,
            labels: t.labels,
            due_at: t.due_at,
            updated_at: t.updated_at,
            source_url: t.source_url,
        }
    }
}

// --- accounts ---------------------------------------------------------------

impl From<Account> for proto::AccountMessage {
    fn from(a: Account) -> Self {
        proto::AccountMessage {
            id: a.id.to_string(),
            provider: a.provider.as_str().to_string(),
            display_name: a.display_name,
            base_url: a.base_url,
            config_json: a.config.to_string(),
            created_at: a
                .created_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

// --- sync -------------------------------------------------------------------

impl From<SyncReport> for proto::SyncReportMessage {
    fn from(r: SyncReport) -> Self {
        proto::SyncReportMessage {
            pulled: r.pulled as u64,
            pushed: r.pushed as u64,
        }
    }
}

// --- dashboard reads --------------------------------------------------------

impl From<RemoteRepo> for proto::RemoteRepoMessage {
    fn from(r: RemoteRepo) -> Self {
        proto::RemoteRepoMessage {
            id: r.id,
            full_name: r.full_name,
            description: r.description,
            html_url: r.html_url,
            stars: r.stars,
            open_issues: r.open_issues,
            language: r.language,
            is_private: r.is_private,
            is_fork: r.is_fork,
            updated_at: r
                .updated_at
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
        }
    }
}

impl From<newt_todo_core::RemoteLabel> for proto::LabelMessage {
    fn from(l: newt_todo_core::RemoteLabel) -> Self {
        proto::LabelMessage {
            name: l.name,
            color: l.color,
        }
    }
}

impl From<newt_todo_service::LabelDto> for proto::LabelMessage {
    fn from(l: newt_todo_service::LabelDto) -> Self {
        proto::LabelMessage {
            name: l.name,
            color: l.color,
        }
    }
}

impl From<RemoteIssue> for proto::RemoteIssueMessage {
    fn from(i: RemoteIssue) -> Self {
        let fmt = time::format_description::well_known::Rfc3339;
        proto::RemoteIssueMessage {
            number: i.number,
            title: i.title,
            html_url: i.html_url,
            state: i.state,
            author_login: i.author_login,
            author_avatar_url: i.author_avatar_url,
            repo_name_with_owner: i.repo_name_with_owner,
            created_at: i.created_at.format(&fmt).unwrap_or_default(),
            updated_at: i.updated_at.format(&fmt).unwrap_or_default(),
            comments_count: i.comments_count,
            labels: i.labels.into_iter().map(Into::into).collect(),
            assignees: i.assignees,
        }
    }
}

impl From<RemotePullRequest> for proto::RemotePullRequestMessage {
    fn from(p: RemotePullRequest) -> Self {
        let fmt = time::format_description::well_known::Rfc3339;
        proto::RemotePullRequestMessage {
            number: p.number,
            title: p.title,
            html_url: p.html_url,
            state: p.state,
            author_login: p.author_login,
            author_avatar_url: p.author_avatar_url,
            repo_name_with_owner: p.repo_name_with_owner,
            created_at: p.created_at.format(&fmt).unwrap_or_default(),
            updated_at: p.updated_at.format(&fmt).unwrap_or_default(),
            comments_count: p.comments_count,
            labels: p.labels.into_iter().map(Into::into).collect(),
            assignees: p.assignees,
            is_draft: p.is_draft,
        }
    }
}

// --- boards / columns / cards ----------------------------------------------

impl From<CardDto> for proto::Card {
    fn from(c: CardDto) -> Self {
        proto::Card {
            id: c.id,
            item_key: c.item_key,
            position: c.position,
        }
    }
}

impl From<ColumnDto> for proto::Column {
    fn from(c: ColumnDto) -> Self {
        proto::Column {
            id: c.id,
            name: c.name,
            position: c.position,
            filter_json: c.filter.to_string(),
            created_at: c.created_at,
            cards: c.cards.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<BoardDto> for proto::Board {
    fn from(b: BoardDto) -> Self {
        proto::Board {
            id: b.id,
            name: b.name,
            position: b.position,
            created_at: b.created_at,
            updated_at: b.updated_at,
            columns: b.columns.into_iter().map(Into::into).collect(),
        }
    }
}

// --- notifications ----------------------------------------------------------

impl From<NotificationDto> for proto::NotificationMessage {
    fn from(n: NotificationDto) -> Self {
        proto::NotificationMessage {
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

// --- CI workflow runs -------------------------------------------------------

impl From<WorkflowRunDto> for proto::WorkflowRunMessage {
    fn from(r: WorkflowRunDto) -> Self {
        proto::WorkflowRunMessage {
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

// --- repository detail ------------------------------------------------------

impl From<RepoDetailDto> for proto::RepoDetailMessage {
    fn from(r: RepoDetailDto) -> Self {
        proto::RepoDetailMessage {
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

impl From<ReleaseDto> for proto::ReleaseMessage {
    fn from(r: ReleaseDto) -> Self {
        proto::ReleaseMessage {
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

impl From<ForkDto> for proto::ForkMessage {
    fn from(f: ForkDto) -> Self {
        proto::ForkMessage {
            full_name: f.full_name,
            html_url: f.html_url,
            stars: f.stars,
            pushed_at: f.pushed_at,
            owner_login: f.owner_login,
        }
    }
}

impl From<ContributorDto> for proto::ContributorMessage {
    fn from(c: ContributorDto) -> Self {
        proto::ContributorMessage {
            login: c.login,
            avatar_url: c.avatar_url,
            html_url: c.html_url,
            contributions: c.contributions,
        }
    }
}

impl From<LanguageDto> for proto::LanguageMessage {
    fn from(l: LanguageDto) -> Self {
        proto::LanguageMessage {
            name: l.name,
            bytes: l.bytes,
        }
    }
}

// --- traffic ----------------------------------------------------------------

impl From<TrafficDto> for proto::TrafficMessage {
    fn from(t: TrafficDto) -> Self {
        proto::TrafficMessage {
            count: t.count,
            uniques: t.uniques,
            days: t
                .days
                .into_iter()
                .map(|d| proto::TrafficDayMessage {
                    timestamp: d.timestamp,
                    count: d.count,
                    uniques: d.uniques,
                })
                .collect(),
        }
    }
}

impl From<ReferrerDto> for proto::ReferrerMessage {
    fn from(r: ReferrerDto) -> Self {
        proto::ReferrerMessage {
            referrer: r.referrer,
            count: r.count,
            uniques: r.uniques,
        }
    }
}

impl From<PathDto> for proto::PathMessage {
    fn from(p: PathDto) -> Self {
        proto::PathMessage {
            path: p.path,
            title: p.title,
            count: p.count,
            uniques: p.uniques,
        }
    }
}

// --- mentions / search ------------------------------------------------------

impl From<CodeHitDto> for proto::CodeHitMessage {
    fn from(h: CodeHitDto) -> Self {
        proto::CodeHitMessage {
            repo_name_with_owner: h.repo_name_with_owner,
            path: h.path,
            html_url: h.html_url,
            name: h.name,
        }
    }
}

// --- in-app issue/PR detail -------------------------------------------------

impl From<UserDto> for proto::UserMessage {
    fn from(u: UserDto) -> Self {
        proto::UserMessage {
            login: u.login,
            avatar_url: u.avatar_url,
        }
    }
}

impl From<CommentDto> for proto::CommentMessage {
    fn from(c: CommentDto) -> Self {
        proto::CommentMessage {
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

impl From<ReviewDto> for proto::ReviewMessage {
    fn from(r: ReviewDto) -> Self {
        proto::ReviewMessage {
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

impl From<IssueDetailDto> for proto::IssueDetailMessage {
    fn from(i: IssueDetailDto) -> Self {
        proto::IssueDetailMessage {
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

impl From<PullRequestDetailDto> for proto::PullRequestDetailMessage {
    fn from(p: PullRequestDetailDto) -> Self {
        proto::PullRequestDetailMessage {
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

// --- daily repo snapshots ---------------------------------------------------

impl From<SnapshotDto> for proto::SnapshotMessage {
    fn from(s: SnapshotDto) -> Self {
        proto::SnapshotMessage {
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

impl From<DigestDto> for proto::DigestMessage {
    fn from(d: DigestDto) -> Self {
        proto::DigestMessage {
            day: d.day,
            previous_day: d.previous_day,
            entries: d
                .entries
                .into_iter()
                .map(|e| proto::DigestEntryMessage {
                    repo_full_name: e.repo_full_name,
                    stars_delta: e.stars_delta,
                    forks_delta: e.forks_delta,
                    open_issues_delta: e.open_issues_delta,
                    stars: e.stars,
                    forks: e.forks,
                    open_issues: e.open_issues,
                })
                .collect(),
        }
    }
}
