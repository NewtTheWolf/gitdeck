# Plan 4a — Provider trait + Sync engine + GitHub client (testable core)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Checkboxes track steps. TDD throughout.

**Goal:** The fully-testable foundation of provider sync: a `Provider` trait + remote types in `core`, `Store` methods for syncing remote tasks, a `sync_account` engine (tested with a mock provider), and a GitHub `Provider` implementation tested against mocked HTTP (`wiremock`). NO OAuth, NO keychain, NO real network — those are Plan 4b.

**Architecture:** `core` gains a `provider` module (trait + `RemoteTask`/`RemoteDraft`/`RemotePatch`, `async-trait`) and a `sync` module (engine generic over `&dyn Provider`). `Store` gains `upsert_remote_task`/`list_dirty`/`mark_synced`. New `crates/providers` crate (`newt-todo-providers`) holds `GitHubProvider` (reqwest) implementing `core::Provider`, with a configurable `base_url` so tests point it at a `wiremock` server. `core` stays free of reqwest.

**Tech Stack:** core + `async-trait`; `crates/providers` with reqwest (json, rustls-tls), serde; dev: wiremock, tokio.

**Scope (4a):** pull (upsert remote → local, LWW: remote wins when newer and local not dirty; local-dirty is preserved) + push (dirty tasks that have a source → provider.update). Promote (local→remote create) and conflict-copy UI are deferred to later plans.

---

## Task 1: `core::provider` — trait + remote types

**Files:** Create `crates/core/src/provider.rs`; Modify `crates/core/src/lib.rs`, `crates/core/Cargo.toml`, root `Cargo.toml`.

- [ ] **Step 1: deps** — root `Cargo.toml` `[workspace.dependencies]` add `async-trait = "0.1"`. `crates/core/Cargo.toml` add `async-trait = { workspace = true }`.

- [ ] **Step 2: failing test** — append to `crates/core/src/provider.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_draft_builds() {
        let d = RemoteDraft { title: "x".into(), body: "b".into(), labels: vec!["l".into()] };
        assert_eq!(d.title, "x");
    }
}
```
Run `cargo test -p newt-todo-core provider` → FAIL.

- [ ] **Step 3: implement** — at top of `crates/core/src/provider.rs`:
```rust
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
    async fn update_task(&self, remote_id: &str, patch: RemotePatch)
        -> Result<RemoteTask, ProviderError>;
}
```
Add to `lib.rs`: `pub mod provider;` and re-export `pub use provider::{Provider, ProviderError, RemoteDraft, RemotePatch, RemoteTask};`.

- [ ] **Step 4** `cargo test -p newt-todo-core provider` → PASS. **Step 5** commit: `feat(core): provider trait and remote task types`.

---

## Task 2: `Store` sync methods

**Files:** Modify `crates/core/src/store.rs`.

