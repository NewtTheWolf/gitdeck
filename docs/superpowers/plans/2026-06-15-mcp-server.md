# MCP Server (stdio) Implementation Plan — Plan 2

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A standalone stdio MCP binary (`newt-todo-mcp`) that lets an agent manage the local todos in the shared SQLite DB via the `newt-todo-core` Store.

**Architecture:** New `crates/mcp` crate depending on `newt-todo-core`. All tool logic lives in a plain, fully-tested `TaskService` (over `core::Store`, returns serde-serializable DTOs). The rmcp layer (`#[tool_router]`/`#[tool]` macros, stdio transport) is a thin adapter that calls `TaskService`. DB path is resolved from `NEWT_TODO_DB` (env override, used by tests) or a default OS data dir.

**Tech Stack:** rmcp 1.7 (features: `server`), schemars, serde, serde_json, tokio, anyhow, directories, time; plus `newt-todo-core` (workspace). `tempfile` for tests.

**Scope:** Local-todo tools only — `list_tasks, get_task, create_task, update_task, complete_task, delete_task`. NO `sync` tool (needs providers — Plan 4+).

---

## File Structure

```
crates/mcp/
├─ Cargo.toml
└─ src/
   ├─ main.rs        # #[tokio::main]: resolve db, build Store+TaskService, serve(stdio())
   ├─ config.rs      # db_url() resolution (env override + default data dir)
   ├─ dto.rs         # TaskDto + From<core::Task>, params structs (schemars)
   ├─ service.rs     # TaskService: async methods over core::Store, returns DTOs
   └─ server.rs      # rmcp tool-router adapter calling TaskService
```

Each file: `config` = path resolution, `dto` = wire types, `service` = tested business logic, `server` = SDK glue, `main` = wiring.

---

## Task 1: Crate scaffold + db_url resolution

**Files:** Create `crates/mcp/Cargo.toml`, `crates/mcp/src/main.rs` (stub), `crates/mcp/src/config.rs`; Modify root `Cargo.toml` (add member).

- [ ] **Step 1: Add crate to workspace**

In root `Cargo.toml`, change `members = ["crates/core"]` to `members = ["crates/core", "crates/mcp"]`. Add to `[workspace.dependencies]`:

```toml
newt-todo-core = { path = "crates/core" }
rmcp = { version = "1", features = ["server"] }
anyhow = "1"
directories = "5"
tempfile = "3"
```

**schemars version — do this carefully:** rmcp derives `JsonSchema` against its OWN `schemars`. Deriving from a mismatched standalone `schemars` is a hard build break. So:
1. First run `cargo add rmcp -F server` (or rely on the workspace dep) and resolve.
2. Find the schemars version rmcp pulls in: `cargo tree -p newt-todo-mcp -i schemars` (or `cargo tree | grep schemars`).
3. Add that EXACT schemars version to `[workspace.dependencies]` (e.g. `schemars = "=<resolved>"`), OR — preferred — skip a separate dep and use rmcp's re-export everywhere: `use rmcp::schemars;` and `#[derive(rmcp::schemars::JsonSchema)]`.

Prefer the re-export route if it compiles; fall back to a version-matched direct dep otherwise. Note which you used.

(If `cargo` resolves a different latest `rmcp`/`directories` major, accept it and adjust APIs accordingly — note it in your report.)

- [ ] **Step 2: Create crate manifest**

Create `crates/mcp/Cargo.toml`:

```toml
[package]
name = "newt-todo-mcp"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[[bin]]
name = "newt-todo-mcp"
path = "src/main.rs"

[dependencies]
newt-todo-core = { workspace = true }
rmcp = { workspace = true }
schemars = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "io-std"] }
time = { workspace = true }
anyhow = { workspace = true }
directories = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "io-std"] }
```

- [ ] **Step 3: Write the failing test for db_url**

Create `crates/mcp/src/config.rs`:

