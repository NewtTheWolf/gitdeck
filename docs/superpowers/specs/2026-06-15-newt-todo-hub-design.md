# Newt Todo Hub — Design Spec

**Date:** 2026-06-15
**Status:** Approved for planning

## Summary

A personal, cross-platform task hub that connects to **ClickUp**, **GitHub**, and
**Codeberg** (Forgejo) via OAuth, mirrors their issues/tasks into one unified local
list alongside private local-only todos, and exposes an **MCP server** so an agent
(e.g. Claude Code) can manage those todos. Built as a **Tauri 2** app (Linux +
Android focus, multi-platform-ready) with a **Svelte + TailwindCSS** UI, sharing a
pure-Rust `core` crate with a standalone stdio **MCP binary**.

Single-user, self-hosted-friendly, no monthly cost — the user registers their own
OAuth apps per provider.

## Goals

- Connect ClickUp, GitHub, Codeberg via OAuth.
- Unified local list of all issues/tasks **plus** private local-only todos (hybrid).
- Offline-capable (important for Android) with background sync back to providers.
- MCP server (stdio) so an agent can list/create/update/complete/sync todos.
- i18n from day one (default English).
- Fully tested (TDD throughout).

## Non-Goals (this MVP)

- **Projects support** and **Dashboards** — explicitly deferred to a later iteration.
  The data model leaves room (`project_id`, `list_projects` on the trait) but no
  project sync/UI ships in MVP.
- iOS / Windows / macOS builds (multi-platform-ready, but Linux + Android are the
  shipping targets).
- Distributing OAuth client secrets — secrets are user-provided and local only.
- Multi-user / team features.

## Tech Stack

- **Tauri 2** (Rust core), targets: Linux desktop + Android.
- **Svelte + TailwindCSS** frontend, **Bun** package manager.
- **Tauri CLI** + **Cargo CLI** for tooling.
- **SQLite** local store.
- **rmcp** (Rust MCP SDK) for the MCP binary.
- **Paraglide (inlang)** for frontend i18n.
- **keyring** crate / `tauri-plugin-stronghold` for secret storage.

## Architecture

Cargo workspace so the app and the MCP binary share one core:

```
todo/
├─ crates/
│  ├─ core/        # Domain (Task, Account, Provider trait), SQLite, Sync, OAuth/Token
│  ├─ providers/   # provider-github, provider-codeberg, provider-clickup (impl Provider)
│  └─ mcp/         # stdio MCP binary (rmcp), depends on core
├─ apps/
│  └─ desktop/     # Tauri 2 (src-tauri), targets: linux + android
└─ ui/             # Svelte + Tailwind + Bun (frontend)
```

- **`core`** is pure Rust (no Tauri dependency) → testable, reused by **both** the
  app and the MCP binary. Both talk to the **same local SQLite**.
- **Data flow:** Provider API ⇄ Sync engine ⇄ SQLite ⇄ (Tauri commands → Svelte UI)
  **and** (MCP tools → agent).

### `Provider` trait

The extensibility seam. Each provider maps its API onto the unified core model.

```rust
trait Provider {
    async fn list_tasks(&self) -> Result<Vec<RemoteTask>>;
    async fn create_task(&self, draft: TaskDraft) -> Result<RemoteTask>;
    async fn update_task(&self, remote_id: &str, patch: TaskPatch) -> Result<RemoteTask>;
    // list_projects(...) — later
}
```

## Data Model (SQLite)

```
Account
  id, provider (github|codeberg|clickup), display_name,
  base_url (for self-hosted / Codeberg), token_ref (→ keychain), created_at

Task
  id (local UUID),
  source: NULL | { account_id, remote_id, html_url },   -- NULL = private local-only todo
  title, body, status (open|done), labels[], due_at,
  project_id (NULL for now),
  remote_updated_at,    -- last state per provider
  local_updated_at,     -- last state per us
  dirty (bool),         -- changed locally, not yet pushed
  deleted (bool)        -- tombstone
```

Private todos (`source = NULL`) and mirrored issues live in the **same table** →
one unified list (the hybrid requirement).

## Sync Engine

1. **Pull:** per account `list_tasks` → upsert by `(account_id, remote_id)`.
2. **Push:** all `dirty` tasks → `create/update_task` → clear `dirty`, refresh
   `remote_updated_at`.
