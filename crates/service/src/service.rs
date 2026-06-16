use newt_todo_auth::TokenStore;
use newt_todo_core::{
    sync_account as core_sync_account, Account, AccountDraft, Provider, ProviderKind, RemoteIssue,
    RemotePullRequest, RemoteRepo, Store, SyncReport, TaskDraft, TaskFilter, TaskPatch, LOCAL_USER,
};
use newt_todo_providers::GitHubProvider;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::dto::{
    compute_digest, parse_status, BoardDto, BoardIdParam, CardDto, CodeHitDto, ColumnDto,
    CommentDto, ContributorDto, CreateBoardParams, CreateColumnParams, CreateTaskParams, DigestDto,
    ForkDto, IssueAssigneesParams, IssueCommentParams, IssueDetailDto, IssueLabelsParams,
    LanguageDto, ListTasksParams, NotificationDto, PathDto, PlaceCardParams, PullRequestDetailDto,
    ReferrerDto, ReleaseDto, RemoveCardParams, RemoveLabelParams, RenameBoardParams, RepoDetailDto,
    ReorderBoardsParams, ReorderColumnsParams, ReviewDto, SetIssueStateParams, SnapshotDto, TaskDto,
    TaskIdParam, TrafficDto, UpdateColumnParams, UpdateTaskParams, WorkflowRunDto,
};
use newt_todo_core::BoardColumn;

pub struct TaskService {
    store: Store,
}

