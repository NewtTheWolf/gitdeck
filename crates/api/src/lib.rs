//! gRPC (tonic) + gRPC-Web (tonic-web) surface over the existing `TaskService`.
//!
//! `GitdeckService` implements the generated `Gitdeck` tonic trait, mapping every
//! RPC 1:1 onto a `TaskService` method. Build a `router(...)` to embed the server
//! in another process or serve it standalone via the `gitdeck-server` binary.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use newt_todo_auth::TokenStore;
use newt_todo_core::{AccountDraft, SyncChange, LOCAL_USER};
use tokio::sync::broadcast;
use newt_todo_service::{
    BoardIdParam, CreateBoardParams, CreateColumnParams, CreateTaskParams, IssueAssigneesParams,
    IssueCommentParams, IssueLabelsParams, ListTasksParams, PlaceCardParams, RemoveCardParams,
    RemoveLabelParams, RenameBoardParams, ReorderBoardsParams, ReorderColumnsParams,
    SetIssueStateParams, TaskIdParam, TaskService, UpdateColumnParams, UpdateTaskParams,
};
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("gitdeck.v1");
}

mod convert;

mod auth;
pub use auth::EnvTokenStore;

mod authstore;
pub use authstore::AuthStore;

mod jwt;
pub use jwt::JwtCodec;

mod interceptor;
pub use interceptor::{AuthContext, AuthLayer, AuthMode};

use proto::gitdeck_server::{Gitdeck, GitdeckServer};

/// Map an `anyhow::Error` to a sensible `tonic::Status`.
///
/// We can't downcast through `anyhow` reliably here, so classify by message:
/// "not found" → `not_found`, "no token"/"unauthor" → `unauthenticated`,
/// everything else → `internal`. The full message is always preserved.
fn to_status(e: anyhow::Error) -> Status {
    let msg = e.to_string();
    let lower = msg.to_lowercase();
    if lower.contains("not found") {
        Status::not_found(msg)
    } else if lower.contains("no token") || lower.contains("unauthor") || lower.contains("token") {
        Status::unauthenticated(msg)
    } else {
        Status::internal(msg)
    }
}

/// The tonic service. Holds a shared `TaskService` plus the headless token store
/// used by every GitHub-backed RPC.
///
/// In multi-user mode it ALSO holds the server-side [`AuthStore`] (users +
/// per-user GitHub tokens) and the [`JwtCodec`] used to issue session tokens.
/// In single-user mode (the default, backwards-compatible) those are `None` and
/// the service behaves exactly as before (using `token_store` / `EnvTokenStore`).
pub struct GitdeckService {
    service: Arc<TaskService>,
    token_store: Arc<dyn TokenStore>,
    auth_store: Option<Arc<AuthStore>>,
    jwt: Option<JwtCodec>,
    /// Per-user fan-out hub for `WatchChanges`: each `PushChanges` broadcasts the
    /// rows it APPLIED to every live watcher of the same user (other sessions).
    watch_hub: Arc<SyncWatchHub>,
}

/// A per-user broadcast hub keyed by `user_id`. Senders are created lazily on
/// first subscribe/publish and live for the process lifetime.
#[derive(Default)]
struct SyncWatchHub {
    senders: Mutex<HashMap<String, broadcast::Sender<proto::Change>>>,
}

impl SyncWatchHub {
    /// Get (or create) the sender for a user.
    fn sender(&self, user_id: &str) -> broadcast::Sender<proto::Change> {
        let mut map = self.senders.lock().expect("watch hub poisoned");
        map.entry(user_id.to_string())
            .or_insert_with(|| broadcast::channel(256).0)
            .clone()
    }

    /// Subscribe a new watcher for a user.
    fn subscribe(&self, user_id: &str) -> broadcast::Receiver<proto::Change> {
        self.sender(user_id).subscribe()
    }

    /// Publish an applied change to a user's watchers. A send error (no live
    /// receivers) is ignored.
    fn publish(&self, user_id: &str, change: proto::Change) {
        let _ = self.sender(user_id).send(change);
    }
}

impl GitdeckService {
    /// Single-user construction (BC): no auth store, no JWT.
    pub fn new(service: Arc<TaskService>, token_store: Arc<dyn TokenStore>) -> Self {
        Self {
            service,
            token_store,
            auth_store: None,
            jwt: None,
            watch_hub: Arc::new(SyncWatchHub::default()),
        }
    }