- [ ] **Step 1: failing tests** — add to store.rs `tests`:
```rust
    use crate::provider::RemoteTask;
    use crate::domain::TaskStatus;

    fn remote(id: &str, title: &str, status: TaskStatus, updated: time::OffsetDateTime) -> RemoteTask {
        RemoteTask { remote_id: id.into(), title: title.into(), body: String::new(),
            status, labels: vec![], html_url: Some(format!("https://x/{id}")), remote_updated_at: updated }
    }

    #[tokio::test]
    async fn upsert_remote_inserts_then_updates_when_not_dirty() {
        let store = mem_store().await;
        let acct = uuid::Uuid::new_v4();
        let t0 = datetime!(2026-06-15 10:00:00 UTC);
        let t1 = datetime!(2026-06-15 11:00:00 UTC);

        let created = store.upsert_remote_task(acct, remote("1", "first", TaskStatus::Open, t0), t0).await.unwrap();
        assert_eq!(created.title, "first");
        assert_eq!(created.source.as_ref().unwrap().remote_id, "1");
        assert!(!created.dirty);

        // remote changed, local not dirty → remote wins
        let updated = store.upsert_remote_task(acct, remote("1", "renamed", TaskStatus::Done, t1), t1).await.unwrap();
        assert_eq!(updated.id, created.id, "same row (matched by account+remote_id)");
        assert_eq!(updated.title, "renamed");
        assert_eq!(updated.status, TaskStatus::Done);
    }

    #[tokio::test]
    async fn upsert_remote_preserves_local_dirty_edits() {
        let store = mem_store().await;
        let acct = uuid::Uuid::new_v4();
        let t0 = datetime!(2026-06-15 10:00:00 UTC);
        let t1 = datetime!(2026-06-15 11:00:00 UTC);
        let created = store.upsert_remote_task(acct, remote("1", "first", TaskStatus::Open, t0), t0).await.unwrap();
        // user edits locally → dirty
        store.update_task(created.id, crate::domain::TaskPatch { title: Some("my edit".into()), ..Default::default() }, t1).await.unwrap();
        // remote pull comes in; must NOT clobber the dirty local edit
        let after = store.upsert_remote_task(acct, remote("1", "remote change", TaskStatus::Done, t1), t1).await.unwrap();
        assert_eq!(after.title, "my edit", "local dirty edit preserved over incoming remote change");
        assert!(after.dirty);
    }

    #[tokio::test]
    async fn list_dirty_and_mark_synced() {
        let store = mem_store().await;
        let acct = uuid::Uuid::new_v4();
        let t0 = datetime!(2026-06-15 10:00:00 UTC);
        let t = store.upsert_remote_task(acct, remote("9", "task", TaskStatus::Open, t0), t0).await.unwrap();
        store.update_task(t.id, crate::domain::TaskPatch { status: Some(TaskStatus::Done), ..Default::default() }, t0).await.unwrap();

        let dirty = store.list_dirty(acct).await.unwrap();
        assert_eq!(dirty.len(), 1);
        assert_eq!(dirty[0].id, t.id);

        store.mark_synced(t.id, t0, t0).await.unwrap();
        assert!(store.list_dirty(acct).await.unwrap().is_empty());
    }
```
Run → FAIL (methods missing).

- [ ] **Step 2: implement** in `impl Store` (use existing imports + `crate::provider::RemoteTask`):
```rust
    /// Insert or update a task mirrored from a provider, keyed by (account_id, remote_id).
    /// If the local row is `dirty` (unpushed local edits), the incoming remote state is
    /// NOT applied (local wins until the next push resolves it). Otherwise remote wins.
    pub async fn upsert_remote_task(
        &self,
        account_id: Uuid,
        remote: RemoteTask,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let existing = sqlx::query("SELECT id, dirty FROM tasks WHERE account_id = ? AND remote_id = ?")
            .bind(account_id.to_string())
            .bind(&remote.remote_id)
            .fetch_optional(&self.pool)
            .await?;

        let labels_json = serde_json::to_string(&remote.labels)?;
        let r_updated = remote.remote_updated_at.format(&Rfc3339)
            .map_err(|_| CoreError::DataFormat("remote_updated_at"))?;
        let now_s = now.format(&Rfc3339).map_err(|_| CoreError::DataFormat("now"))?;

        match existing {
            None => {
                let id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO tasks (id, account_id, remote_id, html_url, title, body, status, labels,
                                        remote_updated_at, local_updated_at, dirty, deleted)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0)",
                )
                .bind(id.to_string())
                .bind(account_id.to_string())
                .bind(&remote.remote_id)
                .bind(&remote.html_url)
                .bind(&remote.title)
                .bind(&remote.body)
                .bind(remote.status.as_str())
                .bind(&labels_json)
                .bind(&r_updated)
                .bind(&now_s)
                .execute(&self.pool)
                .await?;
                self.get_task(id).await?.ok_or(CoreError::NotFound(id))
            }
            Some(row) => {
                let id = Uuid::parse_str(&row.try_get::<String, _>("id")?)
                    .map_err(|_| CoreError::DataFormat("id"))?;
                let dirty: i64 = row.try_get("dirty")?;
                if dirty != 0 {
                    // local edits pending → don't overwrite; just return current row
                    return self.get_task(id).await?.ok_or(CoreError::NotFound(id));
                }
                sqlx::query(
                    "UPDATE tasks SET html_url = ?, title = ?, body = ?, status = ?, labels = ?,
                                      remote_updated_at = ?, local_updated_at = ? WHERE id = ?",
                )
                .bind(&remote.html_url)
                .bind(&remote.title)
                .bind(&remote.body)
                .bind(remote.status.as_str())
                .bind(&labels_json)
                .bind(&r_updated)
                .bind(&now_s)
                .bind(id.to_string())
                .execute(&self.pool)
                .await?;
                self.get_task(id).await?.ok_or(CoreError::NotFound(id))
            }
        }
    }

    /// Tasks with unpushed local edits for an account (have a source on that account).
    pub async fn list_dirty(&self, account_id: Uuid) -> Result<Vec<Task>, CoreError> {
        let rows = sqlx::query("SELECT * FROM tasks WHERE account_id = ? AND dirty = 1")
            .bind(account_id.to_string())
            .fetch_all(&self.pool)
            .await?;
        rows.into_iter().map(map_row).collect()
    }

    /// Clear the dirty flag and record the provider's updated timestamp after a successful push.
    pub async fn mark_synced(
        &self,
        id: Uuid,
        remote_updated_at: OffsetDateTime,
        now: OffsetDateTime,
    ) -> Result<(), CoreError> {
        let r = remote_updated_at.format(&Rfc3339).map_err(|_| CoreError::DataFormat("remote_updated_at"))?;
        let n = now.format(&Rfc3339).map_err(|_| CoreError::DataFormat("now"))?;
        let res = sqlx::query("UPDATE tasks SET dirty = 0, remote_updated_at = ?, local_updated_at = ? WHERE id = ?")
            .bind(&r).bind(&n).bind(id.to_string())
            .execute(&self.pool).await?;
        if res.rows_affected() == 0 { return Err(CoreError::NotFound(id)); }
        Ok(())
    }
```
- [ ] **Step 3** `cargo test -p newt-todo-core store` → PASS. **Step 4** commit: `feat(core): store sync methods (upsert_remote/list_dirty/mark_synced)`.