```rust
use std::path::Path;

/// Build a sqlx SQLite URL for a filesystem path, creating the DB if missing.
pub fn db_url_for_path(path: &Path) -> String {
    format!("sqlite://{}?mode=rwc", path.display())
}

/// Resolve the DB URL: `NEWT_TODO_DB` env var (a filesystem path) wins; otherwise
/// the default OS data dir `<data_dir>/newt-todo/todo.db`. Ensures the parent dir exists.
pub fn resolve_db_url() -> anyhow::Result<String> {
    let path = match std::env::var_os("NEWT_TODO_DB") {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            let dirs = directories::ProjectDirs::from("dev", "NewtTheWolf", "newt-todo")
                .ok_or_else(|| anyhow::anyhow!("cannot determine OS data dir"))?;
            dirs.data_dir().join("todo.db")
        }
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(db_url_for_path(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn db_url_for_path_uses_rwc_mode() {
        let url = db_url_for_path(&PathBuf::from("/tmp/x/todo.db"));
        assert_eq!(url, "sqlite:///tmp/x/todo.db?mode=rwc");
    }
}
```

- [ ] **Step 4: Stub main.rs so the crate builds**

Create `crates/mcp/src/main.rs`:

```rust
mod config;

fn main() -> anyhow::Result<()> {
    let _ = config::resolve_db_url()?;
    Ok(())
}
```

- [ ] **Step 5: Run test + build**

Run: `cargo test -p newt-todo-mcp config`
Expected: PASS.
Run: `cargo build -p newt-todo-mcp`
Expected: builds (may warn about unused `config` items — fine for now).

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock crates/mcp
git commit -m "feat(mcp): scaffold mcp crate and db_url resolution"
```

---

## Task 2: DTOs + TaskService (list/get/create)

**Files:** Create `crates/mcp/src/dto.rs`, `crates/mcp/src/service.rs`; Modify `crates/mcp/src/main.rs` (add `mod dto; mod service;`).

- [ ] **Step 1: Create DTOs**

Create `crates/mcp/src/dto.rs`:

```rust
use newt_todo_core::{Task, TaskStatus};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;

/// Agent-facing view of a task (timestamps as RFC3339 strings).
#[derive(Debug, Clone, Serialize, JsonSchema)]
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

#[derive(Debug, Deserialize, JsonSchema)]
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