    /// Multi-user construction: wire the server auth store + JWT codec.
    pub fn with_auth(
        service: Arc<TaskService>,
        token_store: Arc<dyn TokenStore>,
        auth_store: Arc<AuthStore>,
        jwt: JwtCodec,
    ) -> Self {
        Self {
            service,
            token_store,
            auth_store: Some(auth_store),
            jwt: Some(jwt),
            watch_hub: Arc::new(SyncWatchHub::default()),
        }
    }

    /// The authenticated user id for this request, read from extensions injected
    /// by [`AuthLayer`]. `None` in single-user mode (no auth context present).
    fn user_id(req_ext: &tonic::Extensions) -> Option<String> {
        req_ext.get::<AuthContext>().and_then(|c| c.user_id.clone())
    }

    /// The user id to scope a data RPC by: the authed user (multi mode) or
    /// `LOCAL_USER` (single mode / backwards-compatible, no auth context). This
    /// is what every board/column/card/task RPC threads into the service `*_for`
    /// variants so core scopes (and ownership-gates) the row by the caller.
    fn user_of(req_ext: &tonic::Extensions) -> String {
        Self::user_id(req_ext).unwrap_or_else(|| LOCAL_USER.to_string())
    }

    /// Resolve the `TokenStore` to use for a GitHub-backed RPC.
    ///
    /// In multi-user mode, if the request carries an authenticated user with a
    /// stored GitHub token, return a per-request store yielding THAT token.
    /// Otherwise (single-user mode, or no per-user token set) fall back to the
    /// default `token_store` (`EnvTokenStore` in the standalone server).
    async fn resolve_token_store(&self, req_ext: &tonic::Extensions) -> Arc<dyn TokenStore> {
        if let (Some(auth_store), Some(user_id)) =
            (self.auth_store.as_ref(), Self::user_id(req_ext))
        {
            if let Ok(Some(token)) = auth_store.get_github_token(&user_id).await {
                return Arc::new(EnvTokenStore::new(Some(token)));
            }
        }
        self.token_store.clone()
    }
}

#[tonic::async_trait]
impl Gitdeck for GitdeckService {
    // --- auth (server multi-user; Phase K1) ----------------------------------

