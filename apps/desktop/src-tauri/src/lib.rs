use newt_todo_auth::{
    build_authorize_url, exchange_code, AuthRequest, KeyringTokenStore, TokenRequest, TokenStore,
};
use newt_todo_core::{
    Account, AccountDraft, ProviderKind, RemoteIssue, RemotePullRequest, RemoteRepo, SyncReport,
};
use newt_todo_service::{
    BoardDto, BoardIdParam, CardDto, CodeHitDto, ColumnDto, CommentDto, ContributorDto,
    CreateBoardParams, CreateColumnParams, CreateTaskParams, DigestDto, ForkDto,
    IssueAssigneesParams, IssueCommentParams, IssueDetailDto, IssueLabelsParams, IssueRefParams,
    LanguageDto, ListSnapshotsParams, ListTasksParams, ListWorkflowRunsParams,
    MarkNotificationReadParams, NotificationDto, PathDto, PlaceCardParams, PullRequestDetailDto,
    ReferrerDto, ReleaseDto, RemoveCardParams, RemoveLabelParams, RenameBoardParams, RepoDetailDto,
    RepoRefParams, ReorderBoardsParams, ReorderColumnsParams, ReviewDto, SearchParams,
    SetIssueStateParams, SnapshotDto, TaskDto, TaskIdParam, TaskService, TrafficDto,
    UpdateColumnParams, UpdateTaskParams, WorkflowRunDto,
};
use tauri::State;

const GITHUB_AUTHORIZE_ENDPOINT: &str = "https://github.com/login/oauth/authorize";
const GITHUB_TOKEN_ENDPOINT: &str = "https://github.com/login/oauth/access_token";
const GITHUB_USER_ENDPOINT: &str = "https://api.github.com/user";
/// Fixed loopback port — must match the OAuth app's registered callback URL
/// (`http://127.0.0.1:8788/callback`). GitHub matches host+port exactly.
const GITHUB_OAUTH_PORT: u16 = 8788;

struct AppState {
    service: TaskService,
    token_store: KeyringTokenStore,
}