---

## Task 3: `core::sync` engine

**Files:** Create `crates/core/src/sync.rs`; Modify `lib.rs`.

- [ ] **Step 1: failing test** — `crates/core/src/sync.rs`:
```rust
use time::OffsetDateTime;
use uuid::Uuid;

use crate::provider::{Provider, RemotePatch};
use crate::store::Store;
use crate::CoreError;

#[derive(Debug, Default, PartialEq)]
pub struct SyncReport {
    pub pulled: usize,
    pub pushed: usize,
}

/// Pull remote tasks into the store, then push local dirty edits back to the provider.
pub async fn sync_account(
    store: &Store,
    account_id: Uuid,
    provider: &dyn Provider,
    now: OffsetDateTime,
) -> Result<SyncReport, CoreError> {
    let mut report = SyncReport::default();

    // Pull
    let remotes = provider.list_tasks().await.map_err(|e| CoreError::DataFormat_owned(e.0))?;
    for r in remotes {
        store.upsert_remote_task(account_id, r, now).await?;
        report.pulled += 1;
    }

    // Push local dirty edits that map to a remote task on this account.
    for task in store.list_dirty(account_id).await? {
        let Some(src) = task.source.as_ref() else { continue };
        let patch = RemotePatch {
            title: Some(task.title.clone()),
            body: Some(task.body.clone()),
            status: Some(task.status),
            labels: Some(task.labels.clone()),
        };
        let updated = provider.update_task(&src.remote_id, patch).await
            .map_err(|e| CoreError::DataFormat_owned(e.0))?;
        store.mark_synced(task.id, updated.remote_updated_at, now).await?;
        report.pushed += 1;
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{TaskPatch, TaskStatus};
    use crate::provider::{ProviderError, RemoteDraft, RemoteTask};
    use async_trait::async_trait;
    use std::sync::Mutex;
    use time::macros::datetime;

    struct FakeProvider {
        list: Vec<RemoteTask>,
        updated: Mutex<Vec<(String, String)>>, // (remote_id, new title)
    }
    #[async_trait]
    impl Provider for FakeProvider {
        async fn list_tasks(&self) -> Result<Vec<RemoteTask>, ProviderError> { Ok(self.list.clone()) }
        async fn create_task(&self, _d: RemoteDraft) -> Result<RemoteTask, ProviderError> { unimplemented!() }
        async fn update_task(&self, remote_id: &str, patch: RemotePatch) -> Result<RemoteTask, ProviderError> {
            self.updated.lock().unwrap().push((remote_id.into(), patch.title.clone().unwrap_or_default()));
            Ok(RemoteTask { remote_id: remote_id.into(), title: patch.title.unwrap_or_default(),
                body: String::new(), status: TaskStatus::Open, labels: vec![],
                html_url: None, remote_updated_at: datetime!(2026-06-15 12:00:00 UTC) })
        }
    }

    #[tokio::test]
    async fn pull_then_push_roundtrip() {
        let store = Store::connect("sqlite::memory:").await.unwrap();
        let acct = Uuid::new_v4();
        let now = datetime!(2026-06-15 10:00:00 UTC);
        let provider = FakeProvider {
            list: vec![RemoteTask { remote_id: "1".into(), title: "remote".into(), body: "".into(),
                status: TaskStatus::Open, labels: vec![], html_url: None, remote_updated_at: now }],
            updated: Mutex::new(vec![]),
        };

        let r1 = sync_account(&store, acct, &provider, now).await.unwrap();
        assert_eq!(r1.pulled, 1);
        assert_eq!(r1.pushed, 0);

        // user edits the pulled task → dirty
        let pulled = store.list_tasks(crate::domain::TaskFilter::default()).await.unwrap();
        store.update_task(pulled[0].id, TaskPatch { title: Some("edited".into()), ..Default::default() }, now).await.unwrap();

        let r2 = sync_account(&store, acct, &provider, now).await.unwrap();
        assert_eq!(r2.pushed, 1, "dirty edit pushed");
        assert_eq!(provider.updated.lock().unwrap()[0], ("1".to_string(), "edited".to_string()));
        // after push, no longer dirty
        assert!(store.list_dirty(acct).await.unwrap().is_empty());
    }
}
```
Note: this references `CoreError::DataFormat_owned` — that does not exist. In Step 3 you will add a `CoreError` variant for provider failures instead. For the test to compile, use a real variant.