    async fn register(
        &self,
        req: Request<proto::RegisterRequest>,
    ) -> Result<Response<proto::RegisterResponse>, Status> {
        let auth = self
            .auth_store
            .as_ref()
            .ok_or_else(|| Status::unimplemented("auth disabled (GITDECK_AUTH=single)"))?;
        let r = req.into_inner();
        let user_id = auth
            .register(&r.username, &r.password)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::RegisterResponse { user_id }))
    }

    async fn login(
        &self,
        req: Request<proto::LoginRequest>,
    ) -> Result<Response<proto::LoginResponse>, Status> {
        let auth = self
            .auth_store
            .as_ref()
            .ok_or_else(|| Status::unimplemented("auth disabled (GITDECK_AUTH=single)"))?;
        let jwt = self
            .jwt
            .as_ref()
            .ok_or_else(|| Status::internal("jwt codec not configured"))?;
        let r = req.into_inner();
        let user_id = auth
            .verify_login(&r.username, &r.password)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::unauthenticated("invalid username or password"))?;
        let token = jwt
            .issue(&user_id)
            .map_err(|e| Status::internal(format!("issue token: {e}")))?;
        Ok(Response::new(proto::LoginResponse { token }))
    }

    async fn set_github_token(
        &self,
        req: Request<proto::SetGithubTokenRequest>,
    ) -> Result<Response<proto::SetGithubTokenResponse>, Status> {
        let auth = self
            .auth_store
            .as_ref()
            .ok_or_else(|| Status::unimplemented("auth disabled (GITDECK_AUTH=single)"))?;
        let user_id = Self::user_id(req.extensions())
            .ok_or_else(|| Status::unauthenticated("missing or invalid bearer token"))?;
        let r = req.into_inner();
        auth.set_github_token(&user_id, &r.github_token)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::SetGithubTokenResponse {}))
    }

    async fn get_me(
        &self,
        req: Request<proto::GetMeRequest>,
    ) -> Result<Response<proto::GetMeResponse>, Status> {
        let auth = self
            .auth_store
            .as_ref()
            .ok_or_else(|| Status::unimplemented("auth disabled (GITDECK_AUTH=single)"))?;
        let user_id = Self::user_id(req.extensions())
            .ok_or_else(|| Status::unauthenticated("missing or invalid bearer token"))?;
        let user = auth
            .get_user(&user_id)
            .await
            .map_err(to_status)?
            .ok_or_else(|| Status::not_found("user not found"))?;
        let has_github_token = auth
            .get_github_token(&user_id)
            .await
            .map_err(to_status)?
            .is_some();
        Ok(Response::new(proto::GetMeResponse {
            user_id: user.id,
            username: user.username,
            has_github_token,
        }))
    }

    // --- local todos ---------------------------------------------------------

    async fn create_task(
        &self,
        req: Request<proto::CreateTaskRequest>,
    ) -> Result<Response<proto::TaskResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let task = self
            .service
            .create_for(&user, CreateTaskParams {
                title: r.title,
                body: r.body,
                labels: if r.labels.is_empty() {
                    None
                } else {
                    Some(r.labels)
                },
                due_at: r.due_at,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::TaskResponse {
            task: Some(task.into()),
        }))
    }

    async fn list_tasks(
        &self,
        req: Request<proto::ListTasksRequest>,
    ) -> Result<Response<proto::ListTasksResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let tasks = self
            .service
            .list_for(&user, ListTasksParams {
                status: r.status,
                label: r.label,
                query: r.query,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListTasksResponse {
            tasks: tasks.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_task(
        &self,
        req: Request<proto::GetTaskRequest>,
    ) -> Result<Response<proto::GetTaskResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let task = self
            .service
            .get_for(&user, &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetTaskResponse {
            task: task.map(Into::into),
        }))
    }

    async fn update_task(
        &self,
        req: Request<proto::UpdateTaskRequest>,
    ) -> Result<Response<proto::TaskResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let task = self
            .service
            .update_for(&user, UpdateTaskParams {
                id: r.id,
                title: r.title,
                body: r.body,
                status: r.status,
                labels: if r.set_labels { Some(r.labels) } else { None },
                due_at: r.due_at,
                clear_due: r.clear_due,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::TaskResponse {
            task: Some(task.into()),
        }))
    }

    async fn complete_task(
        &self,
        req: Request<proto::TaskIdRequest>,
    ) -> Result<Response<proto::TaskResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let task = self
            .service
            .complete_for(&user, TaskIdParam {
                id: req.into_inner().id,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::TaskResponse {
            task: Some(task.into()),
        }))
    }

    async fn delete_task(
        &self,
        req: Request<proto::TaskIdRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let user = Self::user_of(req.extensions());
        self.service
            .delete_for(&user, TaskIdParam {
                id: req.into_inner().id,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // --- accounts ------------------------------------------------------------

    async fn list_accounts(
        &self,
        _req: Request<proto::Empty>,
    ) -> Result<Response<proto::ListAccountsResponse>, Status> {
        let accounts = self.service.list_accounts().await.map_err(to_status)?;
        Ok(Response::new(proto::ListAccountsResponse {
            accounts: accounts.into_iter().map(Into::into).collect(),
        }))
    }

    async fn create_account(
        &self,
        req: Request<proto::CreateAccountRequest>,
    ) -> Result<Response<proto::AccountMessage>, Status> {
        let r = req.into_inner();
        let provider = newt_todo_core::ProviderKind::parse(&r.provider)
            .ok_or_else(|| Status::invalid_argument(format!("unknown provider: {}", r.provider)))?;
        let config: serde_json::Value = serde_json::from_str(&r.config_json)
            .map_err(|e| Status::invalid_argument(format!("invalid config_json: {e}")))?;
        let account = self
            .service
            .create_account(AccountDraft {
                provider,
                display_name: r.display_name,
                base_url: r.base_url,
                config,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(account.into()))
    }

    async fn delete_account(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        self.service
            .delete_account(&req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // --- sync ----------------------------------------------------------------

    async fn sync_account(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::SyncReportMessage>, Status> {
        // Per-user GitHub token resolution (K1): in multi mode this uses the
        // authed user's stored token; otherwise it falls back to `token_store`.
        let token_store = self.resolve_token_store(req.extensions()).await;
        let report = self
            .service
            .sync(token_store.as_ref(), &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(report.into()))
    }

    // --- dashboard reads -----------------------------------------------------

    async fn list_repos(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::ListReposResponse>, Status> {
        // Per-user GitHub token resolution (K1) — same as `sync_account`.
        let token_store = self.resolve_token_store(req.extensions()).await;
        let repos = self
            .service
            .list_repos(token_store.as_ref(), &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListReposResponse {
            repos: repos.into_iter().map(Into::into).collect(),
        }))
    }

    // All GitHub-backed RPCs below resolve the per-request token store via
    // `resolve_token_store(req.extensions())` (K2): multi mode uses the authed
    // user's stored GitHub token; single mode falls back to `self.token_store`
    // (`EnvTokenStore`). (`list_snapshots`/`daily_digest` read only the local
    // store and so don't need a token.)

    async fn list_issues(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::ListIssuesResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let issues = self
            .service
            .list_issues(token_store.as_ref(), &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListIssuesResponse {
            issues: issues.into_iter().map(Into::into).collect(),
        }))
    }

    async fn list_pull_requests(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::ListPullRequestsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let prs = self
            .service
            .list_pull_requests(token_store.as_ref(), &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListPullRequestsResponse {
            pull_requests: prs.into_iter().map(Into::into).collect(),
        }))
    }

    // --- boards --------------------------------------------------------------

    async fn list_boards(
        &self,
        req: Request<proto::Empty>,
    ) -> Result<Response<proto::ListBoardsResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let boards = self
            .service
            .list_boards_for(&user)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListBoardsResponse {
            boards: boards.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_board(
        &self,
        req: Request<proto::BoardIdRequest>,
    ) -> Result<Response<proto::GetBoardResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let board = self
            .service
            .get_board_for(&user, BoardIdParam {
                id: req.into_inner().id,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetBoardResponse {
            board: board.map(Into::into),
        }))
    }

    async fn create_board(
        &self,
        req: Request<proto::CreateBoardRequest>,
    ) -> Result<Response<proto::BoardMessage>, Status> {
        let user = Self::user_of(req.extensions());
        let board = self
            .service
            .create_board_for(&user, CreateBoardParams {
                name: req.into_inner().name,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::BoardMessage {
            board: Some(board.into()),
        }))
    }

    async fn rename_board(
        &self,
        req: Request<proto::RenameBoardRequest>,
    ) -> Result<Response<proto::BoardMessage>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let board = self
            .service
            .rename_board_for(&user, RenameBoardParams {
                id: r.id,
                name: r.name,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::BoardMessage {
            board: Some(board.into()),
        }))
    }

    async fn delete_board(
        &self,
        req: Request<proto::BoardIdRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let user = Self::user_of(req.extensions());
        self.service
            .delete_board_for(&user, BoardIdParam {
                id: req.into_inner().id,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn reorder_boards(
        &self,
        req: Request<proto::ReorderBoardsRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let user = Self::user_of(req.extensions());
        self.service
            .reorder_boards_for(&user, ReorderBoardsParams {
                ordered_ids: req.into_inner().ordered_ids,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // --- columns -------------------------------------------------------------

    async fn create_column(
        &self,
        req: Request<proto::CreateColumnRequest>,
    ) -> Result<Response<proto::ColumnMessage>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let filter = parse_optional_json(r.filter_json)?;
        let column = self
            .service
            .create_column_for(&user, CreateColumnParams {
                board_id: r.board_id,
                name: r.name,
                filter,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ColumnMessage {
            column: Some(column.into()),
        }))
    }

    async fn update_column(
        &self,
        req: Request<proto::UpdateColumnRequest>,
    ) -> Result<Response<proto::ColumnMessage>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let filter = parse_optional_json(r.filter_json)?;
        let column = self
            .service
            .update_column_for(&user, UpdateColumnParams {
                id: r.id,
                name: r.name,
                filter,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ColumnMessage {
            column: Some(column.into()),
        }))
    }

    async fn delete_column(
        &self,
        req: Request<proto::ColumnIdRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let user = Self::user_of(req.extensions());
        self.service
            .delete_column_for(&user, &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn reorder_columns(
        &self,
        req: Request<proto::ReorderColumnsRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        self.service
            .reorder_columns_for(&user, ReorderColumnsParams {
                board_id: r.board_id,
                ordered_ids: r.ordered_ids,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // --- cards ---------------------------------------------------------------

    async fn place_card(
        &self,
        req: Request<proto::PlaceCardRequest>,
    ) -> Result<Response<proto::CardMessage>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        let card = self
            .service
            .place_card_for(&user, PlaceCardParams {
                board_id: r.board_id,
                column_id: r.column_id,
                item_key: r.item_key,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::CardMessage {
            card: Some(card.into()),
        }))
    }

    async fn remove_card(
        &self,
        req: Request<proto::RemoveCardRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let user = Self::user_of(req.extensions());
        let r = req.into_inner();
        self.service
            .remove_card_for(&user, RemoveCardParams {
                board_id: r.board_id,
                item_key: r.item_key,
            })
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // --- notifications -------------------------------------------------------

    async fn list_notifications(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::ListNotificationsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let notifications = self
            .service
            .list_notifications(token_store.as_ref(), &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListNotificationsResponse {
            notifications: notifications.into_iter().map(Into::into).collect(),
        }))
    }

    async fn mark_notification_read(
        &self,
        req: Request<proto::MarkNotificationReadRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        self.service
            .mark_notification_read(token_store.as_ref(), &r.id, &r.thread_id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    // --- CI workflow runs ----------------------------------------------------

    async fn list_workflow_runs(
        &self,
        req: Request<proto::ListWorkflowRunsRequest>,
    ) -> Result<Response<proto::ListWorkflowRunsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let runs = self
            .service
            .list_workflow_runs(
                token_store.as_ref(),
                &r.id,
                &r.owner,
                &r.repo,
                r.per_page.unwrap_or(20),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListWorkflowRunsResponse {
            runs: runs.into_iter().map(Into::into).collect(),
        }))
    }

    // --- repository detail ---------------------------------------------------

    async fn get_repo_detail(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::RepoDetailMessage>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let detail = self
            .service
            .get_repo_detail(token_store.as_ref(), &r.id, &r.owner, &r.repo)
            .await
            .map_err(to_status)?;
        Ok(Response::new(detail.into()))
    }

    async fn list_releases(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::ListReleasesResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let releases = self
            .service
            .list_releases(
                token_store.as_ref(),
                &r.id,
                &r.owner,
                &r.repo,
                r.per_page.unwrap_or(30),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListReleasesResponse {
            releases: releases.into_iter().map(Into::into).collect(),
        }))
    }

    async fn list_forks(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::ListForksResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let forks = self
            .service
            .list_forks(
                token_store.as_ref(),
                &r.id,
                &r.owner,
                &r.repo,
                r.per_page.unwrap_or(30),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListForksResponse {
            forks: forks.into_iter().map(Into::into).collect(),
        }))
    }

    async fn list_contributors(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::ListContributorsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let contributors = self
            .service
            .list_contributors(
                token_store.as_ref(),
                &r.id,
                &r.owner,
                &r.repo,
                r.per_page.unwrap_or(30),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListContributorsResponse {
            contributors: contributors.into_iter().map(Into::into).collect(),
        }))
    }

    async fn get_languages(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::GetLanguagesResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let langs = self
            .service
            .get_languages(token_store.as_ref(), &r.id, &r.owner, &r.repo)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetLanguagesResponse {
            languages: langs.into_iter().map(Into::into).collect(),
        }))
    }

    // --- traffic -------------------------------------------------------------

    async fn get_traffic_views(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::GetTrafficResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let traffic = self
            .service
            .get_traffic_views(token_store.as_ref(), &r.id, &r.owner, &r.repo)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetTrafficResponse {
            traffic: traffic.map(Into::into),
        }))
    }

    async fn get_traffic_clones(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::GetTrafficResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let traffic = self
            .service
            .get_traffic_clones(token_store.as_ref(), &r.id, &r.owner, &r.repo)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetTrafficResponse {
            traffic: traffic.map(Into::into),
        }))
    }

    async fn list_referrers(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::ListReferrersResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let referrers = self
            .service
            .list_referrers(token_store.as_ref(), &r.id, &r.owner, &r.repo)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListReferrersResponse {
            referrers: referrers.into_iter().map(Into::into).collect(),
        }))
    }

    async fn list_paths(
        &self,
        req: Request<proto::RepoRefRequest>,
    ) -> Result<Response<proto::ListPathsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let paths = self
            .service
            .list_paths(token_store.as_ref(), &r.id, &r.owner, &r.repo)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListPathsResponse {
            paths: paths.into_iter().map(Into::into).collect(),
        }))
    }

    // --- mentions / search ---------------------------------------------------

    async fn search_code(
        &self,
        req: Request<proto::SearchRequest>,
    ) -> Result<Response<proto::SearchCodeResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let hits = self
            .service
            .search_code(
                token_store.as_ref(),
                &r.id,
                &r.query,
                r.per_page.unwrap_or(20),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::SearchCodeResponse {
            hits: hits.into_iter().map(Into::into).collect(),
        }))
    }

    async fn search_issues(
        &self,
        req: Request<proto::SearchRequest>,
    ) -> Result<Response<proto::ListIssuesResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let issues = self
            .service
            .search_issues(
                token_store.as_ref(),
                &r.id,
                &r.query,
                r.per_page.unwrap_or(20),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListIssuesResponse {
            issues: issues.into_iter().map(Into::into).collect(),
        }))
    }

    // --- in-app issue/PR detail ----------------------------------------------

    async fn get_issue(
        &self,
        req: Request<proto::IssueRefRequest>,
    ) -> Result<Response<proto::GetIssueResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let detail = self
            .service
            .get_issue(token_store.as_ref(), &r.id, &r.owner, &r.repo, r.number)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetIssueResponse {
            issue: detail.map(Into::into),
        }))
    }

    async fn get_pull_request(
        &self,
        req: Request<proto::IssueRefRequest>,
    ) -> Result<Response<proto::GetPullRequestResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let detail = self
            .service
            .get_pull_request(token_store.as_ref(), &r.id, &r.owner, &r.repo, r.number)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::GetPullRequestResponse {
            pull_request: detail.map(Into::into),
        }))
    }

    async fn list_issue_comments(
        &self,
        req: Request<proto::IssueRefRequest>,
    ) -> Result<Response<proto::ListCommentsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let comments = self
            .service
            .list_issue_comments(
                token_store.as_ref(),
                &r.id,
                &r.owner,
                &r.repo,
                r.number,
                r.per_page.unwrap_or(100),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListCommentsResponse {
            comments: comments.into_iter().map(Into::into).collect(),
        }))
    }

    async fn list_pull_reviews(
        &self,
        req: Request<proto::IssueRefRequest>,
    ) -> Result<Response<proto::ListReviewsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let reviews = self
            .service
            .list_pull_reviews(
                token_store.as_ref(),
                &r.id,
                &r.owner,
                &r.repo,
                r.number,
                r.per_page.unwrap_or(100),
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListReviewsResponse {
            reviews: reviews.into_iter().map(Into::into).collect(),
        }))
    }

    // --- triage write actions ------------------------------------------------

    async fn set_issue_state(
        &self,
        req: Request<proto::SetIssueStateRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        self.service
            .set_issue_state(
                token_store.as_ref(),
                SetIssueStateParams {
                    id: r.id,
                    owner: r.owner,
                    repo: r.repo,
                    number: r.number,
                    state: r.state,
                },
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn add_issue_labels(
        &self,
        req: Request<proto::IssueLabelsRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        self.service
            .add_issue_labels(
                token_store.as_ref(),
                IssueLabelsParams {
                    id: r.id,
                    owner: r.owner,
                    repo: r.repo,
                    number: r.number,
                    labels: r.labels,
                },
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn remove_issue_label(
        &self,
        req: Request<proto::RemoveLabelRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        self.service
            .remove_issue_label(
                token_store.as_ref(),
                RemoveLabelParams {
                    id: r.id,
                    owner: r.owner,
                    repo: r.repo,
                    number: r.number,
                    label: r.label,
                },
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn add_issue_assignees(
        &self,
        req: Request<proto::IssueAssigneesRequest>,
    ) -> Result<Response<proto::Empty>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        self.service
            .add_issue_assignees(
                token_store.as_ref(),
                IssueAssigneesParams {
                    id: r.id,
                    owner: r.owner,
                    repo: r.repo,
                    number: r.number,
                    assignees: r.assignees,
                },
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::Empty {}))
    }

    async fn create_issue_comment(
        &self,
        req: Request<proto::IssueCommentRequest>,
    ) -> Result<Response<proto::CreateIssueCommentResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let r = req.into_inner();
        let html_url = self
            .service
            .create_issue_comment(
                token_store.as_ref(),
                IssueCommentParams {
                    id: r.id,
                    owner: r.owner,
                    repo: r.repo,
                    number: r.number,
                    body: r.body,
                },
            )
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::CreateIssueCommentResponse { html_url }))
    }

    // --- daily repo snapshots ------------------------------------------------

    async fn capture_snapshots(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::CaptureSnapshotsResponse>, Status> {
        let token_store = self.resolve_token_store(req.extensions()).await;
        let n = self
            .service
            .capture_snapshots(token_store.as_ref(), &req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::CaptureSnapshotsResponse {
            captured: n as u64,
        }))
    }

    async fn list_snapshots(
        &self,
        req: Request<proto::ListSnapshotsRequest>,
    ) -> Result<Response<proto::ListSnapshotsResponse>, Status> {
        let r = req.into_inner();
        let snaps = self
            .service
            .list_snapshots(&r.id, r.since_day)
            .await
            .map_err(to_status)?;
        Ok(Response::new(proto::ListSnapshotsResponse {
            snapshots: snaps.into_iter().map(Into::into).collect(),
        }))
    }

    async fn daily_digest(
        &self,
        req: Request<proto::AccountIdRequest>,
    ) -> Result<Response<proto::DigestMessage>, Status> {
        let digest = self
            .service
            .daily_digest(&req.into_inner().id)
            .await
            .map_err(to_status)?;
        Ok(Response::new(digest.into()))
    }

    // --- cross-device sync (Phase K3) ----------------------------------------

    async fn push_changes(
        &self,
        req: Request<proto::PushChangesRequest>,
    ) -> Result<Response<proto::PushChangesResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let changes = req.into_inner().changes;
        let store = self.service.store();

        let mut applied = 0u32;
        for change in changes {
            let sc: SyncChange = change.into();
            let did = store
                .apply_change(&user, &sc)
                .await
                .map_err(|e| to_status(e.into()))?;
            if did {
                applied += 1;
                // Fan out the applied change to this user's other watchers.
                self.watch_hub.publish(&user, sc.into());
            }
        }

        let server_cursor = store
            .max_cursor(&user)
            .await
            .map_err(|e| to_status(e.into()))?
            .unwrap_or_default();

        Ok(Response::new(proto::PushChangesResponse {
            applied,
            server_cursor,
        }))
    }

    async fn pull_changes(
        &self,
        req: Request<proto::PullChangesRequest>,
    ) -> Result<Response<proto::PullChangesResponse>, Status> {
        let user = Self::user_of(req.extensions());
        let since = req.into_inner().since_cursor;
        let cursor = if since.is_empty() { None } else { Some(since.as_str()) };

        let changes = self
            .service
            .store()
            .changes_since(&user, cursor)
            .await
            .map_err(|e| to_status(e.into()))?;

        // New cursor = max updated_at returned, else echo the incoming cursor.
        let new_cursor = changes
            .iter()
            .map(|c| c.updated_at.clone())
            .max()
            .unwrap_or(since);

        Ok(Response::new(proto::PullChangesResponse {
            changes: changes.into_iter().map(Into::into).collect(),
            cursor: new_cursor,
        }))
    }

    type WatchChangesStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = Result<proto::Change, Status>> + Send>,
    >;

    async fn watch_changes(
        &self,
        req: Request<proto::WatchChangesRequest>,
    ) -> Result<Response<Self::WatchChangesStream>, Status> {
        let user = Self::user_of(req.extensions());
        use tokio_stream::StreamExt;
        let rx = self.watch_hub.subscribe(&user);
        // Map the broadcast receiver to a gRPC stream; drop lagged-skip errors so
        // a slow client just misses intermediate changes (it can re-pull).
        let stream = tokio_stream::wrappers::BroadcastStream::new(rx)
            .filter_map(|res| res.ok().map(Ok));
        Ok(Response::new(Box::pin(stream)))
    }
}