impl TaskService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// Direct access to the underlying core [`Store`], used by the server's
    /// cross-device sync RPCs (Phase K3) which operate on the sync-change set.
    pub fn store(&self) -> &Store {
        &self.store
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    pub async fn create(&self, p: CreateTaskParams) -> anyhow::Result<TaskDto> {
        self.create_for(LOCAL_USER, p).await
    }

    /// User-scoped task create (server multi-user). The new task is owned by `user_id`.
    pub async fn create_for(&self, user_id: &str, p: CreateTaskParams) -> anyhow::Result<TaskDto> {
        let mut draft = TaskDraft::new(p.title);
        draft.body = p.body.unwrap_or_default();
        draft.labels = p.labels.unwrap_or_default();
        if let Some(due) = p.due_at {
            draft.due_at = Some(OffsetDateTime::parse(&due, &Rfc3339)?);
        }
        let task = self
            .store
            .create_task_for(user_id, draft, Self::now())
            .await?;
        Ok(task.into())
    }

    pub async fn list(&self, p: ListTasksParams) -> anyhow::Result<Vec<TaskDto>> {
        self.list_for(LOCAL_USER, p).await
    }

    /// User-scoped task list: only `user_id`'s tasks.
    pub async fn list_for(
        &self,
        user_id: &str,
        p: ListTasksParams,
    ) -> anyhow::Result<Vec<TaskDto>> {
        let status = p.status.as_deref().map(parse_status).transpose()?;
        let filter = TaskFilter {
            status,
            label: p.label,
            query: p.query,
            include_deleted: false,
        };
        let tasks = self.store.list_tasks_for(user_id, filter).await?;
        Ok(tasks.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<TaskDto>> {
        self.get_for(LOCAL_USER, id).await
    }

    /// User-scoped get: returns the task only if owned by `user_id`.
    pub async fn get_for(&self, user_id: &str, id: &str) -> anyhow::Result<Option<TaskDto>> {
        let uuid = Uuid::parse_str(id)?;
        Ok(self.store.get_task_for(user_id, uuid).await?.map(Into::into))
    }

    pub async fn update(&self, p: UpdateTaskParams) -> anyhow::Result<TaskDto> {
        self.update_for(LOCAL_USER, p).await
    }

    /// User-scoped task update: errors with NotFound if not owned by `user_id`.
    pub async fn update_for(&self, user_id: &str, p: UpdateTaskParams) -> anyhow::Result<TaskDto> {
        let uuid = Uuid::parse_str(&p.id)?;
        let status = p.status.as_deref().map(parse_status).transpose()?;
        let due_at = if p.clear_due {
            Some(None)
        } else {
            match p.due_at {
                Some(s) => Some(Some(OffsetDateTime::parse(&s, &Rfc3339)?)),
                None => None,
            }
        };
        let patch = TaskPatch {
            title: p.title,
            body: p.body,
            status,
            labels: p.labels,
            due_at,
        };
        let task = self
            .store
            .update_task_for(user_id, uuid, patch, Self::now())
            .await?;
        Ok(task.into())
    }

    pub async fn complete(&self, p: TaskIdParam) -> anyhow::Result<TaskDto> {
        self.complete_for(LOCAL_USER, p).await
    }

    /// User-scoped complete: marks `user_id`'s task done (via `update_for`).
    pub async fn complete_for(&self, user_id: &str, p: TaskIdParam) -> anyhow::Result<TaskDto> {
        self.update_for(
            user_id,
            UpdateTaskParams {
                id: p.id,
                title: None,
                body: None,
                status: Some("done".into()),
                labels: None,
                due_at: None,
                clear_due: false,
            },
        )
        .await
    }

    pub async fn delete(&self, p: TaskIdParam) -> anyhow::Result<()> {
        self.delete_for(LOCAL_USER, p).await
    }

    /// User-scoped soft-delete: errors with NotFound if not owned by `user_id`.
    pub async fn delete_for(&self, user_id: &str, p: TaskIdParam) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(&p.id)?;
        self.store
            .delete_task_for(user_id, uuid, Self::now())
            .await?;
        Ok(())
    }

    pub async fn list_accounts(&self) -> anyhow::Result<Vec<Account>> {
        Ok(self.store.list_accounts().await?)
    }

    pub async fn create_account(&self, draft: AccountDraft) -> anyhow::Result<Account> {
        Ok(self.store.create_account(draft, Self::now()).await?)
    }

    pub async fn delete_account(&self, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        self.store.delete_account(uuid).await?;
        Ok(())
    }

    // --- boards ---------------------------------------------------------------

    /// List boards. Returned DTOs carry an EMPTY `columns` vec (cheap overview);
    /// use `get_board` to fetch a board with its ordered columns and manual cards.
    pub async fn list_boards(&self) -> anyhow::Result<Vec<BoardDto>> {
        self.list_boards_for(LOCAL_USER).await
    }

    /// User-scoped board list: only `user_id`'s boards (empty `columns` overview).
    pub async fn list_boards_for(&self, user_id: &str) -> anyhow::Result<Vec<BoardDto>> {
        let boards = self.store.list_boards_for(user_id).await?;
        Ok(boards
            .into_iter()
            .map(|b| BoardDto::from_parts(b, Vec::new()))
            .collect())
    }

    /// Assemble a board with its ordered columns, each carrying its ordered manual cards.
    pub async fn get_board(&self, p: BoardIdParam) -> anyhow::Result<Option<BoardDto>> {
        self.get_board_for(LOCAL_USER, p).await
    }

    /// User-scoped board fetch: returns the board (with columns/cards) only if
    /// owned by `user_id`; columns/cards are reached through the owned board.
    pub async fn get_board_for(
        &self,
        user_id: &str,
        p: BoardIdParam,
    ) -> anyhow::Result<Option<BoardDto>> {
        let uuid = Uuid::parse_str(&p.id)?;
        let Some(board) = self.store.get_board_for(user_id, uuid).await? else {
            return Ok(None);
        };
        let columns = self.store.list_columns(uuid).await?;
        let cards = self.store.list_cards(uuid).await?;

        let column_dtos = columns
            .into_iter()
            .map(|col: BoardColumn| {
                let col_id = col.id;
                let col_cards: Vec<CardDto> = cards
                    .iter()
                    .filter(|c| c.column_id == col_id)
                    .cloned()
                    .map(Into::into)
                    .collect();
                ColumnDto::from_parts(col, col_cards)
            })
            .collect();

        Ok(Some(BoardDto::from_parts(board, column_dtos)))
    }

    pub async fn create_board(&self, p: CreateBoardParams) -> anyhow::Result<BoardDto> {
        self.create_board_for(LOCAL_USER, p).await
    }

    /// User-scoped board create. The new board is owned by `user_id`.
    pub async fn create_board_for(
        &self,
        user_id: &str,
        p: CreateBoardParams,
    ) -> anyhow::Result<BoardDto> {
        let board = self
            .store
            .create_board_for(user_id, &p.name, Self::now())
            .await?;
        Ok(BoardDto::from_parts(board, Vec::new()))
    }

    pub async fn rename_board(&self, p: RenameBoardParams) -> anyhow::Result<BoardDto> {
        self.rename_board_for(LOCAL_USER, p).await
    }

    /// User-scoped rename: errors with NotFound if not owned by `user_id`.
    pub async fn rename_board_for(
        &self,
        user_id: &str,
        p: RenameBoardParams,
    ) -> anyhow::Result<BoardDto> {
        let uuid = Uuid::parse_str(&p.id)?;
        let board = self
            .store
            .rename_board_for(user_id, uuid, &p.name, Self::now())
            .await?;
        Ok(BoardDto::from_parts(board, Vec::new()))
    }

    pub async fn delete_board(&self, p: BoardIdParam) -> anyhow::Result<()> {
        self.delete_board_for(LOCAL_USER, p).await
    }

    /// User-scoped soft-delete: errors with NotFound if not owned by `user_id`.
    pub async fn delete_board_for(&self, user_id: &str, p: BoardIdParam) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(&p.id)?;
        self.store
            .delete_board_for(user_id, uuid, Self::now())
            .await?;
        Ok(())
    }

    pub async fn reorder_boards(&self, p: ReorderBoardsParams) -> anyhow::Result<()> {
        self.reorder_boards_for(LOCAL_USER, p).await
    }

    /// User-scoped reorder: only affects boards owned by `user_id`.
    pub async fn reorder_boards_for(
        &self,
        user_id: &str,
        p: ReorderBoardsParams,
    ) -> anyhow::Result<()> {
        let ids = p
            .ordered_ids
            .iter()
            .map(|s| Uuid::parse_str(s))
            .collect::<Result<Vec<_>, _>>()?;
        self.store
            .reorder_boards_for(user_id, &ids, Self::now())
            .await?;
        Ok(())
    }

    // --- columns --------------------------------------------------------------

    pub async fn create_column(&self, p: CreateColumnParams) -> anyhow::Result<ColumnDto> {
        self.create_column_for(LOCAL_USER, p).await
    }

    /// User-scoped column create. Core verifies the parent board belongs to `user_id`.
    pub async fn create_column_for(
        &self,
        user_id: &str,
        p: CreateColumnParams,
    ) -> anyhow::Result<ColumnDto> {
        let board_id = Uuid::parse_str(&p.board_id)?;
        let filter = p.filter.unwrap_or_else(|| serde_json::json!({}));
        let column = self
            .store
            .create_column_for(user_id, board_id, &p.name, &filter, Self::now())
            .await?;
        Ok(ColumnDto::from_parts(column, Vec::new()))
    }

    pub async fn update_column(&self, p: UpdateColumnParams) -> anyhow::Result<ColumnDto> {
        self.update_column_for(LOCAL_USER, p).await
    }

    /// User-scoped column update. Core verifies the column's board belongs to `user_id`.
    pub async fn update_column_for(
        &self,
        user_id: &str,
        p: UpdateColumnParams,
    ) -> anyhow::Result<ColumnDto> {
        let id = Uuid::parse_str(&p.id)?;
        let filter = p.filter.unwrap_or_else(|| serde_json::json!({}));
        let column = self
            .store
            .update_column_for(user_id, id, &p.name, &filter, Self::now())
            .await?;
        Ok(ColumnDto::from_parts(column, Vec::new()))
    }

    pub async fn delete_column(&self, id: &str) -> anyhow::Result<()> {
        self.delete_column_for(LOCAL_USER, id).await
    }

    /// User-scoped column soft-delete. Core verifies the column's board belongs to `user_id`.
    pub async fn delete_column_for(&self, user_id: &str, id: &str) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(id)?;
        self.store
            .delete_column_for(user_id, uuid, Self::now())
            .await?;
        Ok(())
    }

    pub async fn reorder_columns(&self, p: ReorderColumnsParams) -> anyhow::Result<()> {
        self.reorder_columns_for(LOCAL_USER, p).await
    }

    /// User-scoped column reorder. Core verifies the board belongs to `user_id`.
    pub async fn reorder_columns_for(
        &self,
        user_id: &str,
        p: ReorderColumnsParams,
    ) -> anyhow::Result<()> {
        let board_id = Uuid::parse_str(&p.board_id)?;
        let ids = p
            .ordered_ids
            .iter()
            .map(|s| Uuid::parse_str(s))
            .collect::<Result<Vec<_>, _>>()?;
        self.store
            .reorder_columns_for(user_id, board_id, &ids, Self::now())
            .await?;
        Ok(())
    }

    // --- cards ----------------------------------------------------------------

    pub async fn place_card(&self, p: PlaceCardParams) -> anyhow::Result<CardDto> {
        self.place_card_for(LOCAL_USER, p).await
    }

    /// User-scoped card placement. Core verifies the board belongs to `user_id`.
    pub async fn place_card_for(
        &self,
        user_id: &str,
        p: PlaceCardParams,
    ) -> anyhow::Result<CardDto> {
        let board_id = Uuid::parse_str(&p.board_id)?;
        let column_id = Uuid::parse_str(&p.column_id)?;
        let card = self
            .store
            .place_card_for(user_id, board_id, column_id, &p.item_key, Self::now())
            .await?;
        Ok(card.into())
    }

    pub async fn remove_card(&self, p: RemoveCardParams) -> anyhow::Result<()> {
        self.remove_card_for(LOCAL_USER, p).await
    }

    /// User-scoped card removal. Core verifies the board belongs to `user_id`.
    pub async fn remove_card_for(&self, user_id: &str, p: RemoveCardParams) -> anyhow::Result<()> {
        let board_id = Uuid::parse_str(&p.board_id)?;
        self.store
            .remove_card_for(user_id, board_id, &p.item_key, Self::now())
            .await?;
        Ok(())
    }

    /// Orchestrate a sync for one account: load it, build the right provider
    /// using its stored token, and run `core::sync::sync_account`.
    pub async fn sync(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<SyncReport> {
        let uuid = Uuid::parse_str(account_id)?;
        let account = self
            .store
            .get_account(uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("account not found: {account_id}"))?;

        let now = OffsetDateTime::now_utc();
        match account.provider {
            ProviderKind::Github => {
                let owner = account.config["owner"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("github account missing config.owner"))?;
                let repo = account.config["repo"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("github account missing config.repo"))?;
                let token = token_store
                    .load(account_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .ok_or_else(|| anyhow::anyhow!("no token stored for account {account_id}"))?;
                let base_url = account
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.github.com".into());
                let provider = GitHubProvider::new(
                    reqwest::Client::new(),
                    base_url,
                    token.access_token,
                    owner,
                    repo,
                );
                let report = core_sync_account(&self.store, uuid, &provider, now).await?;
                Ok(report)
            }
            other => anyhow::bail!("provider not yet supported: {}", other.as_str()),
        }
    }

    /// List the repositories visible to an account's authenticated token.
    pub async fn list_repos(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<Vec<RemoteRepo>> {
        let uuid = Uuid::parse_str(account_id)?;
        let account = self
            .store
            .get_account(uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("account not found: {account_id}"))?;

        match account.provider {
            ProviderKind::Github => {
                // /user/repos is account-wide and ignores owner/repo, but the
                // constructor takes them — fall back to empty when not configured
                // (an account connected for the dashboard has no single repo).
                let owner = account.config["owner"].as_str().unwrap_or("");
                let repo = account.config["repo"].as_str().unwrap_or("");
                let token = token_store
                    .load(account_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .ok_or_else(|| anyhow::anyhow!("no token stored for account {account_id}"))?;
                let base_url = account
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.github.com".into());
                let provider = GitHubProvider::new(
                    reqwest::Client::new(),
                    base_url,
                    token.access_token,
                    owner,
                    repo,
                );
                let repos = provider
                    .list_repos()
                    .await
                    .map_err(|e| anyhow::anyhow!(e))?;
                Ok(repos)
            }
            other => anyhow::bail!("provider not yet supported: {}", other.as_str()),
        }
    }

    /// Build a GitHubProvider for an account's stored token.
    ///
    /// The dashboard endpoints (`/search/issues`) are account-wide and ignore
    /// owner/repo, so those fall back to empty when not configured.
    async fn github_provider(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<GitHubProvider> {
        let uuid = Uuid::parse_str(account_id)?;
        let account = self
            .store
            .get_account(uuid)
            .await?
            .ok_or_else(|| anyhow::anyhow!("account not found: {account_id}"))?;

        match account.provider {
            ProviderKind::Github => {
                let owner = account.config["owner"].as_str().unwrap_or("");
                let repo = account.config["repo"].as_str().unwrap_or("");
                let token = token_store
                    .load(account_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .ok_or_else(|| anyhow::anyhow!("no token stored for account {account_id}"))?;
                let base_url = account
                    .base_url
                    .clone()
                    .unwrap_or_else(|| "https://api.github.com".into());
                Ok(GitHubProvider::new(
                    reqwest::Client::new(),
                    base_url,
                    token.access_token,
                    owner,
                    repo,
                ))
            }
            other => anyhow::bail!("provider not yet supported: {}", other.as_str()),
        }
    }

    /// List issues that involve an account's authenticated user.
    pub async fn list_issues(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<Vec<RemoteIssue>> {
        let provider = self.github_provider(token_store, account_id).await?;
        provider.list_issues().await.map_err(|e| anyhow::anyhow!(e))
    }

    /// List pull requests that involve an account's authenticated user.
    pub async fn list_pull_requests(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<Vec<RemotePullRequest>> {
        let provider = self.github_provider(token_store, account_id).await?;
        provider
            .list_pull_requests()
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Search code across GitHub for an account (Mentions tab).
    pub async fn search_code(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        query: &str,
        per_page: u32,
    ) -> anyhow::Result<Vec<CodeHitDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let hits = provider
            .search_code(query, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(hits.into_iter().map(Into::into).collect())
    }

    /// Search issues/PRs across GitHub for an account (Mentions tab).
    pub async fn search_issues(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        query: &str,
        per_page: u32,
    ) -> anyhow::Result<Vec<RemoteIssue>> {
        let provider = self.github_provider(token_store, account_id).await?;
        provider
            .search_issues(query, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// List notification threads for an account (Inbox view).
    pub async fn list_notifications(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<Vec<NotificationDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let notifications = provider
            .list_notifications()
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(notifications.into_iter().map(Into::into).collect())
    }

    /// Mark a single notification thread read for an account.
    pub async fn mark_notification_read(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        thread_id: &str,
    ) -> anyhow::Result<()> {
        let provider = self.github_provider(token_store, account_id).await?;
        provider
            .mark_notification_read(thread_id)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// List recent workflow runs for a repo under an account (CI Health view).
    pub async fn list_workflow_runs(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> anyhow::Result<Vec<WorkflowRunDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let runs = provider
            .list_workflow_runs(owner, repo, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(runs.into_iter().map(Into::into).collect())
    }

    /// Get full detail for a single repository (Repository Details view).
    pub async fn get_repo_detail(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<RepoDetailDto> {
        let provider = self.github_provider(token_store, account_id).await?;
        let detail = provider
            .get_repo_detail(owner, repo)
            .await
            .map_err(|e| anyhow::anyhow!(e))?
            .ok_or_else(|| anyhow::anyhow!("repository not found: {owner}/{repo}"))?;
        Ok(detail.into())
    }

    /// List releases for a repository (Repository Details view).
    pub async fn list_releases(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> anyhow::Result<Vec<ReleaseDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let releases = provider
            .list_releases(owner, repo, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(releases.into_iter().map(Into::into).collect())
    }

    /// List forks for a repository (Repository Details view).
    pub async fn list_forks(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> anyhow::Result<Vec<ForkDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let forks = provider
            .list_forks(owner, repo, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(forks.into_iter().map(Into::into).collect())
    }

    /// List contributors for a repository (Repository Details view).
    pub async fn list_contributors(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        per_page: u32,
    ) -> anyhow::Result<Vec<ContributorDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let contributors = provider
            .list_contributors(owner, repo, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(contributors.into_iter().map(Into::into).collect())
    }

    /// Get language breakdown for a repository (Repository Details view).
    pub async fn get_languages(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<Vec<LanguageDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let langs = provider
            .get_languages(owner, repo)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(langs.into_iter().map(Into::into).collect())
    }

    /// Get the daily view traffic for a repository (Traffic view).
    ///
    /// Returns `None` when the account can't read traffic (no push access) so
    /// the UI degrades gracefully instead of surfacing an error.
    pub async fn get_traffic_views(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<Option<TrafficDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let traffic = provider
            .get_traffic_views(owner, repo)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(traffic.map(Into::into))
    }

    /// Get the daily clone traffic for a repository (Traffic view).
    ///
    /// Returns `None` when the account can't read traffic (no push access).
    pub async fn get_traffic_clones(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<Option<TrafficDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let traffic = provider
            .get_traffic_clones(owner, repo)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(traffic.map(Into::into))
    }

    /// List the popular referrers for a repository (Traffic view).
    ///
    /// Returns an empty list when the account can't read traffic.
    pub async fn list_referrers(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<Vec<ReferrerDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let referrers = provider
            .list_referrers(owner, repo)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(referrers.into_iter().map(Into::into).collect())
    }

    /// List the popular content paths for a repository (Traffic view).
    ///
    /// Returns an empty list when the account can't read traffic.
    pub async fn list_paths(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
    ) -> anyhow::Result<Vec<PathDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let paths = provider
            .list_paths(owner, repo)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(paths.into_iter().map(Into::into).collect())
    }

    // --- in-app issue/PR detail -----------------------------------------------

    /// Get full detail for a single issue. `None` when it doesn't exist.
    pub async fn get_issue(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> anyhow::Result<Option<IssueDetailDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let detail = provider
            .get_issue(owner, repo, number)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(detail.map(Into::into))
    }

    /// Get full detail for a single pull request. `None` when it doesn't exist.
    pub async fn get_pull_request(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        number: u64,
    ) -> anyhow::Result<Option<PullRequestDetailDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let detail = provider
            .get_pull_request(owner, repo, number)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(detail.map(Into::into))
    }

    /// List the issue-comment timeline for an issue or PR.
    pub async fn list_issue_comments(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        number: u64,
        per_page: u32,
    ) -> anyhow::Result<Vec<CommentDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let comments = provider
            .list_issue_comments(owner, repo, number, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(comments.into_iter().map(Into::into).collect())
    }

    /// List the reviews on a pull request.
    pub async fn list_pull_reviews(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
        owner: &str,
        repo: &str,
        number: u64,
        per_page: u32,
    ) -> anyhow::Result<Vec<ReviewDto>> {
        let provider = self.github_provider(token_store, account_id).await?;
        let reviews = provider
            .list_pull_reviews(owner, repo, number, per_page)
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        Ok(reviews.into_iter().map(Into::into).collect())
    }

    // --- triage write actions -------------------------------------------------

    /// Set an issue's state ("open" | "closed") for an account.
    pub async fn set_issue_state(
        &self,
        token_store: &dyn TokenStore,
        p: SetIssueStateParams,
    ) -> anyhow::Result<()> {
        let provider = self.github_provider(token_store, &p.id).await?;
        provider
            .set_issue_state(&p.owner, &p.repo, p.number, &p.state)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Add labels to an issue (additive) for an account.
    pub async fn add_issue_labels(
        &self,
        token_store: &dyn TokenStore,
        p: IssueLabelsParams,
    ) -> anyhow::Result<()> {
        let provider = self.github_provider(token_store, &p.id).await?;
        provider
            .add_issue_labels(&p.owner, &p.repo, p.number, &p.labels)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Remove a single label from an issue for an account.
    pub async fn remove_issue_label(
        &self,
        token_store: &dyn TokenStore,
        p: RemoveLabelParams,
    ) -> anyhow::Result<()> {
        let provider = self.github_provider(token_store, &p.id).await?;
        provider
            .remove_issue_label(&p.owner, &p.repo, p.number, &p.label)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Add assignees to an issue for an account.
    pub async fn add_issue_assignees(
        &self,
        token_store: &dyn TokenStore,
        p: IssueAssigneesParams,
    ) -> anyhow::Result<()> {
        let provider = self.github_provider(token_store, &p.id).await?;
        provider
            .add_issue_assignees(&p.owner, &p.repo, p.number, &p.assignees)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    /// Create a comment on an issue for an account; returns the comment's URL.
    pub async fn create_issue_comment(
        &self,
        token_store: &dyn TokenStore,
        p: IssueCommentParams,
    ) -> anyhow::Result<String> {
        let provider = self.github_provider(token_store, &p.id).await?;
        provider
            .create_issue_comment(&p.owner, &p.repo, p.number, &p.body)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }

    // --- daily repo snapshots -------------------------------------------------

    /// Capture today's metrics for every repo visible to the account.
    ///
    /// Fetches the account's repos via the same provider path `list_repos` uses,
    /// then upserts one snapshot row per repo for `today_ymd()`. Re-running the
    /// same day overwrites that day's rows with the latest numbers. Returns the
    /// number of repos captured. Intended to be called once on app open.
    ///
    /// Note: `RemoteRepo` exposes stars and open_issues but not forks, so `forks`
    /// is recorded as 0 for now (the column exists for digest symmetry and a
    /// future fork count).
    pub async fn capture_snapshots(
        &self,
        token_store: &dyn TokenStore,
        account_id: &str,
    ) -> anyhow::Result<usize> {
        let repos = self.list_repos(token_store, account_id).await?;
        let day = newt_todo_core::today_ymd();
        let mut count = 0usize;
        for repo in repos {
            self.store
                .upsert_snapshot(
                    account_id,
                    &repo.full_name,
                    &day,
                    repo.stars as i64,
                    0,
                    repo.open_issues as i64,
                )
                .await?;
            count += 1;
        }
        Ok(count)
    }

    /// List persisted snapshots for an account, optionally from `since_day` onward.
    pub async fn list_snapshots(
        &self,
        account_id: &str,
        since_day: Option<String>,
    ) -> anyhow::Result<Vec<SnapshotDto>> {
        let snaps = self
            .store
            .list_snapshots(account_id, since_day.as_deref())
            .await?;
        Ok(snaps.into_iter().map(Into::into).collect())
    }

    /// Build a daily digest by diffing the latest captured day against the
    /// previous captured day for each repo. The diff itself is the pure
    /// [`compute_digest`] function (unit-tested in `dto`); this method only
    /// loads the rows and delegates.
    pub async fn daily_digest(&self, account_id: &str) -> anyhow::Result<DigestDto> {
        // Only the two most recent days are needed for the diff; load from the
        // earlier of them so `compute_digest` sees both.
        let days = self.store.latest_two_days(account_id).await?;
        let since = days.last().cloned();
        let snaps = self
            .store
            .list_snapshots(account_id, since.as_deref())
            .await?;
        Ok(compute_digest(&snaps))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use newt_todo_auth::{MemoryTokenStore, OAuthToken, TokenStore};
    use newt_todo_core::{AccountDraft, ProviderKind};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn svc() -> TaskService {
        TaskService::new(Store::connect("sqlite::memory:").await.unwrap())
    }

    #[tokio::test]
    async fn sync_pulls_issues_from_github_account() {
        let server = MockServer::start().await;
        let issue = serde_json::json!([{
            "number": 1,
            "title": "remote issue",
            "body": "from github",
            "state": "open",
            "html_url": "https://github.com/o/r/issues/1",
            "updated_at": "2026-06-15T10:00:00Z",
            "labels": []
        }]);
        Mock::given(method("GET"))
            .and(path("/repos/o/r/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(issue))
            .mount(&server)
            .await;

        let svc = svc().await;
        let account = svc
            .create_account(AccountDraft {
                provider: ProviderKind::Github,
                display_name: "gh".into(),
                base_url: Some(server.uri()),
                config: serde_json::json!({ "owner": "o", "repo": "r" }),
            })
            .await
            .unwrap();
        let account_id = account.id.to_string();

        let token_store = MemoryTokenStore::new();
        token_store
            .save(
                &account_id,
                &OAuthToken {
                    access_token: "gho_x".into(),
                    refresh_token: None,
                },
            )
            .await
            .unwrap();

        let report = svc.sync(&token_store, &account_id).await.unwrap();
        assert_eq!(report.pulled, 1);
    }

    #[tokio::test]
    async fn list_repos_returns_account_repos() {
        let server = MockServer::start().await;
        let body = serde_json::json!([{
            "id": 100,
            "full_name": "o/alpha",
            "description": "the alpha repo",
            "html_url": "https://github.com/o/alpha",
            "stargazers_count": 5,
            "open_issues_count": 1,
            "language": "Rust",
            "private": false,
            "fork": false,
            "updated_at": "2026-06-15T10:00:00Z"
        }]);
        Mock::given(method("GET"))
            .and(path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let svc = svc().await;
        let account = svc
            .create_account(AccountDraft {
                provider: ProviderKind::Github,
                display_name: "gh".into(),
                base_url: Some(server.uri()),
                config: serde_json::json!({ "owner": "o", "repo": "r" }),
            })
            .await
            .unwrap();
        let account_id = account.id.to_string();

        let token_store = MemoryTokenStore::new();
        token_store
            .save(
                &account_id,
                &OAuthToken {
                    access_token: "gho_x".into(),
                    refresh_token: None,
                },
            )
            .await
            .unwrap();

        let repos = svc.list_repos(&token_store, &account_id).await.unwrap();
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].full_name, "o/alpha");
    }

    #[tokio::test]
    async fn list_issues_returns_account_issues() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "items": [{
                "number": 11,
                "title": "service issue",
                "html_url": "https://github.com/o/r/issues/11",
                "state": "open",
                "user": { "login": "alice", "avatar_url": null },
                "comments": 0,
                "labels": [{ "name": "bug", "color": "d73a4a" }],
                "assignees": [],
                "created_at": "2026-06-10T10:00:00Z",
                "updated_at": "2026-06-15T10:00:00Z",
                "repository_url": "https://api.github.com/repos/o/r"
            }]
        });
        Mock::given(method("GET"))
            .and(path("/search/issues"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let svc = svc().await;
        let account = svc
            .create_account(AccountDraft {
                provider: ProviderKind::Github,
                display_name: "gh".into(),
                base_url: Some(server.uri()),
                config: serde_json::json!({}),
            })
            .await
            .unwrap();
        let account_id = account.id.to_string();

        let token_store = MemoryTokenStore::new();
        token_store
            .save(
                &account_id,
                &OAuthToken {
                    access_token: "gho_x".into(),
                    refresh_token: None,
                },
            )
            .await
            .unwrap();

        let issues = svc.list_issues(&token_store, &account_id).await.unwrap();
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 11);
        assert_eq!(issues[0].repo_name_with_owner, "o/r");
        assert_eq!(issues[0].labels[0].color, "d73a4a");
    }

    #[tokio::test]
    async fn capture_snapshots_then_list_and_digest() {
        let server = MockServer::start().await;
        let body = serde_json::json!([
            {
                "id": 1,
                "full_name": "o/alpha",
                "description": null,
                "html_url": "https://github.com/o/alpha",
                "stargazers_count": 5,
                "open_issues_count": 2,
                "language": "Rust",
                "private": false,
                "fork": false,
                "updated_at": "2026-06-16T10:00:00Z"
            },
            {
                "id": 2,
                "full_name": "o/beta",
                "description": null,
                "html_url": "https://github.com/o/beta",
                "stargazers_count": 1,
                "open_issues_count": 0,
                "language": null,
                "private": false,
                "fork": false,
                "updated_at": "2026-06-16T10:00:00Z"
            }
        ]);
        Mock::given(method("GET"))
            .and(path("/user/repos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(&server)
            .await;

        let svc = svc().await;
        let account = svc
            .create_account(AccountDraft {
                provider: ProviderKind::Github,
                display_name: "gh".into(),
                base_url: Some(server.uri()),
                config: serde_json::json!({}),
            })
            .await
            .unwrap();
        let account_id = account.id.to_string();

        let token_store = MemoryTokenStore::new();
        token_store
            .save(
                &account_id,
                &OAuthToken {
                    access_token: "gho_x".into(),
                    refresh_token: None,
                },
            )
            .await
            .unwrap();

        let n = svc
            .capture_snapshots(&token_store, &account_id)
            .await
            .unwrap();
        assert_eq!(n, 2);

        let snaps = svc.list_snapshots(&account_id, None).await.unwrap();
        assert_eq!(snaps.len(), 2);
        assert_eq!(snaps[0].repo_full_name, "o/alpha");
        assert_eq!(snaps[0].stars, 5);
        assert_eq!(snaps[0].open_issues, 2);

        // Only one captured day → digest has no previous day and zero deltas.
        let digest = svc.daily_digest(&account_id).await.unwrap();
        assert_eq!(digest.previous_day, None);
        assert_eq!(digest.entries.len(), 2);
        assert!(digest
            .entries
            .iter()
            .all(|e| e.stars_delta == 0 && e.open_issues_delta == 0));
    }

    #[tokio::test]
    async fn create_then_list_and_get() {
        let svc = svc().await;
        let created = svc
            .create(CreateTaskParams {
                title: "write tests".into(),
                body: Some("for the mcp".into()),
                labels: Some(vec!["dev".into()]),
                due_at: None,
            })
            .await
            .unwrap();
        assert_eq!(created.title, "write tests");
        assert_eq!(created.status, "open");

        let listed = svc
            .list(ListTasksParams {
                status: None,
                label: None,
                query: None,
            })
            .await
            .unwrap();
        assert_eq!(listed.len(), 1);

        let got = svc.get(&created.id).await.unwrap().unwrap();
        assert_eq!(got.id, created.id);

        assert!(svc
            .get(&Uuid::new_v4().to_string())
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn list_filters_by_status() {
        let svc = svc().await;
        svc.create(CreateTaskParams {
            title: "a".into(),
            body: None,
            labels: None,
            due_at: None,
        })
        .await
        .unwrap();
        let only_done = svc
            .list(ListTasksParams {
                status: Some("done".into()),
                label: None,
                query: None,
            })
            .await
            .unwrap();
        assert!(only_done.is_empty());
        let bad = svc
            .list(ListTasksParams {
                status: Some("nope".into()),
                label: None,
                query: None,
            })
            .await;
        assert!(bad.is_err(), "invalid status must error");
    }

    #[tokio::test]
    async fn update_complete_delete() {
        let svc = svc().await;
        let t = svc
            .create(CreateTaskParams {
                title: "task".into(),
                body: None,
                labels: None,
                due_at: None,
            })
            .await
            .unwrap();

        // update title + status
        let upd = svc
            .update(UpdateTaskParams {
                id: t.id.clone(),
                title: Some("renamed".into()),
                body: None,
                status: Some("done".into()),
                labels: None,
                due_at: None,
                clear_due: false,
            })
            .await
            .unwrap();
        assert_eq!(upd.title, "renamed");
        assert_eq!(upd.status, "done");

        // complete is idempotent-ish: sets status done
        let t2 = svc
            .create(CreateTaskParams {
                title: "two".into(),
                body: None,
                labels: None,
                due_at: None,
            })
            .await
            .unwrap();
        let done = svc
            .complete(TaskIdParam { id: t2.id.clone() })
            .await
            .unwrap();
        assert_eq!(done.status, "done");

        // delete hides it from listing
        svc.delete(TaskIdParam { id: t.id.clone() }).await.unwrap();
        let remaining = svc
            .list(ListTasksParams {
                status: None,
                label: None,
                query: None,
            })
            .await
            .unwrap();
        assert!(
            remaining.iter().all(|x| x.id != t.id),
            "deleted task is hidden"
        );
    }

    #[tokio::test]
    async fn get_board_assembles_ordered_columns_and_cards() {
        use crate::dto::{
            CreateBoardParams, CreateColumnParams, PlaceCardParams, ReorderColumnsParams,
        };

        let svc = svc().await;
        let board = svc
            .create_board(CreateBoardParams { name: "PM".into() })
            .await
            .unwrap();

        let col_a = svc
            .create_column(CreateColumnParams {
                board_id: board.id.clone(),
                name: "A".into(),
                filter: Some(serde_json::json!({ "state": "open" })),
            })
            .await
            .unwrap();
        let col_b = svc
            .create_column(CreateColumnParams {
                board_id: board.id.clone(),
                name: "B".into(),
                filter: None,
            })
            .await
            .unwrap();

        // Two manual cards in col_a (order by placement position).
        svc.place_card(PlaceCardParams {
            board_id: board.id.clone(),
            column_id: col_a.id.clone(),
            item_key: "todo:1".into(),
        })
        .await
        .unwrap();
        svc.place_card(PlaceCardParams {
            board_id: board.id.clone(),
            column_id: col_a.id.clone(),
            item_key: "todo:2".into(),
        })
        .await
        .unwrap();

        // Reorder columns: B before A.
        svc.reorder_columns(ReorderColumnsParams {
            board_id: board.id.clone(),
            ordered_ids: vec![col_b.id.clone(), col_a.id.clone()],
        })
        .await
        .unwrap();

        let full = svc
            .get_board(BoardIdParam { id: board.id.clone() })
            .await
            .unwrap()
            .unwrap();

        assert_eq!(full.columns.len(), 2);
        assert_eq!(full.columns[0].id, col_b.id, "reordered: B first");
        assert_eq!(full.columns[1].id, col_a.id);
        assert!(full.columns[0].cards.is_empty());
        let a_cards: Vec<&str> = full.columns[1]
            .cards
            .iter()
            .map(|c| c.item_key.as_str())
            .collect();
        assert_eq!(a_cards, vec!["todo:1", "todo:2"]);
        // Filter is carried through opaquely.
        assert_eq!(full.columns[1].filter, serde_json::json!({ "state": "open" }));

        // list_boards is cheap: columns empty.
        let boards = svc.list_boards().await.unwrap();
        assert_eq!(boards.len(), 1);
        assert!(boards[0].columns.is_empty());
    }

    #[tokio::test]
    async fn update_clear_due_overrides_due_at() {
        let svc = svc().await;
        let t = svc
            .create(CreateTaskParams {
                title: "due".into(),
                body: None,
                labels: None,
                due_at: Some("2026-07-01T09:00:00Z".into()),
            })
            .await
            .unwrap();
        assert!(t.due_at.is_some());

        let cleared = svc
            .update(UpdateTaskParams {
                id: t.id.clone(),
                title: None,
                body: None,
                status: None,
                labels: None,
                due_at: Some("2026-08-01T09:00:00Z".into()),
                clear_due: true,
            })
            .await
            .unwrap();
        assert!(cleared.due_at.is_none(), "clear_due overrides due_at");
    }
}