- [ ] **Step 2: add a CoreError variant** in `lib.rs`:
```rust
    #[error("provider failure: {0}")]
    Provider(String),
```
Then in `sync.rs` replace both `CoreError::DataFormat_owned(e.0)` with `CoreError::Provider(e.0)`.

- [ ] **Step 3** add `pub mod sync;` + `pub use sync::{sync_account, SyncReport};` to `lib.rs`. Run `cargo test -p newt-todo-core sync` → PASS. **Step 4** commit: `feat(core): sync_account engine (pull + push)`.

---

## Task 4: `crates/providers` — GitHub client

**Files:** Create `crates/providers/Cargo.toml`, `crates/providers/src/lib.rs`, `crates/providers/src/github.rs`; Modify root `Cargo.toml` (member + deps).

- [ ] **Step 1: crate + deps** — add `"crates/providers"` to members; workspace deps: `reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"] }`, `wiremock = "0.6"`. Create `crates/providers/Cargo.toml`:
```toml
[package]
name = "newt-todo-providers"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
newt-todo-core = { workspace = true }
reqwest = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
time = { workspace = true }
async-trait = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
wiremock = { workspace = true }
```
`crates/providers/src/lib.rs`: `pub mod github; pub use github::GitHubProvider;`

- [ ] **Step 2: failing test** — `crates/providers/src/github.rs` test module: spin up a `wiremock::MockServer`, mock `GET /repos/o/r/issues` returning a JSON array with one issue (`number:1, title:"hi", body:"b", state:"open", html_url, updated_at:"2026-06-15T10:00:00Z", labels:[{"name":"bug"}]`) and one PR-like entry (with a `pull_request` field) that MUST be filtered out. Construct `GitHubProvider::new(client, base_url=server.uri(), token="t", owner="o", repo="r")`, call `list_tasks()`, assert exactly one `RemoteTask` with `remote_id == "1"`, `status == Open`, `labels == ["bug"]`. Add a second test mocking `PATCH /repos/o/r/issues/1` for `update_task` (state→closed maps to `status: Done`). Run → FAIL.