/// Parse an optional JSON-string filter into a `serde_json::Value`. `None` or
/// empty → `None` (the service defaults to `{}`); a non-empty string must parse.
// `tonic::Status` is intentionally the error type here to match the RPC handlers
// that call this; the large-Err lint is inherent to tonic's Result shape.
#[allow(clippy::result_large_err)]
fn parse_optional_json(s: Option<String>) -> Result<Option<serde_json::Value>, Status> {
    match s {
        None => Ok(None),
        Some(s) if s.is_empty() => Ok(None),
        Some(s) => serde_json::from_str(&s)
            .map(Some)
            .map_err(|e| Status::invalid_argument(format!("invalid filter_json: {e}"))),
    }
}

/// Optional bearer-API-key interceptor. When an api key is configured, every
/// request must carry `authorization: Bearer <key>`; otherwise it's rejected
/// with `unauthenticated`. When no key is configured the request passes through.
#[derive(Clone)]
pub struct ApiKeyInterceptor {
    api_key: Option<Arc<String>>,
}

impl ApiKeyInterceptor {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key: api_key.map(Arc::new),
        }
    }
}

impl tonic::service::Interceptor for ApiKeyInterceptor {
    fn call(&mut self, req: Request<()>) -> Result<Request<()>, Status> {
        let Some(expected) = &self.api_key else {
            return Ok(req);
        };
        let provided = req
            .metadata()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "));
        match provided {
            Some(token) if token == expected.as_str() => Ok(req),
            _ => Err(Status::unauthenticated("missing or invalid API key")),
        }
    }
}

