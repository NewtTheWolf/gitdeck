# Plan 4b — Accounts + OAuth + Token store + Sync wiring

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. TDD. Checkboxes track steps.

**Goal:** Persist provider accounts, store OAuth tokens securely, implement the OAuth 2.0 Authorization-Code logic (PKCE + token exchange), and wire `GitHubProvider` + `core::sync` into a runnable `sync` (Tauri command + MCP tool). The testable backend is built and tested here; the interactive loopback/browser login + settings UI is a final, explicitly-unverifiable shell (Task 5).

**Architecture:**
- `core`: an `accounts` table (migration `0002`) + `Store` account CRUD.
- New `crates/auth` (`newt-todo-auth`): `TokenStore` trait (+ `MemoryTokenStore` for tests, `KeyringTokenStore` real) and an `oauth` module (PKCE pair, authorize-URL builder, `exchange_code` via reqwest — wiremock-tested).
- `service`: a `sync_account(account_id)` orchestration that loads the account + token, builds the right `Provider`, and runs `core::sync::sync_account`. Exposed as a Tauri command and an MCP `sync` tool.

**Scope:** GitHub only (Authorization-Code, `client_secret`, no PKCE — GitHub has none; PKCE plumbing built for Codeberg later). Codeberg/ClickUp providers themselves are Plan 5.

---

## Task 1: `accounts` table + Store CRUD

**Files:** `crates/core/migrations/0002_accounts.sql`, `crates/core/src/account.rs` (types), `crates/core/src/store.rs` (CRUD), `lib.rs`.

- [ ] **Step 1:** migration `0002_accounts.sql`:
```sql
CREATE TABLE accounts (
    id           TEXT PRIMARY KEY NOT NULL,
    provider     TEXT NOT NULL,           -- 'github' | 'codeberg' | 'clickup'
    display_name TEXT NOT NULL,
    base_url     TEXT,                     -- API base (null = provider default)
    config       TEXT NOT NULL DEFAULT '{}', -- provider-specific JSON, e.g. {"owner":"o","repo":"r"}
    created_at   TEXT NOT NULL
);
```

- [ ] **Step 2:** `crates/core/src/account.rs`:
```rust
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind { Github, Codeberg, Clickup }

impl ProviderKind {
    pub fn as_str(&self) -> &'static str {
        match self { ProviderKind::Github => "github", ProviderKind::Codeberg => "codeberg", ProviderKind::Clickup => "clickup" }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s { "github" => Some(Self::Github), "codeberg" => Some(Self::Codeberg), "clickup" => Some(Self::Clickup), _ => None }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: Uuid,
    pub provider: ProviderKind,
    pub display_name: String,
    pub base_url: Option<String>,
    pub config: serde_json::Value,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AccountDraft {
    pub provider: ProviderKind,
    pub display_name: String,
    pub base_url: Option<String>,
    pub config: serde_json::Value,
}
```
Register `pub mod account;` + re-export `Account`, `AccountDraft`, `ProviderKind` in `lib.rs`.

- [ ] **Step 3 (TDD):** add Store methods `create_account(AccountDraft, now) -> Account`, `list_accounts() -> Vec<Account>`, `get_account(id) -> Option<Account>`, `delete_account(id)`. Tests (in store.rs): create→get roundtrip (config JSON preserved), list returns all, delete removes, get-missing→None. Implement (store config via `serde_json::to_string`, parse back; timestamps RFC3339 like existing code). Run `cargo test -p newt-todo-core` → all pass.

- [ ] **Step 4:** commit `feat(core): accounts table and store CRUD`.

---

## Task 2: `crates/auth` — TokenStore + OAuth logic

**Files:** `crates/auth/Cargo.toml`, `src/lib.rs`, `src/tokens.rs`, `src/oauth.rs`; root `Cargo.toml` (member + deps `keyring = "3"`, `sha2 = "0.10"`, `base64 = "0.22"`, `url = "2"`).

- [ ] **Step 1:** crate manifest (`newt-todo-auth`) deps: `reqwest` (json, rustls-tls), `serde`, `serde_json`, `keyring`, `sha2`, `base64`, `url`, `async-trait`; dev: `tokio`, `wiremock`.

- [ ] **Step 2 — `tokens.rs` (TDD):**
```rust
use async_trait::async_trait;

#[derive(Debug, Clone, PartialEq)]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
}

#[derive(Debug, thiserror::Error)]
#[error("token store error: {0}")]
pub struct TokenError(pub String);

#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn save(&self, account_id: &str, token: &OAuthToken) -> Result<(), TokenError>;
    async fn load(&self, account_id: &str) -> Result<Option<OAuthToken>, TokenError>;
    async fn delete(&self, account_id: &str) -> Result<(), TokenError>;
}
```
Provide `MemoryTokenStore` (a `Mutex<HashMap<String, OAuthToken>>` — for tests) and `KeyringTokenStore` (real, uses `keyring::Entry::new("newt-todo", account_id)`, serializing the token as JSON). Test `MemoryTokenStore`: save→load roundtrip, load-missing→None, delete. (Do NOT unit-test `KeyringTokenStore` — it needs an OS secret service; just ensure it compiles.)

