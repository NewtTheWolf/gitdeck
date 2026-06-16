# Foundation: Workspace + Core + Local Task CRUD — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the Cargo workspace and a pure-Rust `core` crate with a SQLite-backed store supporting local-only todos (create/get/list/update/delete), fully tested.

**Architecture:** Cargo workspace. `crates/core` is pure async Rust (tokio) with no Tauri/UI dependency, so it is reused later by both the Tauri app and the MCP binary. SQLite via `sqlx` (runtime query API, no compile-time DB needed). All store methods take an explicit `now` timestamp for deterministic tests. Tests run against in-memory SQLite.

**Tech Stack:** Rust 1.96, sqlx (sqlite, runtime-tokio), tokio, uuid, time, serde_json, thiserror. Plan 4 adds the `source`/provider fields' behavior; this plan only exercises local todos (`source = None`).

---

## File Structure

```
todo/
├─ Cargo.toml                      # workspace manifest (members = ["crates/core"])
└─ crates/
   └─ core/
      ├─ Cargo.toml
      ├─ migrations/
      │  └─ 0001_init.sql          # tasks table
      └─ src/
         ├─ lib.rs                 # re-exports, CoreError
         ├─ domain.rs              # Task, TaskStatus, SourceRef, TaskDraft, TaskPatch, TaskFilter
         └─ store.rs               # Store: connect + CRUD
```

Each file has one responsibility: `domain.rs` = data types (no I/O), `store.rs` = persistence, `lib.rs` = crate surface + error type.

---

## Task 0: Workspace + core crate scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`

- [ ] **Step 1: Create workspace manifest**

Create `Cargo.toml`:

```toml
[workspace]
resolver = "2"
members = ["crates/core"]

[workspace.package]
edition = "2021"
license = "MIT"

[workspace.dependencies]
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
sqlx = { version = "0.8", default-features = false, features = ["runtime-tokio", "sqlite", "time"] }
uuid = { version = "1", features = ["v4", "serde"] }
time = { version = "0.3", features = ["serde", "formatting", "parsing", "macros"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

- [ ] **Step 2: Create core crate manifest**

Create `crates/core/Cargo.toml`:

```toml
[package]
name = "newt-todo-core"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[dependencies]
tokio = { workspace = true }
sqlx = { workspace = true }
uuid = { workspace = true }
time = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
```

- [ ] **Step 3: Create minimal lib.rs**

Create `crates/core/src/lib.rs`:

```rust
pub mod domain;
pub mod store;

pub use domain::{SourceRef, Task, TaskDraft, TaskFilter, TaskPatch, TaskStatus};
pub use store::Store;

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("task not found: {0}")]
    NotFound(uuid::Uuid),
}
```

- [ ] **Step 4: Verify it builds (modules empty for now will fail — create stubs)**

Create `crates/core/src/domain.rs` with a single line `// types added in Task 1` and `crates/core/src/store.rs` with `// store added in Task 2`.

Run: `cargo build`
Expected: FAIL — `domain` is empty so `pub use domain::{...}` is unresolved. This is expected; Task 1 fills it.

- [ ] **Step 5: Commit scaffold**

Temporarily comment out the `pub use` lines in `lib.rs` so the scaffold compiles, then:

Run: `cargo build`
Expected: PASS (empty crate compiles).

```bash
git add Cargo.toml crates/core
git commit -m "chore: scaffold cargo workspace and core crate"
```

---

## Task 1: Domain types

**Files:**
- Modify: `crates/core/src/domain.rs`
- Modify: `crates/core/src/lib.rs` (restore the `pub use` lines)

- [ ] **Step 1: Write the failing test**

Append to `crates/core/src/domain.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_defaults_are_empty() {
        let d = TaskDraft::new("Buy milk");
        assert_eq!(d.title, "Buy milk");
        assert_eq!(d.body, "");
        assert!(d.labels.is_empty());
        assert!(d.due_at.is_none());
    }

    #[test]
    fn status_roundtrips_as_str() {
        assert_eq!(TaskStatus::Open.as_str(), "open");
        assert_eq!(TaskStatus::Done.as_str(), "done");
        assert_eq!(TaskStatus::from_str("done"), Some(TaskStatus::Done));
        assert_eq!(TaskStatus::from_str("nope"), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p newt-todo-core domain`