#[derive(Debug, Deserialize, JsonSchema)]
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskIdParam {
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
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

pub(crate) fn parse_status(s: &str) -> anyhow::Result<TaskStatus> {
    TaskStatus::from_str(s).ok_or_else(|| anyhow::anyhow!("invalid status `{s}` (use open|done)"))
}
```

- [ ] **Step 2: Write the failing test for TaskService create/get/list**

Create `crates/mcp/src/service.rs`:

```rust
use newt_todo_core::{Store, TaskDraft, TaskFilter};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::dto::{
    parse_status, CreateTaskParams, ListTasksParams, TaskDto, UpdateTaskParams,
};

pub struct TaskService {
    store: Store,
}

impl TaskService {
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    fn now() -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }

    pub async fn create(&self, p: CreateTaskParams) -> anyhow::Result<TaskDto> {
        let mut draft = TaskDraft::new(p.title);
        draft.body = p.body.unwrap_or_default();
        draft.labels = p.labels.unwrap_or_default();
        if let Some(due) = p.due_at {
            draft.due_at = Some(OffsetDateTime::parse(&due, &Rfc3339)?);
        }
        let task = self.store.create_task(draft, Self::now()).await?;
        Ok(task.into())
    }

    pub async fn list(&self, p: ListTasksParams) -> anyhow::Result<Vec<TaskDto>> {
        let status = p.status.as_deref().map(parse_status).transpose()?;
        let filter = TaskFilter {
            status,
            label: p.label,
            query: p.query,
            include_deleted: false,
        };
        let tasks = self.store.list_tasks(filter).await?;
        Ok(tasks.into_iter().map(Into::into).collect())
    }

    pub async fn get(&self, id: &str) -> anyhow::Result<Option<TaskDto>> {
        let uuid = Uuid::parse_str(id)?;
        Ok(self.store.get_task(uuid).await?.map(Into::into))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn svc() -> TaskService {
        TaskService::new(Store::connect("sqlite::memory:").await.unwrap())
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

        let listed = svc.list(ListTasksParams { status: None, label: None, query: None }).await.unwrap();
        assert_eq!(listed.len(), 1);

        let got = svc.get(&created.id).await.unwrap().unwrap();
        assert_eq!(got.id, created.id);

        assert!(svc.get(&Uuid::new_v4().to_string()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn list_filters_by_status() {
        let svc = svc().await;
        svc.create(CreateTaskParams { title: "a".into(), body: None, labels: None, due_at: None }).await.unwrap();
        let only_done = svc.list(ListTasksParams { status: Some("done".into()), label: None, query: None }).await.unwrap();
        assert!(only_done.is_empty());
        let bad = svc.list(ListTasksParams { status: Some("nope".into()), label: None, query: None }).await;
        assert!(bad.is_err(), "invalid status must error");
    }
}
```

Add `mod dto; mod service;` to `main.rs` (keep `mod config;`). Add `uuid` to the mcp crate deps (it's a workspace dep): in `crates/mcp/Cargo.toml` `[dependencies]` add `uuid = { workspace = true }`.

- [ ] **Step 3: Run the tests**

Run: `cargo test -p newt-todo-mcp service`
Expected: FAIL first if anything is missing, then after the code above compiles → PASS (both tests).

- [ ] **Step 4: Verify whole crate**

Run: `cargo test -p newt-todo-mcp`
Expected: PASS (config + service tests).

- [ ] **Step 5: Commit**

```bash
git add crates/mcp/src crates/mcp/Cargo.toml Cargo.lock
git commit -m "feat(mcp): TaskDto and TaskService create/list/get"
```

---

## Task 3: TaskService update / complete / delete

**Files:** Modify `crates/mcp/src/service.rs`.

- [ ] **Step 1: Write the failing tests**

Add inside `service.rs` `tests` module:

```rust
    use crate::dto::{TaskIdParam, UpdateTaskParams};

    #[tokio::test]
    async fn update_complete_delete() {
        let svc = svc().await;
        let t = svc.create(CreateTaskParams { title: "task".into(), body: None, labels: None, due_at: None }).await.unwrap();

        // update title + status
        let upd = svc.update(UpdateTaskParams {
            id: t.id.clone(), title: Some("renamed".into()), body: None,
            status: Some("done".into()), labels: None, due_at: None, clear_due: false,
        }).await.unwrap();
        assert_eq!(upd.title, "renamed");
        assert_eq!(upd.status, "done");

        // complete is idempotent-ish: sets status done
        let t2 = svc.create(CreateTaskParams { title: "two".into(), body: None, labels: None, due_at: None }).await.unwrap();
        let done = svc.complete(TaskIdParam { id: t2.id.clone() }).await.unwrap();
        assert_eq!(done.status, "done");

        // delete hides it from listing
        svc.delete(TaskIdParam { id: t.id.clone() }).await.unwrap();
        let remaining = svc.list(ListTasksParams { status: None, label: None, query: None }).await.unwrap();
        assert!(remaining.iter().all(|x| x.id != t.id), "deleted task is hidden");
    }

    #[tokio::test]
    async fn update_clear_due_overrides_due_at() {
        let svc = svc().await;
        let t = svc.create(CreateTaskParams {
            title: "due".into(), body: None, labels: None,
            due_at: Some("2026-07-01T09:00:00Z".into()),
        }).await.unwrap();
        assert!(t.due_at.is_some());

        let cleared = svc.update(UpdateTaskParams {
            id: t.id.clone(), title: None, body: None, status: None, labels: None,
            due_at: Some("2026-08-01T09:00:00Z".into()), clear_due: true,
        }).await.unwrap();
        assert!(cleared.due_at.is_none(), "clear_due overrides due_at");
    }
```

- [ ] **Step 2: Run to verify failure**

Run: `cargo test -p newt-todo-mcp service::tests::update_complete_delete`
Expected: FAIL — `update`/`complete`/`delete` not defined.

- [ ] **Step 3: Implement the methods**

Add to `impl TaskService` (need `use newt_todo_core::TaskPatch;` at top of service.rs, and `use crate::dto::TaskIdParam;`):

```rust
    pub async fn update(&self, p: UpdateTaskParams) -> anyhow::Result<TaskDto> {
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
        let task = self.store.update_task(uuid, patch, Self::now()).await?;
        Ok(task.into())
    }

    pub async fn complete(&self, p: TaskIdParam) -> anyhow::Result<TaskDto> {
        self.update(UpdateTaskParams {
            id: p.id,
            title: None,
            body: None,
            status: Some("done".into()),
            labels: None,
            due_at: None,
            clear_due: false,
        })
        .await
    }

    pub async fn delete(&self, p: TaskIdParam) -> anyhow::Result<()> {
        let uuid = Uuid::parse_str(&p.id)?;
        self.store.delete_task(uuid, Self::now()).await?;
        Ok(())
    }
```

- [ ] **Step 4: Run the tests**

Run: `cargo test -p newt-todo-mcp`
Expected: PASS (config + all service tests).

- [ ] **Step 5: Commit**

```bash
git add crates/mcp/src/service.rs
git commit -m "feat(mcp): TaskService update/complete/delete"
```

---

## Task 4: rmcp tool adapter + stdio main + smoke test

**Files:** Create `crates/mcp/src/server.rs`; rewrite `crates/mcp/src/main.rs`.

> **rmcp API note:** Target rmcp 1.x. The intended shape is a `#[tool_router]` impl with `#[tool]` methods taking `Parameters<T>` and returning a JSON result. If the exact macro names/signatures differ in the resolved rmcp version, consult `cargo doc -p rmcp` or the crate source under `~/.cargo/registry` and ADAPT while keeping the same six tools and stdio transport. Report any deviation from the code below.

- [ ] **Step 1: Implement the rmcp adapter**

Create `crates/mcp/src/server.rs`:

```rust
use std::sync::Arc;

use rmcp::handler::server::wrapper::Parameters;
use rmcp::{tool, tool_router, ServerHandler};

use crate::dto::{CreateTaskParams, ListTasksParams, TaskIdParam, UpdateTaskParams};
use crate::service::TaskService;

#[derive(Clone)]
pub struct TodoServer {
    service: Arc<TaskService>,
}

impl TodoServer {
    pub fn new(service: TaskService) -> Self {
        Self { service: Arc::new(service) }
    }

    fn ok<T: serde::Serialize>(value: T) -> Result<String, rmcp::ErrorData> {
        serde_json::to_string(&value)
            .map_err(|e| rmcp::ErrorData::internal_error(e.to_string(), None))
    }

    fn fail(e: anyhow::Error) -> rmcp::ErrorData {
        rmcp::ErrorData::internal_error(e.to_string(), None)
    }
}

#[tool_router(server_handler)]
impl TodoServer {
    #[tool(description = "List todos. Optional filters: status (open|done), label, query.")]
    async fn list_tasks(&self, Parameters(p): Parameters<ListTasksParams>) -> Result<String, rmcp::ErrorData> {
        Self::ok(self.service.list(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Get a single todo by id.")]
    async fn get_task(&self, Parameters(p): Parameters<TaskIdParam>) -> Result<String, rmcp::ErrorData> {
        Self::ok(self.service.get(&p.id).await.map_err(Self::fail)?)
    }

    #[tool(description = "Create a new local todo. Fields: title (required), body, labels, due_at (RFC3339).")]
    async fn create_task(&self, Parameters(p): Parameters<CreateTaskParams>) -> Result<String, rmcp::ErrorData> {
        Self::ok(self.service.create(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Update a todo. Fields: id (required), title, body, status, labels, due_at, clear_due.")]
    async fn update_task(&self, Parameters(p): Parameters<UpdateTaskParams>) -> Result<String, rmcp::ErrorData> {
        Self::ok(self.service.update(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Mark a todo done by id.")]
    async fn complete_task(&self, Parameters(p): Parameters<TaskIdParam>) -> Result<String, rmcp::ErrorData> {
        Self::ok(self.service.complete(p).await.map_err(Self::fail)?)
    }

    #[tool(description = "Delete (tombstone) a todo by id.")]
    async fn delete_task(&self, Parameters(p): Parameters<TaskIdParam>) -> Result<String, rmcp::ErrorData> {
        self.service.delete(p).await.map_err(Self::fail)?;
        Self::ok(serde_json::json!({ "deleted": true }))
    }
}

impl ServerHandler for TodoServer {}
```

(If `#[tool_router(server_handler)]` already supplies `ServerHandler`, remove the explicit `impl ServerHandler`. Adapt per resolved rmcp API and note it.)

- [ ] **Step 2: Rewrite main.rs**

Replace `crates/mcp/src/main.rs`:

```rust
mod config;
mod dto;
mod server;
mod service;

use newt_todo_core::Store;
use rmcp::transport::stdio;
use rmcp::ServiceExt;

use crate::server::TodoServer;
use crate::service::TaskService;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db_url = config::resolve_db_url()?;
    let store = Store::connect(&db_url).await?;
    let server = TodoServer::new(TaskService::new(store));

    let running = server.serve(stdio()).await?;
    running.waiting().await?;
    Ok(())
}
```

- [ ] **Step 3: Build**

Run: `cargo build -p newt-todo-mcp`
Expected: builds. Fix any rmcp API mismatches per the note above until it compiles cleanly.

- [ ] **Step 4: Stdio smoke test**

Create `crates/mcp/tests/smoke.rs`:

```rust
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// Sends an MCP `initialize` request over stdio and asserts the server responds
/// with a JSON-RPC result containing serverInfo. Proves the binary speaks MCP.
#[test]
fn server_responds_to_initialize() {
    let db = tempfile::NamedTempFile::new().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_newt-todo-mcp"))
        .env("NEWT_TODO_DB", db.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let init = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-06-18",
            "capabilities": {},
            "clientInfo": { "name": "smoke", "version": "0.0.0" }
        }
    });

    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{}", init).unwrap();
    stdin.flush().unwrap();

    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    child.kill().ok();

    assert!(line.contains("\"result\""), "expected a JSON-RPC result, got: {line}");
    assert!(line.contains("serverInfo"), "expected serverInfo in response, got: {line}");
}
```

Run: `cargo test -p newt-todo-mcp --test smoke`
Expected: PASS. If the MCP protocol version string or response framing differs in the resolved rmcp, adjust the request's `protocolVersion` and the assertions to match what the server actually emits (the goal: prove a real MCP handshake response). Note any change.

- [ ] **Step 5: Final checks + commit**

Run: `cargo test -p newt-todo-mcp` (all pass), `cargo clippy -p newt-todo-mcp --all-targets -- -D warnings` (clean), `cargo fmt`.

```bash
git add crates/mcp Cargo.lock
git commit -m "feat(mcp): rmcp stdio server exposing todo tools"
```

---

## Done Criteria

- `newt-todo-mcp` binary builds and speaks the MCP handshake over stdio (smoke test passes).
- Six tools wired: list_tasks, get_task, create_task, update_task, complete_task, delete_task.
- All tool logic unit-tested via `TaskService` (independent of rmcp).
- Shares the same SQLite DB as the future Tauri app via `NEWT_TODO_DB` / default data dir.

## Manual verification (after merge)

Register with Claude Code: `claude mcp add newt-todo -- /path/to/newt-todo-mcp` (or add to `.mcp.json`), then ask the agent to create/list todos and confirm they persist in the DB.

## Next Plan

Plan 3: Tauri 2 + Svelte UI shell (reads/writes the same `core` Store) with Paraglide i18n scaffolding.