/// The configured tonic `GitdeckServer` wrapped with the optional API-key
/// interceptor. Use [`router`] to wrap this with gRPC-Web + CORS for serving.
pub type ConfiguredGitdeckServer = tonic::codegen::InterceptedService<
    GitdeckServer<GitdeckService>,
    ApiKeyInterceptor,
>;

/// Build the configured gRPC server (with the optional API-key interceptor).
pub fn gitdeck_server(
    service: Arc<TaskService>,
    token_store: Arc<dyn TokenStore>,
    api_key: Option<String>,
) -> ConfiguredGitdeckServer {
    GitdeckServer::with_interceptor(
        GitdeckService::new(service, token_store),
        ApiKeyInterceptor::new(api_key),
    )
}

/// Build a `tonic::transport::server::Router` wrapping the Gitdeck gRPC service
/// with gRPC-Web support (so a webview/browser gRPC-Web client can call it) and
/// a permissive CORS layer. Embed it or serve it standalone.
pub fn router(
    service: Arc<TaskService>,
    token_store: Arc<dyn TokenStore>,
    api_key: Option<String>,
) -> tonic::transport::server::Router<tower_layer::Stack<
    tower_http::cors::CorsLayer,
    tower_layer::Stack<tonic_web::GrpcWebLayer, tower::layer::util::Identity>,