3. **Conflict** (remote *and* local changed since last sync): **Last-Write-Wins** by
   timestamp, but the loser is **not silently dropped** — a conflict flag + a copy of
   the foreign state are attached to the task and surfaced in the UI.
4. **Trigger:** manual (UI button / MCP `sync` tool) **plus** background interval.
   Offline → changes accumulate as `dirty`, pushed on next online.

**Promote private todo → issue:** assigning an account to a local task turns it into a
real issue on next push (`source` gets set). Supported by the model; nice-to-have.

## OAuth & Secrets

Single-user → the user registers their **own** OAuth apps per provider and enters
`client_id` (+ secret where required) in settings. **Everything is stored in the OS
keychain** (`keyring` / `tauri-plugin-stronghold`) — never plaintext, never in code.

**Unified flow: OAuth 2.0 Authorization Code (redirect)** for all three providers —
PKCE where the provider supports it, `client_secret` (stored in keychain) where it
doesn't. This gives a consistent "click → authorize → back" UX and one code path.

| Provider | Flow | PKCE? | Secret? |
|---|---|---|---|
| **GitHub** | Auth Code | No (GitHub has no PKCE) | Yes (secret in keychain) |
| **Codeberg** (Forgejo) | Auth Code + PKCE (public client) | Yes | No |
| **ClickUp** | Auth Code | No | Yes (secret in keychain) |

**Redirect mechanics (same for all three):**
- Desktop: short-lived `127.0.0.1:PORT` loopback listener (`tauri-plugin-oauth` pattern)
  captures the `?code=…&state=…` callback.
- Android: custom-scheme deep link (`tauri-plugin-deep-link`), e.g.
  `dev.newtthewolf.newt-todo://oauth/callback`.
- `state` validated on return (CSRF); PKCE `code_verifier`/`code_challenge` for Codeberg.

> Note: GitHub's *Device Authorization Grant* (RFC 8628) was considered (no secret, no
> redirect) but rejected in favor of the unified redirect flow — since this is
> single-user with the user's own registered apps, storing `client_secret` locally is
> acceptable and the redirect UX is smoother.

**Token refresh** lives in `core`, transparent to app & MCP (Codeberg/ClickUp refresh
tokens; GitHub tokens are long-lived).

**MCP shares the same keychain tokens:** log in once in the app, the agent reuses them.

## MCP Tools (stdio, via `rmcp`)

```
list_accounts
list_tasks(filter?: status|account|label|query)
get_task(id)
create_task(title, body?, account?, labels?, due?)   -- omit account = private todo
update_task(id, {title?, body?, status?, labels?, due?})
complete_task(id)
delete_task(id)
sync(account?)                                        -- trigger pull+push
```

All go through `core` → SQLite → provider on next sync.

## i18n

- **Frontend:** Paraglide (inlang) — type-safe, tree-shakeable, default `en`,
  structured for more locales from day one.
- **Backend/MCP:** errors as stable **codes** (e.g. `auth.token_expired`), not fixed
  strings — UI translates; MCP returns code + English default message.

## Testing Strategy (fully tested, TDD)

- **`core`:** unit tests (domain, deterministic sync/conflict logic); provider clients
  against **mocked HTTP** (`wiremock`) — no real API calls in tests.
- **Sync engine:** scenario tests (offline→online, conflict, promote).
- **`mcp`:** integration test over the real stdio protocol (tool call → DB effect).
- **Frontend:** Vitest + Testing Library for Svelte components/stores.
- TDD throughout (red → green → refactor).

## MVP Boundary

In: all three providers (issues/tasks only), OAuth, hybrid local store + private todos,
sync, Svelte list UI, stdio MCP, i18n scaffolding, full test coverage.

Out (later): Projects support, Dashboards, additional OS targets.

## Open Questions / Risks

- ClickUp's hierarchy (Workspace > Space > Folder > List > Task) must be flattened to
  the unified `Task` model for MVP — decide which ClickUp scope (which List/Space) is
  synced; likely a per-account setting.
- Android keychain story (`tauri-plugin-stronghold` vs Android Keystore) needs a spike.
- Background sync on Android (battery/lifecycle constraints) — MVP may rely on
  foreground/manual sync first.