Expected: FAIL — `TaskDraft`, `TaskStatus` not defined.

- [ ] **Step 3: Write the types**

Replace the top of `crates/core/src/domain.rs` (above the `#[cfg(test)]` block) with:

```rust
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub type TaskId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Open,
    Done,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Open => "open",
            TaskStatus::Done => "done",
        }
    }
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open" => Some(TaskStatus::Open),
            "done" => Some(TaskStatus::Done),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    pub account_id: Uuid,
    pub remote_id: String,
    pub html_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub source: Option<SourceRef>,
    pub title: String,
    pub body: String,
    pub status: TaskStatus,
    pub labels: Vec<String>,
    pub due_at: Option<OffsetDateTime>,
    pub local_updated_at: OffsetDateTime,
    pub dirty: bool,
    pub deleted: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaskDraft {
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
    pub due_at: Option<OffsetDateTime>,
}

impl TaskDraft {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            body: String::new(),
            labels: Vec::new(),
            due_at: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub body: Option<String>,
    pub status: Option<TaskStatus>,
    pub labels: Option<Vec<String>>,
    /// Outer Option = "field present in patch"; inner Option = "set to None".
    pub due_at: Option<Option<OffsetDateTime>>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TaskFilter {
    pub status: Option<TaskStatus>,
    pub label: Option<String>,
    pub query: Option<String>,
    pub include_deleted: bool,
}
```

- [ ] **Step 4: Restore lib.rs exports and run tests**

Uncomment the `pub use domain::{...}` and `pub use store::Store;` lines in `lib.rs`. (Store still missing — temporarily keep the `store` `pub use` commented until Task 2; restore the `domain` exports now.)