- [ ] **Step 3 — `oauth.rs` (TDD):**
```rust
pub struct PkcePair { pub verifier: String, pub challenge: String }

/// S256 PKCE: challenge = base64url(sha256(verifier)).
pub fn generate_pkce(verifier: impl Into<String>) -> PkcePair { /* sha2 + base64 url-safe no-pad */ }

pub struct AuthRequest<'a> {
    pub authorize_endpoint: &'a str,
    pub client_id: &'a str,
    pub redirect_uri: &'a str,
    pub scope: &'a str,
    pub state: &'a str,
    pub pkce_challenge: Option<&'a str>,
}
pub fn build_authorize_url(req: &AuthRequest) -> String { /* url crate, query params */ }

pub struct TokenRequest<'a> {
    pub token_endpoint: &'a str,
    pub client_id: &'a str,
    pub client_secret: Option<&'a str>,
    pub code: &'a str,
    pub redirect_uri: &'a str,
    pub pkce_verifier: Option<&'a str>,
}
pub async fn exchange_code(client: &reqwest::Client, req: &TokenRequest<'_>)
    -> Result<OAuthToken, TokenError>;
```
Tests (deterministic, no network for the first two):
1. `generate_pkce` with a known verifier produces the known S256 challenge (precompute the expected base64url string for verifier `"test-verifier-1234567890"`; assert exact match — base64url, no padding).
2. `build_authorize_url` includes `response_type=code`, `client_id`, `redirect_uri` (url-encoded), `scope`, `state`, and `code_challenge`+`code_challenge_method=S256` when a challenge is given; omits PKCE params when `None`.
3. `exchange_code` against a `wiremock` server: mock `POST /login/oauth/access_token` returning `{"access_token":"gho_x","token_type":"bearer"}` (GitHub returns form or JSON — send `Accept: application/json` so it returns JSON); assert the returned `OAuthToken.access_token == "gho_x"`. The request must send `Accept: application/json` and form/body params (`client_id`, `client_secret`, `code`, `redirect_uri`, and `code_verifier` when present).

- [ ] **Step 4:** `cargo test -p newt-todo-auth` → pass. Commit `feat(auth): token store + oauth pkce/url/exchange`.

---

## Task 3: sync orchestration + MCP tool + Tauri command

**Files:** `crates/service` (add `sync` module + deps on `newt-todo-providers`, `newt-todo-auth`); `crates/mcp/src/server.rs` (sync tool); `apps/desktop/src-tauri/src/lib.rs` (sync command + account commands).

- [ ] **Step 1 — service sync (TDD):** In `crates/service`, add `pub struct SyncService` (or free fn) that, given a `Store`, a `&dyn TokenStore`, and an `account_id`:
  1. loads the `Account`; matches `provider`:
     - `Github` → read `config.owner`/`config.repo`, load token, build `GitHubProvider::new(client, base_url.unwrap_or("https://api.github.com"), token.access_token, owner, repo)`.
     - others → `anyhow::bail!("provider not yet supported")` for now.
  2. runs `core::sync::sync_account(store, account_id, &provider, now)` and returns the `SyncReport` (`{pulled, pushed}`).
  Add `newt-todo-providers` + `newt-todo-auth` as service deps.
  **Test:** with a `MemoryTokenStore` seeded with a token + an account whose `config` points at a wiremock GitHub server (`base_url` = server.uri()), call the sync fn and assert `pulled >= 1` after mocking `GET /repos/o/r/issues`. (This exercises account→provider→sync end to end with mocked HTTP.)

- [ ] **Step 2 — expose account management in the service** (thin): `create_account`, `list_accounts`, `delete_account` passthroughs returning DTO-friendly types (the `Account` already derives Serialize).

- [ ] **Step 3 — MCP `sync` tool + account tools:** in `crates/mcp/src/server.rs` add tools `list_accounts()` and `sync(account_id: String)` that call the service. (The mcp `TodoServer` will need a `TokenStore` — use `KeyringTokenStore` in `main.rs`.) Build; the existing smoke test still passes.

- [ ] **Step 4 — Tauri commands:** in `apps/desktop/src-tauri/src/lib.rs` add `list_accounts`, `create_account`, `delete_account`, and `sync_account(id)` commands wired to the service + a `KeyringTokenStore` in managed state. `cargo build -p newt-todo-desktop`.

- [ ] **Step 5:** `cargo test --workspace` + clippy clean + fmt. Commit `feat: sync orchestration with mcp tool and tauri command`.

---

## Task 4: interactive login orchestration + settings UI (UNVERIFIABLE HERE — build, do not claim it works)

> The loopback OAuth round-trip (browser + `tauri-plugin-oauth`) and the Svelte settings UI cannot be verified in this environment (no real GitHub app creds; WebKitGTK won't render here). Build it cleanly, keep logic testable where possible, and report it as **built but unverified** — the user verifies on their machine with their own GitHub OAuth app.

- [ ] Add `tauri-plugin-oauth` + `tauri-plugin-opener`. A Tauri command `start_github_login(client_id, client_secret, owner, repo)` that: starts the loopback server, builds the authorize URL (`scope=repo`, random `state`), opens the browser, awaits the callback, validates `state`, calls `auth::exchange_code`, saves the token via `KeyringTokenStore`, and creates the account. 
- [ ] A minimal Svelte `settings` route: form (client_id/secret/owner/repo) → "Connect GitHub", a list of accounts with a "Sync" button (calls `sync_account`), and surfacing the `SyncReport`. i18n strings added to `messages/{en,de}.json`.
- [ ] `cargo build` + `bun run build` succeed. Report as built-but-unverified.

---

## Done Criteria
- Accounts persist; tokens abstracted behind `TokenStore` (in-memory tested, keyring real).
- OAuth PKCE/url/exchange unit + wiremock tested.
- `sync` runs account→provider→`sync_account` end-to-end against mocked HTTP; exposed as MCP tool + Tauri command.
- Tasks 1-3 fully tested; Task 4 built but explicitly unverified.

## After this
Plan 5: Codeberg (Auth Code + PKCE) + ClickUp providers, reusing this OAuth + sync machinery.