#[tauri::command]
async fn list_tasks(
    params: ListTasksParams,
    state: State<'_, AppState>,
) -> Result<Vec<TaskDto>, String> {
    state.service.list(params).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_task(
    params: CreateTaskParams,
    state: State<'_, AppState>,
) -> Result<TaskDto, String> {
    state
        .service
        .create(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_task(
    params: UpdateTaskParams,
    state: State<'_, AppState>,
) -> Result<TaskDto, String> {
    state
        .service
        .update(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn complete_task(id: String, state: State<'_, AppState>) -> Result<TaskDto, String> {
    state
        .service
        .complete(TaskIdParam { id })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_task(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .service
        .delete(TaskIdParam { id })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, String> {
    state
        .service
        .list_accounts()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_account(
    draft: AccountDraft,
    state: State<'_, AppState>,
) -> Result<Account, String> {
    state
        .service
        .create_account(draft)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_account(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .service
        .delete_account(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn sync_account(id: String, state: State<'_, AppState>) -> Result<SyncReport, String> {
    state
        .service
        .sync(&state.token_store, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_repos(id: String, state: State<'_, AppState>) -> Result<Vec<RemoteRepo>, String> {
    state
        .service
        .list_repos(&state.token_store, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_issues(id: String, state: State<'_, AppState>) -> Result<Vec<RemoteIssue>, String> {
    state
        .service
        .list_issues(&state.token_store, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_pull_requests(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<RemotePullRequest>, String> {
    state
        .service
        .list_pull_requests(&state.token_store, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_code(
    params: SearchParams,
    state: State<'_, AppState>,
) -> Result<Vec<CodeHitDto>, String> {
    let per_page = params.per_page.unwrap_or(20);
    state
        .service
        .search_code(&state.token_store, &params.id, &params.query, per_page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn search_issues(
    params: SearchParams,
    state: State<'_, AppState>,
) -> Result<Vec<RemoteIssue>, String> {
    let per_page = params.per_page.unwrap_or(20);
    state
        .service
        .search_issues(&state.token_store, &params.id, &params.query, per_page)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_notifications(
    id: String,
    state: State<'_, AppState>,
) -> Result<Vec<NotificationDto>, String> {
    state
        .service
        .list_notifications(&state.token_store, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn mark_notification_read(
    params: MarkNotificationReadParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .mark_notification_read(&state.token_store, &params.id, &params.thread_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_workflow_runs(
    params: ListWorkflowRunsParams,
    state: State<'_, AppState>,
) -> Result<Vec<WorkflowRunDto>, String> {
    state
        .service
        .list_workflow_runs(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.per_page.unwrap_or(20),
        )
        .await
        .map_err(|e| e.to_string())
}

// --- repository details -----------------------------------------------------

#[tauri::command]
async fn get_repo_detail(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<RepoDetailDto, String> {
    state
        .service
        .get_repo_detail(&state.token_store, &params.id, &params.owner, &params.repo)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_releases(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<ReleaseDto>, String> {
    state
        .service
        .list_releases(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.per_page.unwrap_or(30),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_forks(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<ForkDto>, String> {
    state
        .service
        .list_forks(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.per_page.unwrap_or(30),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_contributors(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<ContributorDto>, String> {
    state
        .service
        .list_contributors(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.per_page.unwrap_or(30),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_languages(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<LanguageDto>, String> {
    state
        .service
        .get_languages(&state.token_store, &params.id, &params.owner, &params.repo)
        .await
        .map_err(|e| e.to_string())
}

// --- traffic ----------------------------------------------------------------

#[tauri::command]
async fn get_traffic_views(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Option<TrafficDto>, String> {
    state
        .service
        .get_traffic_views(&state.token_store, &params.id, &params.owner, &params.repo)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_traffic_clones(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Option<TrafficDto>, String> {
    state
        .service
        .get_traffic_clones(&state.token_store, &params.id, &params.owner, &params.repo)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_referrers(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<ReferrerDto>, String> {
    state
        .service
        .list_referrers(&state.token_store, &params.id, &params.owner, &params.repo)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_paths(
    params: RepoRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<PathDto>, String> {
    state
        .service
        .list_paths(&state.token_store, &params.id, &params.owner, &params.repo)
        .await
        .map_err(|e| e.to_string())
}

// --- in-app issue/PR detail -------------------------------------------------

#[tauri::command]
async fn get_issue(
    params: IssueRefParams,
    state: State<'_, AppState>,
) -> Result<Option<IssueDetailDto>, String> {
    state
        .service
        .get_issue(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.number,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_pull_request(
    params: IssueRefParams,
    state: State<'_, AppState>,
) -> Result<Option<PullRequestDetailDto>, String> {
    state
        .service
        .get_pull_request(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.number,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_issue_comments(
    params: IssueRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<CommentDto>, String> {
    state
        .service
        .list_issue_comments(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.number,
            params.per_page.unwrap_or(100),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_pull_reviews(
    params: IssueRefParams,
    state: State<'_, AppState>,
) -> Result<Vec<ReviewDto>, String> {
    state
        .service
        .list_pull_reviews(
            &state.token_store,
            &params.id,
            &params.owner,
            &params.repo,
            params.number,
            params.per_page.unwrap_or(100),
        )
        .await
        .map_err(|e| e.to_string())
}

// --- triage write actions ---------------------------------------------------

#[tauri::command]
async fn set_issue_state(
    params: SetIssueStateParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .set_issue_state(&state.token_store, params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_issue_labels(
    params: IssueLabelsParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .add_issue_labels(&state.token_store, params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_issue_label(
    params: RemoveLabelParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .remove_issue_label(&state.token_store, params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_issue_assignees(
    params: IssueAssigneesParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .add_issue_assignees(&state.token_store, params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_issue_comment(
    params: IssueCommentParams,
    state: State<'_, AppState>,
) -> Result<String, String> {
    state
        .service
        .create_issue_comment(&state.token_store, params)
        .await
        .map_err(|e| e.to_string())
}

// --- daily repo snapshots ---------------------------------------------------

#[tauri::command]
async fn capture_snapshots(id: String, state: State<'_, AppState>) -> Result<usize, String> {
    state
        .service
        .capture_snapshots(&state.token_store, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_snapshots(
    params: ListSnapshotsParams,
    state: State<'_, AppState>,
) -> Result<Vec<SnapshotDto>, String> {
    state
        .service
        .list_snapshots(&params.id, params.since_day)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn daily_digest(id: String, state: State<'_, AppState>) -> Result<DigestDto, String> {
    state
        .service
        .daily_digest(&id)
        .await
        .map_err(|e| e.to_string())
}

// --- boards -----------------------------------------------------------------

#[tauri::command]
async fn list_boards(state: State<'_, AppState>) -> Result<Vec<BoardDto>, String> {
    state.service.list_boards().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_board(id: String, state: State<'_, AppState>) -> Result<Option<BoardDto>, String> {
    state
        .service
        .get_board(BoardIdParam { id })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_board(
    params: CreateBoardParams,
    state: State<'_, AppState>,
) -> Result<BoardDto, String> {
    state
        .service
        .create_board(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn rename_board(
    params: RenameBoardParams,
    state: State<'_, AppState>,
) -> Result<BoardDto, String> {
    state
        .service
        .rename_board(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_board(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .service
        .delete_board(BoardIdParam { id })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reorder_boards(
    params: ReorderBoardsParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .reorder_boards(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_column(
    params: CreateColumnParams,
    state: State<'_, AppState>,
) -> Result<ColumnDto, String> {
    state
        .service
        .create_column(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_column(
    params: UpdateColumnParams,
    state: State<'_, AppState>,
) -> Result<ColumnDto, String> {
    state
        .service
        .update_column(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_column(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .service
        .delete_column(&id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reorder_columns(
    params: ReorderColumnsParams,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .service
        .reorder_columns(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn place_card(
    params: PlaceCardParams,
    state: State<'_, AppState>,
) -> Result<CardDto, String> {
    state
        .service
        .place_card(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_card(params: RemoveCardParams, state: State<'_, AppState>) -> Result<(), String> {
    state
        .service
        .remove_card(params)
        .await
        .map_err(|e| e.to_string())
}

/// Parse the `code` and `state` query parameters out of an OAuth callback URL.
///
/// Returns `Err` with a descriptive message if the URL can't be parsed or the
/// expected parameters are missing. Kept as a free function so it stays unit-testable
/// without a running Tauri/WebView context.
fn parse_callback(callback_url: &str) -> Result<(String, String), String> {
    let parsed = url::Url::parse(callback_url).map_err(|e| format!("invalid callback url: {e}"))?;
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    for (key, value) in parsed.query_pairs() {
        match key.as_ref() {
            "code" => code = Some(value.into_owned()),
            "state" => state = Some(value.into_owned()),
            _ => {}
        }
    }
    let code = code.ok_or_else(|| "callback url missing `code` parameter".to_string())?;
    let state = state.ok_or_else(|| "callback url missing `state` parameter".to_string())?;
    Ok((code, state))
}

/// Interactive GitHub OAuth (Authorization-Code, no PKCE) via a loopback server.
///
/// RUNTIME-UNVERIFIED: requires a real GitHub OAuth app and a rendering WebView/browser,
/// neither available in CI. The orchestration logic is built to spec; the round-trip
/// must be verified by the user on their own machine.
#[tauri::command]
async fn start_github_login(
    state: State<'_, AppState>,
    client_id: String,
    client_secret: String,
) -> Result<Account, String> {
    // 1. Random `state` to defend against CSRF on the callback.
    let csrf_state = uuid::Uuid::new_v4().to_string();

    // 2. Start the loopback server on a FIXED port. GitHub requires the callback
    //    URL's host AND port to match exactly, so a random port can't be registered.
    //    The user registers `http://127.0.0.1:8788/callback` as the OAuth app callback.
    let (tx, rx) = tokio::sync::oneshot::channel::<String>();
    // The handler is `FnMut`, so the single-use `Sender` lives in an `Option` we `take`.
    let mut tx = Some(tx);
    let port = tauri_plugin_oauth::start_with_config(
        tauri_plugin_oauth::OauthConfig {
            ports: Some(vec![GITHUB_OAUTH_PORT]),
            ..Default::default()
        },
        move |url| {
            if let Some(tx) = tx.take() {
                // Receiver may have been dropped if the command already returned; ignore.
                let _ = tx.send(url);
            }
        },
    )
    .map_err(|e| format!("failed to start loopback server on port {GITHUB_OAUTH_PORT}: {e}"))?;

    // The plugin serves an HTML page on any path and reports the full original URL.
    // Path `/callback` must match the OAuth app's registered callback URL exactly.
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    // 3. Build the authorize URL (GitHub has no PKCE).
    let authorize_url = build_authorize_url(&AuthRequest {
        authorize_endpoint: GITHUB_AUTHORIZE_ENDPOINT,
        client_id: &client_id,
        redirect_uri: &redirect_uri,
        scope: "repo",
        state: &csrf_state,
        pkce_challenge: None,
    });

    // 4. Open the system browser at the authorize URL.
    if let Err(e) = open::that(&authorize_url) {
        let _ = tauri_plugin_oauth::cancel(port);
        return Err(format!("failed to open browser: {e}"));
    }

    // 5. Await the callback URL (with a timeout so an abandoned flow doesn't hang
    //    the command future forever), then parse + validate `state`.
    let callback_url = match tokio::time::timeout(std::time::Duration::from_secs(180), rx).await {
        Ok(received) => received
            .map_err(|_| "oauth callback channel closed before a response arrived".to_string())?,
        Err(_) => {
            // Timed out waiting for the callback; tear down the loopback server.
            let _ = tauri_plugin_oauth::cancel(port);
            return Err("github login timed out".to_string());
        }
    };
    let (code, returned_state) = parse_callback(&callback_url)?;
    if returned_state != csrf_state {
        return Err("oauth `state` mismatch (possible CSRF) — aborting".to_string());
    }

    // 6. Exchange the code for a token.
    let client = reqwest::Client::new();
    let token = exchange_code(
        &client,
        &TokenRequest {
            token_endpoint: GITHUB_TOKEN_ENDPOINT,
            client_id: &client_id,
            client_secret: Some(&client_secret),
            code: &code,
            redirect_uri: &redirect_uri,
            pkce_verifier: None,
        },
    )
    .await
    .map_err(|e| e.to_string())?;

    // 7. Identify the account by its GitHub login (so the user sees "octocat",
    //    not a repo path). The dashboard lists all repos via /user/repos, so no
    //    single owner/repo is needed at connect time.
    let login = client
        .get(GITHUB_USER_ENDPOINT)
        .header("Authorization", format!("Bearer {}", token.access_token))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "newt-todo")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?
        .get("login")
        .and_then(|v| v.as_str())
        .unwrap_or("github")
        .to_string();

    // 8. Create the account (OAuth-only; no repo bound at connect time).
    let account = state
        .service
        .create_account(AccountDraft {
            provider: ProviderKind::Github,
            display_name: login,
            base_url: None,
            config: serde_json::json!({}),
        })
        .await
        .map_err(|e| e.to_string())?;

    // 8. Persist the token keyed by the new account id.
    state
        .token_store
        .save(&account.id.to_string(), &token)
        .await
        .map_err(|e| e.to_string())?;

    // 9. Return the freshly created account.
    Ok(account)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create the sqlx pool on Tauri's own async runtime so it lives on the
    // same runtime the commands execute on (tauri::async_runtime is tokio-based).
    let db_url = newt_todo_service::resolve_db_url().expect("resolve db url");
    let store = tauri::async_runtime::block_on(newt_todo_core::Store::connect(&db_url))
        .expect("connect store");
    let service = TaskService::new(store);

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            service,
            token_store: KeyringTokenStore::new(),
        })
        .invoke_handler(tauri::generate_handler![
            list_tasks,
            create_task,
            update_task,
            complete_task,
            delete_task,
            list_accounts,
            create_account,
            delete_account,
            sync_account,
            list_repos,
            list_issues,
            list_pull_requests,
            search_code,
            search_issues,
            list_notifications,
            mark_notification_read,
            list_workflow_runs,
            get_repo_detail,
            list_releases,
            list_forks,
            list_contributors,
            get_languages,
            get_traffic_views,
            get_traffic_clones,
            list_referrers,
            list_paths,
            get_issue,
            get_pull_request,
            list_issue_comments,
            list_pull_reviews,
            set_issue_state,
            add_issue_labels,
            remove_issue_label,
            add_issue_assignees,
            create_issue_comment,
            capture_snapshots,
            list_snapshots,
            daily_digest,
            start_github_login,
            list_boards,
            get_board,
            create_board,
            rename_board,
            delete_board,
            reorder_boards,
            create_column,
            update_column,
            delete_column,
            reorder_columns,
            place_card,
            remove_card
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::parse_callback;

    #[test]
    fn parse_callback_extracts_code_and_state() {
        let (code, state) = parse_callback("http://127.0.0.1:8765/?code=abc123&state=xyz").unwrap();
        assert_eq!(code, "abc123");
        assert_eq!(state, "xyz");
    }

    #[test]
    fn parse_callback_handles_extra_params_and_order() {
        let (code, state) =
            parse_callback("http://127.0.0.1:8765/cb?state=s1&foo=bar&code=c1").unwrap();
        assert_eq!(code, "c1");
        assert_eq!(state, "s1");
    }

    #[test]
    fn parse_callback_url_decodes_values() {
        let (code, state) =
            parse_callback("http://127.0.0.1:8765/?code=a%2Fb&state=x%20y").unwrap();
        assert_eq!(code, "a/b");
        assert_eq!(state, "x y");
    }

    #[test]
    fn parse_callback_missing_code_errors() {
        let err = parse_callback("http://127.0.0.1:8765/?state=xyz").unwrap_err();
        assert!(err.contains("code"), "{err}");
    }

    #[test]
    fn parse_callback_missing_state_errors() {
        let err = parse_callback("http://127.0.0.1:8765/?code=abc").unwrap_err();
        assert!(err.contains("state"), "{err}");
    }

    #[test]
    fn parse_callback_invalid_url_errors() {
        let err = parse_callback("not a url").unwrap_err();
        assert!(err.contains("invalid callback url"), "{err}");
    }
}