Run: `cargo test -p newt-todo-core domain`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/domain.rs crates/core/src/lib.rs
git commit -m "feat(core): add task domain types"
```

---

## Task 2: Store connect + migration

**Files:**
- Create: `crates/core/migrations/0001_init.sql`
- Modify: `crates/core/src/store.rs`
- Modify: `crates/core/src/lib.rs` (restore `pub use store::Store;`)

- [ ] **Step 1: Write the migration**

Create `crates/core/migrations/0001_init.sql`:

```sql
CREATE TABLE tasks (
    id                TEXT PRIMARY KEY NOT NULL,
    account_id        TEXT,
    remote_id         TEXT,
    html_url          TEXT,
    title             TEXT NOT NULL,
    body              TEXT NOT NULL DEFAULT '',
    status            TEXT NOT NULL DEFAULT 'open',
    labels            TEXT NOT NULL DEFAULT '[]',
    due_at            TEXT,
    project_id        TEXT,
    remote_updated_at TEXT,
    local_updated_at  TEXT NOT NULL,
    dirty             INTEGER NOT NULL DEFAULT 0,
    deleted           INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE UNIQUE INDEX idx_tasks_source ON tasks(account_id, remote_id)
    WHERE account_id IS NOT NULL AND remote_id IS NOT NULL;
```

- [ ] **Step 2: Write the failing test**

Replace `crates/core/src/store.rs` with:

```rust
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use crate::CoreError;

pub struct Store {
    pool: SqlitePool,
}

impl Store {
    pub async fn connect(url: &str) -> Result<Self, CoreError> {
        let pool = SqlitePoolOptions::new().connect(url).await?;
        sqlx::migrate!("./migrations").run(&pool).await
            .map_err(|e| CoreError::Db(sqlx::Error::Migrate(Box::new(e))))?;
        Ok(Self { pool })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn mem_store() -> Store {
        Store::connect("sqlite::memory:").await.unwrap()
    }

    #[tokio::test]
    async fn connect_runs_migration_and_table_exists() {
        let store = mem_store().await;
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tasks")
            .fetch_one(&store.pool)
            .await
            .unwrap();
        assert_eq!(count, 0);
    }
}
```

- [ ] **Step 3: Run test to verify it fails, then passes**

Restore `pub use store::Store;` in `lib.rs`.

Run: `cargo test -p newt-todo-core store::tests::connect_runs_migration`
Expected: PASS (migration runs, empty table). If it fails to compile first, fix imports until it compiles, then it should pass.

- [ ] **Step 4: Verify whole crate builds**

Run: `cargo test -p newt-todo-core`
Expected: PASS (domain + store tests).

- [ ] **Step 5: Commit**

```bash
git add crates/core/migrations crates/core/src/store.rs crates/core/src/lib.rs
git commit -m "feat(core): sqlite store connect with migration"
```

---

## Task 3: create_task + get_task

**Files:**
- Modify: `crates/core/src/store.rs`

- [ ] **Step 1: Write the failing test**

Add inside the `tests` module in `store.rs`:

```rust
    use crate::domain::{TaskDraft, TaskStatus};
    use time::macros::datetime;

    #[tokio::test]
    async fn create_then_get_roundtrips() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let mut draft = TaskDraft::new("Buy milk");
        draft.body = "2 litres".into();
        draft.labels = vec!["home".into(), "shopping".into()];

        let created = store.create_task(draft, now).await.unwrap();
        assert_eq!(created.title, "Buy milk");
        assert_eq!(created.status, TaskStatus::Open);
        assert!(created.source.is_none());
        assert!(!created.dirty);
        assert_eq!(created.local_updated_at, now);

        let fetched = store.get_task(created.id).await.unwrap().unwrap();
        assert_eq!(fetched, created);
    }

    #[tokio::test]
    async fn get_missing_returns_none() {
        let store = mem_store().await;
        let missing = uuid::Uuid::new_v4();
        assert!(store.get_task(missing).await.unwrap().is_none());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p newt-todo-core store::tests::create_then_get`
Expected: FAIL — `create_task` / `get_task` not defined.

- [ ] **Step 3: Implement create_task, get_task, and the row mapper**

Add to `impl Store` (above the `#[cfg(test)]` block) in `store.rs`. First add imports at the top of the file:

```rust
use sqlx::Row;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::domain::{SourceRef, Task, TaskDraft, TaskStatus};
```

Then the methods:

```rust
impl Store {
    pub async fn create_task(
        &self,
        draft: TaskDraft,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let id = Uuid::new_v4();
        let labels_json = serde_json::to_string(&draft.labels)?;
        let due = draft.due_at.map(|d| d.format(&Rfc3339)).transpose()
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad due_at".into())))?;
        let updated = now.format(&Rfc3339)
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad now".into())))?;

        sqlx::query(
            "INSERT INTO tasks (id, title, body, status, labels, due_at, local_updated_at, dirty, deleted)
             VALUES (?, ?, ?, 'open', ?, ?, ?, 0, 0)",
        )
        .bind(id.to_string())
        .bind(&draft.title)
        .bind(&draft.body)
        .bind(&labels_json)
        .bind(&due)
        .bind(&updated)
        .execute(&self.pool)
        .await?;

        self.get_task(id).await?.ok_or(CoreError::NotFound(id))
    }

    pub async fn get_task(&self, id: Uuid) -> Result<Option<Task>, CoreError> {
        let row = sqlx::query("SELECT * FROM tasks WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;
        row.map(map_row).transpose()
    }
}

fn map_row(row: sqlx::sqlite::SqliteRow) -> Result<Task, CoreError> {
    let id: String = row.try_get("id")?;
    let account_id: Option<String> = row.try_get("account_id")?;
    let remote_id: Option<String> = row.try_get("remote_id")?;
    let html_url: Option<String> = row.try_get("html_url")?;
    let title: String = row.try_get("title")?;
    let body: String = row.try_get("body")?;
    let status: String = row.try_get("status")?;
    let labels: String = row.try_get("labels")?;
    let due_at: Option<String> = row.try_get("due_at")?;
    let local_updated_at: String = row.try_get("local_updated_at")?;
    let dirty: i64 = row.try_get("dirty")?;
    let deleted: i64 = row.try_get("deleted")?;

    let source = match (account_id, remote_id) {
        (Some(account_id), Some(remote_id)) => Some(SourceRef {
            account_id: Uuid::parse_str(&account_id)
                .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad account_id".into())))?,
            remote_id,
            html_url,
        }),
        _ => None,
    };

    let parse_dt = |s: &str| {
        OffsetDateTime::parse(s, &Rfc3339)
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad datetime".into())))
    };

    Ok(Task {
        id: Uuid::parse_str(&id)
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad id".into())))?,
        source,
        title,
        body,
        status: TaskStatus::from_str(&status)
            .ok_or_else(|| CoreError::Db(sqlx::Error::Protocol("bad status".into())))?,
        labels: serde_json::from_str(&labels)?,
        due_at: due_at.as_deref().map(parse_dt).transpose()?,
        local_updated_at: parse_dt(&local_updated_at)?,
        dirty: dirty != 0,
        deleted: deleted != 0,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p newt-todo-core store`
Expected: PASS (create roundtrip + get-missing + earlier connect test).

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/store.rs
git commit -m "feat(core): create_task and get_task"
```

---

## Task 4: list_tasks with filter

**Files:**
- Modify: `crates/core/src/store.rs`

- [ ] **Step 1: Write the failing test**

Add inside the `tests` module:

```rust
    use crate::domain::TaskFilter;

    #[tokio::test]
    async fn list_filters_by_status_and_excludes_deleted() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let a = store.create_task(TaskDraft::new("open one"), now).await.unwrap();
        let _b = store.create_task(TaskDraft::new("open two"), now).await.unwrap();
        // mark a done directly for this test
        sqlx::query("UPDATE tasks SET status = 'done' WHERE id = ?")
            .bind(a.id.to_string())
            .execute(&store.pool).await.unwrap();

        let open = store
            .list_tasks(TaskFilter { status: Some(TaskStatus::Open), ..Default::default() })
            .await
            .unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].title, "open two");

        let all = store.list_tasks(TaskFilter::default()).await.unwrap();
        assert_eq!(all.len(), 2);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p newt-todo-core store::tests::list_filters`
Expected: FAIL — `list_tasks` not defined.

- [ ] **Step 3: Implement list_tasks**

Add to `impl Store` (use the `TaskFilter` import; add `use crate::domain::TaskFilter;` to the file's imports):

```rust
    pub async fn list_tasks(&self, filter: TaskFilter) -> Result<Vec<Task>, CoreError> {
        let mut sql = String::from("SELECT * FROM tasks WHERE 1 = 1");
        if !filter.include_deleted {
            sql.push_str(" AND deleted = 0");
        }
        if filter.status.is_some() {
            sql.push_str(" AND status = ?");
        }
        if filter.label.is_some() {
            // labels is a JSON array; match the quoted element
            sql.push_str(" AND labels LIKE ?");
        }
        if filter.query.is_some() {
            sql.push_str(" AND (title LIKE ? OR body LIKE ?)");
        }
        sql.push_str(" ORDER BY local_updated_at DESC");

        let mut q = sqlx::query(&sql);
        if let Some(status) = filter.status {
            q = q.bind(status.as_str().to_string());
        }
        if let Some(label) = &filter.label {
            q = q.bind(format!("%\"{}\"%", label));
        }
        if let Some(query) = &filter.query {
            let like = format!("%{}%", query);
            q = q.bind(like.clone()).bind(like);
        }

        let rows = q.fetch_all(&self.pool).await?;
        rows.into_iter().map(map_row).collect()
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p newt-todo-core store`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/store.rs
git commit -m "feat(core): list_tasks with status/label/query filter"
```

---

## Task 5: update_task (patch) — marks dirty

**Files:**
- Modify: `crates/core/src/store.rs`

- [ ] **Step 1: Write the failing test**

Add inside the `tests` module:

```rust
    use crate::domain::TaskPatch;

    #[tokio::test]
    async fn update_applies_patch_and_marks_dirty() {
        let store = mem_store().await;
        let t0 = datetime!(2026-06-15 12:00:00 UTC);
        let t1 = datetime!(2026-06-15 13:00:00 UTC);
        let task = store.create_task(TaskDraft::new("draft title"), t0).await.unwrap();

        let patch = TaskPatch {
            title: Some("new title".into()),
            status: Some(TaskStatus::Done),
            ..Default::default()
        };
        let updated = store.update_task(task.id, patch, t1).await.unwrap();

        assert_eq!(updated.title, "new title");
        assert_eq!(updated.status, TaskStatus::Done);
        assert!(updated.dirty, "local edits must mark the task dirty for later push");
        assert_eq!(updated.local_updated_at, t1);
        assert_eq!(updated.body, ""); // body untouched by the patch
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p newt-todo-core store::tests::update_applies_patch`
Expected: FAIL — `update_task` not defined.

- [ ] **Step 3: Implement update_task**

Add to `impl Store`:

```rust
    pub async fn update_task(
        &self,
        id: Uuid,
        patch: TaskPatch,
        now: OffsetDateTime,
    ) -> Result<Task, CoreError> {
        let mut current = self.get_task(id).await?.ok_or(CoreError::NotFound(id))?;

        if let Some(title) = patch.title {
            current.title = title;
        }
        if let Some(body) = patch.body {
            current.body = body;
        }
        if let Some(status) = patch.status {
            current.status = status;
        }
        if let Some(labels) = patch.labels {
            current.labels = labels;
        }
        if let Some(due) = patch.due_at {
            current.due_at = due;
        }

        let labels_json = serde_json::to_string(&current.labels)?;
        let due = current.due_at.map(|d| d.format(&Rfc3339)).transpose()
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad due_at".into())))?;
        let updated = now.format(&Rfc3339)
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad now".into())))?;

        sqlx::query(
            "UPDATE tasks SET title = ?, body = ?, status = ?, labels = ?, due_at = ?,
                              local_updated_at = ?, dirty = 1 WHERE id = ?",
        )
        .bind(&current.title)
        .bind(&current.body)
        .bind(current.status.as_str())
        .bind(&labels_json)
        .bind(&due)
        .bind(&updated)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        self.get_task(id).await?.ok_or(CoreError::NotFound(id))
    }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p newt-todo-core store`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/store.rs
git commit -m "feat(core): update_task applies patch and marks dirty"
```

---

## Task 6: delete_task (tombstone) + final crate check

**Files:**
- Modify: `crates/core/src/store.rs`

- [ ] **Step 1: Write the failing test**

Add inside the `tests` module:

```rust
    #[tokio::test]
    async fn delete_tombstones_and_hides_from_default_list() {
        let store = mem_store().await;
        let now = datetime!(2026-06-15 12:00:00 UTC);
        let task = store.create_task(TaskDraft::new("temp"), now).await.unwrap();

        store.delete_task(task.id, now).await.unwrap();

        let default_list = store.list_tasks(TaskFilter::default()).await.unwrap();
        assert!(default_list.is_empty(), "deleted tasks hidden by default");

        let with_deleted = store
            .list_tasks(TaskFilter { include_deleted: true, ..Default::default() })
            .await
            .unwrap();
        assert_eq!(with_deleted.len(), 1);
        assert!(with_deleted[0].deleted);
        assert!(with_deleted[0].dirty, "tombstone is dirty so a later sync can push the deletion");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p newt-todo-core store::tests::delete_tombstones`
Expected: FAIL — `delete_task` not defined.

- [ ] **Step 3: Implement delete_task**

Add to `impl Store`:

```rust
    pub async fn delete_task(&self, id: Uuid, now: OffsetDateTime) -> Result<(), CoreError> {
        let updated = now.format(&Rfc3339)
            .map_err(|_| CoreError::Db(sqlx::Error::Protocol("bad now".into())))?;
        let result = sqlx::query(
            "UPDATE tasks SET deleted = 1, dirty = 1, local_updated_at = ? WHERE id = ?",
        )
        .bind(&updated)
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Err(CoreError::NotFound(id));
        }
        Ok(())
    }
```

- [ ] **Step 4: Run the full test suite**

Run: `cargo test -p newt-todo-core`
Expected: PASS (all domain + store tests).

Also run: `cargo fmt && cargo clippy -p newt-todo-core -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add crates/core/src/store.rs
git commit -m "feat(core): delete_task tombstone"
```

---

## Done Criteria

- `cargo test` passes with: domain roundtrip, store connect/migration, create+get, list+filter, update+dirty, delete tombstone.
- `core` has zero Tauri/UI dependencies (reusable by app and MCP in later plans).
- All store methods take an explicit `now` → deterministic tests.

## Next Plan

Plan 2: MCP binary (`crates/mcp`, rmcp) exposing `list_tasks/create_task/update_task/complete_task/delete_task/get_task` over stdio, driving this `core` Store.