- [ ] **Step 3: implement** `GitHubProvider`:
```rust
use async_trait::async_trait;
use newt_todo_core::{Provider, ProviderError, RemoteDraft, RemotePatch, RemoteTask, TaskStatus};
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
    pub fn new(client: reqwest::Client, base_url: impl Into<String>, token: impl Into<String>,
               owner: impl Into<String>, repo: impl Into<String>) -> Self {
        Self { client, base_url: base_url.into(), token: token.into(),
               owner: owner.into(), repo: repo.into() }
    }
    fn issues_url(&self) -> String {
        format!("{}/repos/{}/{}/issues", self.base_url, self.owner, self.repo)
    }
}

#[derive(Deserialize)]
struct GhLabel { name: String }
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
            status: if self.state == "closed" { TaskStatus::Done } else { TaskStatus::Open },
            labels: self.labels.into_iter().map(|l| l.name).collect(),
            html_url: Some(self.html_url),
            remote_updated_at: OffsetDateTime::parse(&self.updated_at, &Rfc3339)
                .map_err(|e| ProviderError(format!("bad updated_at: {e}")))?,
        })
    }
}

#[async_trait]
impl Provider for GitHubProvider {
    async fn list_tasks(&self) -> Result<Vec<RemoteTask>, ProviderError> {
        let resp = self.client.get(self.issues_url())
            .query(&[("state", "all"), ("per_page", "100")])
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .send().await.map_err(|e| ProviderError(e.to_string()))?;
        let issues: Vec<GhIssue> = resp.error_for_status().map_err(|e| ProviderError(e.to_string()))?
            .json().await.map_err(|e| ProviderError(e.to_string()))?;
        issues.into_iter()
            .filter(|i| i.pull_request.is_none()) // exclude PRs
            .map(GhIssue::into_remote)
            .collect()
    }

    async fn create_task(&self, draft: RemoteDraft) -> Result<RemoteTask, ProviderError> {
        let resp = self.client.post(self.issues_url())
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::json!({ "title": draft.title, "body": draft.body, "labels": draft.labels }))
            .send().await.map_err(|e| ProviderError(e.to_string()))?;
        let issue: GhIssue = resp.error_for_status().map_err(|e| ProviderError(e.to_string()))?
            .json().await.map_err(|e| ProviderError(e.to_string()))?;
        issue.into_remote()
    }

    async fn update_task(&self, remote_id: &str, patch: RemotePatch) -> Result<RemoteTask, ProviderError> {
        let mut body = serde_json::Map::new();
        if let Some(t) = patch.title { body.insert("title".into(), t.into()); }
        if let Some(b) = patch.body { body.insert("body".into(), b.into()); }
        if let Some(s) = patch.status {
            body.insert("state".into(), (if s == TaskStatus::Done { "closed" } else { "open" }).into());
        }
        if let Some(l) = patch.labels { body.insert("labels".into(), serde_json::json!(l)); }
        let url = format!("{}/{}", self.issues_url(), remote_id);
        let resp = self.client.patch(url)
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Accept", "application/vnd.github+json")
            .header("User-Agent", "newt-todo")
            .json(&serde_json::Value::Object(body))
            .send().await.map_err(|e| ProviderError(e.to_string()))?;
        let issue: GhIssue = resp.error_for_status().map_err(|e| ProviderError(e.to_string()))?
            .json().await.map_err(|e| ProviderError(e.to_string()))?;
        issue.into_remote()
    }
}
```
- [ ] **Step 4** `cargo test -p newt-todo-providers` → PASS (wiremock list + update, PR filtered). **Step 5** `cargo test --workspace` (all green), `cargo clippy --workspace --all-targets -- -D warnings` (clean), `cargo fmt`. Commit: `feat(providers): github issues provider with wiremock tests`.

---

## Done Criteria
- `Provider` trait + `RemoteTask` types in core; `Store` sync methods; `sync_account` engine — all unit-tested.
- `GitHubProvider` maps issues ↔ core, filters PRs, tested against mocked HTTP.
- `core` has no reqwest; providers crate depends on core.
- Full workspace test suite green.

## Next: Plan 4b (needs your real GitHub OAuth app to verify)
OAuth authorization-code + loopback (`tauri-plugin-oauth`), `client_secret`/token in OS keychain (`keyring`), Account management, wire `GitHubProvider` + `sync_account` into the app (a `sync` Tauri command) and the MCP (`sync` tool), settings UI to add an account/repo.