>> {
    tonic::transport::Server::builder()
        .accept_http1(true)
        .layer(tonic_web::GrpcWebLayer::new())
        .layer(permissive_cors())
        .add_service(gitdeck_server(service, token_store, api_key))
}

// ============================================================================
// Multi-user (Phase K1) builders
// ============================================================================

/// Bundle of server-side multi-user auth pieces, passed to the `*_with_auth`
/// builders. In single mode you can still use these builders with `mode =
/// AuthMode::Single` and `auth_store/jwt = None` (fully BC).
pub struct AuthSetup {
    pub mode: AuthMode,
    pub auth_store: Option<Arc<AuthStore>>,
    pub jwt: Option<JwtCodec>,
    pub api_key: Option<String>,
}

impl AuthSetup {
    /// Single-user setup (BC): no auth store, no JWT, optional API key.
    pub fn single(api_key: Option<String>) -> Self {
        Self {
            mode: AuthMode::Single,
            auth_store: None,
            jwt: None,
            api_key,
        }
    }

    /// Multi-user setup: server auth store + JWT codec (+ optional API key).
    pub fn multi(auth_store: Arc<AuthStore>, jwt: JwtCodec, api_key: Option<String>) -> Self {
        Self {
            mode: AuthMode::Multi,
            auth_store: Some(auth_store),
            jwt: Some(jwt),
            api_key,
        }
    }
}

