# Plan: standalone gRPC server + remote desktop + multi-DB + sync

Date: 2026-06-16 · Branch: `master`

## User intent
Extract the API into a crate that is (a) embedded in the desktop app AND (b) hostable
standalone, so the desktop can connect to a remote server. "Full gitdeck support", backwards-
compatible. Transport: **gRPC + protobuf** (user choice — "geilo"). Standalone binary named
**`gitdeck-server`**. Multi-DB via **SeaORM** (SQLite for desktop/mobile, Postgres for the
hosted server). A **sync** feature (boards/todos across devices) is the motivation; it comes last.

## Decisions (locked with user)
- **Transport:** gRPC/protobuf. Webview can't do native gRPC → server = **tonic + tonic-web**
  (serves gRPC AND gRPC-Web, no Envoy); desktop remote client = **connect-web** (typed, generated).
- **Embedded mode:** desktop local/offline keeps **Tauri `invoke`** (no local server, mobile/offline,
  fully BC). gRPC-Web is used only when connected to a remote server. One frontend API interface,
  two transports; the `.proto` is the single source of truth for both.
- **Sync:** later phase (K). Foundation first.
- **Binary name:** `gitdeck-server`.
- **DB:** SeaORM for SQLite + Postgres — but as **phase J3**, after the server works on the
  current sqlx/SQLite store (the Store's public API stays, so this is low-ripple).

## Phases
### J1 — proto + `crates/api` (tonic) + `gitdeck-server` binary
- `proto/gitdeck/v1/*.proto` — the service contract: messages mirror the existing DTOs; RPCs
  mirror the existing Tauri commands (repos/issues/prs, boards CRUD, todos, accounts, notifications,
  CI workflow-runs, repo detail/releases/forks/contributors/languages, traffic, mentions,
  issue/PR detail + comments + reviews, triage writes, snapshots/digest). `gitdeck.v1` package.
- `crates/api` — `tonic` service impl that wraps a `TaskService`; `tonic-web` layer for gRPC-Web;
  CORS for the webview origin. `tonic-build` codegen (vendor `protoc` via `protobuf-src` so
  `cargo build` needs no system protoc). A library (`build_router(service) -> Router`) so it can be
  embedded OR served standalone.
- `gitdeck-server` binary (in `crates/api` or `apps/server`) — builds a headless `TaskService`
  (sqlite path + token from env) and serves. Env: `GITDECK_BIND` (default 127.0.0.1:50061),
  `GITDECK_DB` (default ./gitdeck.db), `GITHUB_TOKEN` (headless token source), optional
  `GITDECK_API_KEY` (bearer to protect the server).
- Headless token store: a `TokenStore` impl returning the env `GITHUB_TOKEN` (single-token
  hosting for v1; per-account server tokens later). Reuse `crates/auth` patterns.
- Tests: tonic service unit/integration test (in-process server + client hitting a couple RPCs,
  service backed by a wiremock'd provider or a fake). Keep `cargo test --workspace` green.

### J2 — desktop remote-connect (gRPC-Web)
- Frontend: generate the TS client from the proto (`buf` + connect-es). A transport abstraction in
  `lib/api.ts`: `mode = "local" (invoke) | "remote" (connect-web baseUrl)`. Settings UI to pick
  mode + server URL (+ API key). All hooks keep working; only the transport swaps.
- Keep invoke as default; remote is opt-in.

### J3 — SeaORM (SQLite + Postgres)
- Migrate `crates/core` Store to SeaORM entities + `sea-orm-migration` (port the 0001-0004 SQL).
- DB selected by connection string / feature: SQLite (desktop) and Postgres (server). Keep the
  Store's public method signatures so `service`/`api`/desktop don't change. All tests stay green
  (they're the safety net for this refactor).

### K — sync
- Server as source-of-truth for boards/todos/account-config. Client push/pull + server-streaming
  for live updates. Conflict strategy (LWW + dirty flags already exist in core::sync). Offline
  desktop keeps local store; reconciles on connect.

## Notes
- "no sidecar, mobile" constraint preserved: embedded uses in-process Rust (invoke), not a sidecar;
  the gRPC server is the OPTIONAL remote/hosted piece.
- gitdeck-`/api/*` REST compat (the earlier open question) is deferred/optional — can be added to
  `crates/api` as a thin REST facade later if wanted.
