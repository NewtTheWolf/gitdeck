# Plan: Phase K — multi-user server + local-first sync

Date: 2026-06-16 · Branch: `master`

## User decisions
- **Sync model:** local-first + background sync. Every device keeps its local SQLite store
  (offline-capable); a sync engine reconciles boards/todos with the server. LWW conflicts
  (core already has `dirty` + `updated_at` + tombstones on tasks). gRPC server-streaming for
  live cross-device updates.
- **Server:** **multi-user** — real accounts/login on the hosted server, per-user data isolation,
  per-user GitHub token (replaces the single `GITHUB_TOKEN`).

## Multi-tenancy model
Shared DB + **`user_id` scoping** (Postgres-friendly; fine on SQLite). The auth/users system is a
**server-only** concern → it lives in `crates/api` (NOT `crates/core`, which the offline desktop
shares and must stay single-tenant/local). The server scopes the synced entities by the
authenticated user.

## Sub-phases (each committed + green)
### K1 — server auth (backend, crates/api) — START HERE
- SeaORM `users` table (server db): `id, username UNIQUE, password_hash, created_at` (argon2).
- Per-user GitHub token store (a `user_tokens` table keyed by user_id) — replaces `EnvTokenStore`
  for multi-user; keep EnvTokenStore as a single-user fallback when auth is disabled.
- Proto: `Register(username,password)`, `Login(username,password) -> {token}`,
  `SetGithubToken(token)` (auth'd). Token = a signed session (JWT, `jsonwebtoken`) or opaque.
- tonic auth interceptor: validate the bearer token → inject `user_id` into request extensions;
  unauthenticated requests to data RPCs are rejected when auth mode is on.
- Server resolves the per-request user and uses THAT user's GitHub token for GitHub-backed RPCs.
- Config: `GITDECK_AUTH=multi|single` (single = today's behavior, BC), `GITDECK_JWT_SECRET`.
- Tests: register→login→authed call; wrong password rejected; token gates a data RPC.
- NOTE: data RPCs aren't user-scoped yet in K1 (that's K2) — K1 lands the auth machinery.

### K2 — user-scoped data + sync schema (backend)
- Add `user_id` (+ ensure `updated_at`/`dirty`/`deleted` tombstones) to the SYNCED entities
  (tasks, boards, board_columns, board_cards). Boards/columns/cards currently lack sync fields →
  add them (migration). The server scopes all board/todo reads+writes by the authed `user_id`.
- Decide where scoping lives: a thin server-side repository over the SeaORM entities that always
  filters by user_id (keeps `crates/core` single-tenant for the desktop; the server uses its own
  user-scoped queries OR a `user_id` param threaded into a server-only Store variant).

### K3 — sync protocol + engine (backend + client)
- Proto: `PullChanges(since_cursor) -> {changes, cursor}`, `PushChanges(changes) -> {applied}`,
  and `WatchChanges(stream)` server-streaming for live updates. "Changes" = upserts/tombstones of
  boards/columns/cards/tasks with `updated_at`.
- Server: apply pushes with LWW (compare `updated_at`), return pulls since a cursor, fan out a
  stream on change.
- Desktop: a sync engine (always local-first): on connect + on local change + on stream event,
  push dirty rows, pull remote since cursor, merge LWW into the local store, clear dirty. Replace
  J2's thin-client remote mode for boards/todos with this local-first sync (GitHub reads can stay
  direct-to-server or local provider).
- Conflict: LWW by `updated_at`; tombstones propagate deletes; reuse `core::sync` patterns.

### K4 — desktop auth UX
- Settings: when mode=remote, register/login to the server, store the session token, set your
  GitHub token on the server. Surface sync status (synced / offline / syncing).

## Constraints / notes
- Offline-first preserved: desktop always works against its local store; sync is additive.
- `crates/core` stays single-tenant + local (no server-user concepts leak in).
- Single-user hosting still works (`GITDECK_AUTH=single` + `GITHUB_TOKEN`) — fully BC with J1/J2.
