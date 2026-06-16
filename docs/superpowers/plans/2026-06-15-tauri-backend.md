# Plan 3a — Shared Service Crate + Tauri Backend

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Extract the tested task logic into a reusable `newt-todo-service` crate shared by the MCP binary and the new Tauri app, then scaffold the Tauri 2 desktop app exposing Tauri commands over that service.

**Architecture:** `crates/core` stays pure (domain + Store). New `crates/service` holds `TaskService`, DTOs, params, and DB-path resolution (with an optional `schemars` feature for MCP's JSON schemas). `crates/mcp` becomes a thin rmcp wrapper over the service. `apps/desktop` is a Tauri 2 app (Svelte+Bun frontend, scaffolded in Plan 3b's setup) whose Rust side holds a `TaskService` in managed state and exposes commands. Linux desktop first; Android deferred.

**Tech Stack:** Rust workspace; `newt-todo-service` (core, time, uuid, serde, serde_json, anyhow, directories, optional schemars); tauri 2; existing rmcp mcp crate.

---

## Task 1: Extract `crates/service` from `crates/mcp`

This is a refactor of EXISTING, tracked code. Move files, adjust module paths/imports, add an optional `schemars` feature, and rewire `mcp`. Keep ALL existing tests green (they move with the code).

**Files:**
- Create: `crates/service/Cargo.toml`, `crates/service/src/lib.rs`
- Move: `crates/mcp/src/config.rs` → `crates/service/src/db.rs`; `crates/mcp/src/dto.rs` → `crates/service/src/dto.rs`; `crates/mcp/src/service.rs` → `crates/service/src/service.rs`
- Modify: root `Cargo.toml` (add member + workspace deps), `crates/mcp/Cargo.toml`, `crates/mcp/src/main.rs`, `crates/mcp/src/server.rs`

- [ ] **Step 1: Create the service crate manifest**

In root `Cargo.toml`, add `"crates/service"` to `members`. Add to `[workspace.dependencies]`:
```toml
newt-todo-service = { path = "crates/service" }
schemars = "1"
```

Create `crates/service/Cargo.toml`:
```toml
[package]
name = "newt-todo-service"
version = "0.1.0"
edition.workspace = true
license.workspace = true

[features]
# Enables JsonSchema derives on the param structs (needed by the MCP crate).
schemars = ["dep:schemars"]

[dependencies]
newt-todo-core = { workspace = true }
time = { workspace = true }
uuid = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
anyhow = { workspace = true }
directories = { workspace = true }
schemars = { workspace = true, optional = true }

[dev-dependencies]
tokio = { workspace = true, features = ["macros", "rt-multi-thread"] }
tempfile = { workspace = true }
```

- [ ] **Step 2: Move the three source files**

`git mv crates/mcp/src/config.rs crates/service/src/db.rs`
`git mv crates/mcp/src/dto.rs crates/service/src/dto.rs`
`git mv crates/mcp/src/service.rs crates/service/src/service.rs`

Create `crates/service/src/lib.rs`:
```rust
pub mod db;
pub mod dto;
pub mod service;

pub use db::{db_url_for_path, resolve_db_url};
pub use dto::{
    CreateTaskParams, ListTasksParams, TaskDto, TaskIdParam, UpdateTaskParams,
};
pub use service::TaskService;
```

- [ ] **Step 3: Fix schemars derives to be feature-gated**

In `crates/service/src/dto.rs`, the structs currently use `use rmcp::schemars::JsonSchema;` + `#[derive(... JsonSchema)]` + `#[schemars(crate = "rmcp::schemars")]`. Change to use the service crate's OWN optional `schemars` dep so the service crate does not depend on rmcp:

- Remove `use rmcp::schemars::JsonSchema;`.
- For each param struct that derived `JsonSchema`, replace the derive with a feature-gated form and remove the `#[schemars(crate = ...)]` attribute. Example for `CreateTaskParams`:
```rust
#[derive(Debug, serde::Deserialize)]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub struct CreateTaskParams {
    // ...fields unchanged...
}
```
Apply the same pattern to `ListTasksParams`, `TaskIdParam`, `UpdateTaskParams`. `TaskDto` keeps `#[derive(Serialize)]` and ALSO add `#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]` to it (harmless, future-proof). Keep `parse_status` and all field definitions exactly as they were.