/// Build the `GitdeckServer` for the given auth setup. Unlike [`gitdeck_server`]
/// this does NOT wrap a per-request `Interceptor`; api-key + JWT enforcement is
/// done by the [`AuthLayer`] applied in [`router_with_auth`] (which can see the
/// gRPC method path and so exempt `Register`/`Login`).
fn gitdeck_server_inner(
    service: Arc<TaskService>,
    token_store: Arc<dyn TokenStore>,
    setup: &AuthSetup,
) -> GitdeckServer<GitdeckService> {
    let svc = match (setup.auth_store.clone(), setup.jwt.clone()) {
        (Some(auth_store), Some(jwt)) => {
            GitdeckService::with_auth(service, token_store, auth_store, jwt)
        }
        _ => GitdeckService::new(service, token_store),
    };
    GitdeckServer::new(svc)
}

/// Multi-user-aware [`router`]. Applies the [`AuthLayer`] (single = BC no-op +
/// optional API key; multi = JWT-gated, user id injected) under gRPC-Web + CORS.
// The return type spells out the tower layer stack (same pattern as `router`).
#[allow(clippy::type_complexity)]
pub fn router_with_auth(
    service: Arc<TaskService>,
    token_store: Arc<dyn TokenStore>,
    setup: AuthSetup,
) -> tonic::transport::server::Router<
    tower_layer::Stack<
        tower_http::cors::CorsLayer,
        tower_layer::Stack<
            tonic_web::GrpcWebLayer,
            tower_layer::Stack<AuthLayer, tower::layer::util::Identity>,
        >,
    >,
> {
    let auth_layer = AuthLayer::new(setup.mode, setup.jwt.clone(), setup.api_key.clone());
    let server = gitdeck_server_inner(service, token_store, &setup);
    tonic::transport::Server::builder()
        .accept_http1(true)
        .layer(auth_layer)
        .layer(tonic_web::GrpcWebLayer::new())
        .layer(permissive_cors())
        .add_service(server)
}

/// Resolve a `GITDECK_DB` setting to a SeaORM DSN. A value containing `://`
/// (e.g. `postgres://...` or `sqlite://...`) is used verbatim; otherwise it's
/// treated as a SQLite file path. Shared by the core `Store` and the server's
/// own [`AuthStore`] so both connect to the same database.
pub fn db_url_from_setting(setting: &str) -> String {
    if setting.contains("://") {
        setting.to_string()
    } else {
        newt_todo_service::db_url_for_path(std::path::Path::new(setting))
    }
}

/// A permissive CORS layer suitable for a webview/browser gRPC-Web client.
fn permissive_cors() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};
    CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any)
        .expose_headers(Any)
}