(Because the optional dep is named `schemars` and is the same version rmcp uses, no `#[schemars(crate = ...)]` attribute is needed, and rmcp's `Parameters<T>` bound is still satisfied when the feature is on.)

- [ ] **Step 4: Fix internal module paths in the moved files**

In `service.rs`: change `use crate::dto::{...};` — these still work since dto is a sibling module in the new crate (`crate::dto`). Verify `db.rs`/`dto.rs`/`service.rs` only reference `crate::` siblings and `newt_todo_core::...`, not `rmcp` or `mcp` paths. The service tests (in `service.rs`) and the db test (in `db.rs`) move unchanged.

- [ ] **Step 5: Rewire the mcp crate to depend on the service**

In `crates/mcp/Cargo.toml`:
- Remove the now-unused direct deps that moved out (anyhow stays — main.rs uses it; directories can be removed; uuid can be removed if unused; time stays only if used). Add: `newt-todo-service = { workspace = true, features = ["schemars"] }`. Keep `rmcp`, `serde`, `serde_json`, `tokio`, `newt-todo-core` (if still referenced), `anyhow`.

In `crates/mcp/src/main.rs`:
- Remove `mod config; mod dto; mod service;`. Keep `mod server;`.
- Replace `use crate::service::TaskService;` with `use newt_todo_service::TaskService;`.
- Replace `config::resolve_db_url()` with `newt_todo_service::resolve_db_url()`.

In `crates/mcp/src/server.rs`:
- Replace `use crate::dto::{...};` with `use newt_todo_service::{CreateTaskParams, ListTasksParams, TaskIdParam, UpdateTaskParams};`.
- Replace `use crate::service::TaskService;` with `use newt_todo_service::TaskService;`.
- The `Parameters<T>` bound now resolves T's `JsonSchema` via the service crate's schemars (feature enabled). No other change.

- [ ] **Step 6: Build and test**

Run: `cargo test --workspace`
Expected: ALL pass — the moved service+db tests now run under `newt-todo-service`, and the mcp smoke test still passes. Total test count unchanged (15).
Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "refactor: extract newt-todo-service crate shared by mcp and app"
```

---

## Task 2: Scaffold Tauri 2 app (executed by the coordinator, not a subagent)

> The coordinator performs this step directly because it is environment-sensitive (scaffolding tool, version resolution). Documented here for the record.

- [ ] Scaffold a Tauri 2 app under `apps/desktop` with a Svelte (TS) + Vite + Bun frontend (the frontend lives at `apps/desktop`, Tauri Rust at `apps/desktop/src-tauri`).
- [ ] Add `apps/desktop/src-tauri` to the workspace `members` (or keep it a standalone crate referencing the workspace — decide based on what `tauri init` generates; prefer workspace member with `newt-todo-service` as a path dep).
- [ ] Set the app identifier to `dev.newtthewolf.newt-todo`, product name `Newt Todo`.
- [ ] Verify `bun install` and `cargo build -p <tauri-crate>` (or `bun run tauri build --debug`) succeed on Linux.
- [ ] Commit the scaffold: `chore(app): scaffold tauri 2 + svelte desktop app`.

---

## Task 3: Tauri commands over `TaskService`

**Files:**
- Modify: `apps/desktop/src-tauri/Cargo.toml` (add `newt-todo-service`, `tokio`), `apps/desktop/src-tauri/src/lib.rs` (commands + managed state).

- [ ] **Step 1: Add the service dependency**

In `apps/desktop/src-tauri/Cargo.toml` `[dependencies]`, add:
```toml
newt-todo-service = { workspace = true }
tokio = { workspace = true, features = ["rt-multi-thread"] }
serde_json = { workspace = true }
```

- [ ] **Step 2: Define commands wrapping the service**

In the Tauri app's `lib.rs` (the `run()` setup), build a `TaskService` from `resolve_db_url()` during setup, store it in Tauri managed state, and expose async commands. Use this shape (adapt to the generated file's existing structure):

```rust
use newt_todo_service::{
    CreateTaskParams, ListTasksParams, TaskDto, TaskService, UpdateTaskParams,
};
use newt_todo_core::Store;
use tauri::State;

struct AppState {
    service: TaskService,
}

#[tauri::command]
async fn list_tasks(state: State<'_, AppState>, params: ListTasksParams) -> Result<Vec<TaskDto>, String> {
    state.service.list(params).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_task(state: State<'_, AppState>, params: CreateTaskParams) -> Result<TaskDto, String> {
    state.service.create(params).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn update_task(state: State<'_, AppState>, params: UpdateTaskParams) -> Result<TaskDto, String> {
    state.service.update(params).await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn complete_task(state: State<'_, AppState>, id: String) -> Result<TaskDto, String> {
    state.service
        .complete(newt_todo_service::TaskIdParam { id })
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_task(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.service
        .delete(newt_todo_service::TaskIdParam { id })
        .await
        .map_err(|e| e.to_string())
}
```

In `run()`, connect the Store and manage the state before `.run(...)`:
```rust
let db_url = newt_todo_service::resolve_db_url().expect("resolve db url");
let store = tauri::async_runtime::block_on(Store::connect(&db_url)).expect("connect store");
let service = TaskService::new(store);

tauri::Builder::default()
    .manage(AppState { service })
    .invoke_handler(tauri::generate_handler![
        list_tasks, create_task, update_task, complete_task, delete_task
    ])
    // ...existing plugins/setup...
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

(Adapt `block_on`/runtime to whatever the generated template uses. If the template uses `#[cfg_attr(mobile, tauri::mobile_entry_point)]`, keep it.)

- [ ] **Step 3: Build**

Run: `cargo build -p <tauri-crate>`
Expected: compiles. Fix any Tauri 2 command-signature mismatches (async commands must return `Result<_, _>` where the error is `Serialize` — `String` works).

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/src-tauri
git commit -m "feat(app): tauri commands over TaskService"
```

---

## Done Criteria
- `newt-todo-service` crate exists; `core` stays pure; `mcp` is a thin wrapper; all 15 prior tests still pass.
- Tauri app builds on Linux with five commands wired to the shared service, sharing the same SQLite DB.

## Next Plan
Plan 3b: Tailwind v4 + Paraglide i18n + Svelte task-list UI consuming these commands, with Vitest tests; then run the app to verify.
