# Plan 5 — Codeberg + ClickUp providers

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. TDD. Checkboxes track steps.

**Goal:** Add `CodebergProvider` (Forgejo/Gitea issues) and `ClickUpProvider` (ClickUp tasks) implementing `core::Provider`, wire both into `TaskService::sync`, all wiremock-tested. Then (flagged-unverified) generalize the interactive login + settings UI to pick a provider (Codeberg uses Auth-Code + PKCE).

**Architecture:** Both providers live in `crates/providers` alongside `GitHubProvider`, same shape (configurable `base_url` for wiremock). `TaskService::sync`'s provider match gains `Codeberg`/`Clickup` arms. The OAuth machinery in `crates/auth` already supports PKCE (`build_authorize_url` with a challenge, `exchange_code` with a verifier) — Codeberg just passes a PKCE pair.

**Known per-provider limits (MVP — document in code):**
- **Codeberg** (Gitea/Forgejo): labels on create/update need numeric label IDs, not names → push updates **title/body/state only**, skip labels. Pull reads label names fine. Auth header: `Authorization: token <token>`. API base: `{base_url}/api/v1`.
- **ClickUp:** tasks live in a **List** (account config needs `list_id`). Status is a custom per-list object → push updates **name/description only**, skip status/tags. Pull maps `status.type=="closed"` → Done. Auth header: `Authorization: <token>` (NO "Bearer"). Timestamps are **Unix-ms strings**. API base: `{base_url}/api/v2`.

---

## Task 1: `CodebergProvider`

**Files:** Create `crates/providers/src/codeberg.rs`; Modify `crates/providers/src/lib.rs`.

- [ ] **Step 1 (TDD):** wiremock tests in `codeberg.rs`:
  - `list_tasks`: mock `GET /api/v1/repos/o/r/issues` returning an array with one issue (`{"number":5,"title":"hi","body":"b","state":"open","html_url":"https://codeberg.org/o/r/issues/5","updated_at":"2026-06-15T10:00:00Z","labels":[{"name":"bug"}]}`) and one PR-like entry (`{"number":6,...,"pull_request":{}}`). Assert ONE `RemoteTask`, `remote_id=="5"`, `status==Open`, `labels==["bug"]`.
  - `update_task`: mock `PATCH /api/v1/repos/o/r/issues/5` returning the issue with `"state":"closed"`; call `update_task("5", RemotePatch{ status: Some(Done), .. })`; assert returned `status==Done`.
  Construct `CodebergProvider::new(reqwest::Client::new(), server.uri(), "t", "o", "r")` (base_url is the instance root; the impl appends `/api/v1`).

- [ ] **Step 2: implement** `CodebergProvider` modeled on `GitHubProvider` (`crates/providers/src/github.rs`). Differences:
  - URL: `format!("{}/api/v1/repos/{}/{}/issues", base_url, owner, repo)`; per-issue `…/issues/{remote_id}`.
  - Auth header: `Authorization: token {token}` (Gitea style). Keep `Accept: application/json`, `User-Agent: newt-todo`.
  - `list_tasks`: query `?state=all&type=issues`; ALSO filter client-side any entry with a `pull_request` field present (defensive), like GitHub.
  - Issue JSON shape is the same fields as GitHub (`number, title, body, state, html_url, updated_at, labels[].name`) — `updated_at` is RFC3339.
  - `create_task`: POST `{base}/api/v1/repos/o/r/issues` `{title, body}` — **omit labels** (Gitea needs IDs). Add a `// labels need numeric IDs on Gitea; pushed separately later` comment.
  - `update_task`: PATCH with only `title`/`body`/`state` from the patch (status→state "closed"/"open"); **ignore `patch.labels`** (comment why).
  - Reuse the same `state=="closed" → Done` mapping. You may share a helper or duplicate the small `GhIssue`-style struct as `GiteaIssue`.

- [ ] **Step 3:** register `pub mod codeberg; pub use codeberg::CodebergProvider;` in `lib.rs`. `cargo test -p newt-todo-providers` → pass. **Step 4:** commit `feat(providers): codeberg (forgejo) issues provider`.

---

## Task 2: `ClickUpProvider`

**Files:** Create `crates/providers/src/clickup.rs`; Modify `lib.rs`.

- [ ] **Step 1 (TDD):** wiremock tests in `clickup.rs`:
  - `list_tasks`: mock `GET /api/v2/list/L1/task` returning `{"tasks":[{"id":"abc","name":"do it","description":"d","status":{"status":"open","type":"open"},"url":"https://app.clickup.com/t/abc","date_updated":"1781000000000","tags":[{"name":"home"}]}, {"id":"def","name":"done one","description":"","status":{"status":"complete","type":"closed"},"url":"https://app.clickup.com/t/def","date_updated":"1781000000000","tags":[]}]}`. Assert TWO `RemoteTask`s; the first `remote_id=="abc"`, `status==Open`, `labels==["home"]`, `html_url==Some(...)`; the second `status==Done` (type "closed"). Assert `remote_updated_at` parsed from the ms-epoch string (just assert it's the expected `OffsetDateTime` for `1781000000000` ms).
  - `update_task`: mock `PUT /api/v2/task/abc` returning a task JSON with `name:"renamed"`; call `update_task("abc", RemotePatch{ title: Some("renamed".into()), .. })`; assert returned `title=="renamed"`.
  Construct `ClickUpProvider::new(reqwest::Client::new(), server.uri(), "t", "L1")` (base_url root; impl appends `/api/v2`).

- [ ] **Step 2: implement** `ClickUpProvider { client, base_url, token, list_id }`:
  - Auth header `Authorization: {token}` (NO "Bearer"/"token" prefix — ClickUp uses the raw token). `Accept: application/json`, `User-Agent: newt-todo`.
  - `list_tasks`: GET `{base}/api/v2/list/{list_id}/task`; response wrapper `{ tasks: [CuTask] }`. `CuTask { id: String, name: String, description: Option<String>, status: CuStatus, url: String, date_updated: String, #[serde(default)] tags: Vec<CuTag> }`, `CuStatus { #[serde(rename="type")] kind: String }`, `CuTag { name: String }`. Map: `remote_id=id`, `title=name`, `body=description.unwrap_or_default()`, `status = if status.kind=="closed" { Done } else { Open }`, `labels = tags[].name`, `html_url=Some(url)`, `remote_updated_at` = parse `date_updated` as i128 ms → `OffsetDateTime::from_unix_timestamp_nanos(ms * 1_000_000)` (map errors to `ProviderError`).
  - `create_task`: POST `{base}/api/v2/list/{list_id}/task` `{name, description}` (map `RemoteDraft.title`→name, body→description).
  - `update_task`: PUT `{base}/api/v2/task/{remote_id}` with only `name` (from `patch.title`) and `description` (from `patch.body`) when present; **skip status/labels** (comment: ClickUp status is a per-list custom string we don't resolve in MVP). Return the mapped task.
  - Add a tiny helper `parse_clickup_ms(&str) -> Result<OffsetDateTime, ProviderError>`.

- [ ] **Step 3:** register in `lib.rs`. `cargo test -p newt-todo-providers` → pass. **Step 4:** commit `feat(providers): clickup tasks provider`.

---

## Task 3: wire both into `TaskService::sync`

**Files:** Modify `crates/service` (the `sync` match + deps already include providers).

- [ ] **Step 1 (TDD):** extend the service sync test(s) — add two tests modeled on the existing `sync_pulls_issues_from_github_account`:
  - Codeberg account: `config = {"owner":"o","repo":"r"}`, `base_url = wiremock uri`; token seeded; mock `GET /api/v1/repos/o/r/issues`; assert `pulled == 1`.
  - ClickUp account: `config = {"list_id":"L1"}`, `base_url = wiremock uri`; token seeded; mock `GET /api/v2/list/L1/task` returning one task; assert `pulled == 1`.

- [ ] **Step 2: implement** — in `TaskService::sync`, extend the `match account.provider`:
  - `Codeberg` → read `config["owner"]`/`["repo"]`, build `CodebergProvider::new(reqwest::Client::new(), base_url.unwrap_or_else(|| "https://codeberg.org".into()), token.access_token, owner, repo)`.
  - `Clickup` → read `config["list_id"]`, build `ClickUpProvider::new(reqwest::Client::new(), base_url.unwrap_or_else(|| "https://api.clickup.com".into()), token.access_token, list_id)`.
  - Remove the catch-all bail for these two; keep a sensible error if config keys are missing.

- [ ] **Step 3:** `cargo test --workspace` (all green), clippy clean, fmt. Commit `feat(service): wire codeberg + clickup into sync`.

---

## Task 4 (RUNTIME-UNVERIFIED — build, do not claim it works): provider-aware login + settings UI

> Same caveat as Plan 4b Task 4: unverifiable here; needs the user's real Codeberg/ClickUp OAuth apps. Build cleanly, keep parsing/logic testable, report as built-but-unverified.

- [ ] Generalize the desktop login: a `start_oauth_login(provider, client_id, client_secret, config_json)` command (or per-provider commands `start_codeberg_login`/`start_clickup_login`) that uses the right endpoints:
  - **Codeberg** (Auth-Code **+ PKCE**): authorize `https://codeberg.org/login/oauth/authorize`, token `https://codeberg.org/login/oauth/access_token`; generate a PKCE pair (`auth::generate_pkce` with a random verifier — add a random-verifier generator), pass `pkce_challenge` to the authorize URL and `pkce_verifier` to `exchange_code` (client_secret may still be required by Forgejo — send both). scope e.g. `read:issue write:issue` (or `repository`). config `{owner, repo}`.
  - **ClickUp** (Auth-Code, no PKCE): authorize `https://app.clickup.com/api`, token `https://api.clickup.com/api/v2/oauth/token`; client_secret required. config `{list_id}`. NOTE ClickUp's token endpoint expects `client_id`/`client_secret`/`code` as query params — verify against their docs; adapt `exchange_code` or add a ClickUp-specific exchange if the form-body shape differs.
- [ ] Settings UI: a provider dropdown (GitHub / Codeberg / ClickUp) that shows the right fields and calls the matching command. Reuse the existing account list/sync/delete. Add i18n keys to `messages/{en,de}.json`.
- [ ] `cargo build` + `bun run build` + `bun run check` succeed; report unverified.

---

## Done Criteria
- `CodebergProvider` + `ClickUpProvider` implement `Provider`, wiremock-tested (pull + update; PR/closed mapping; ClickUp ms-timestamp + status-type mapping).
- `TaskService::sync` builds and runs all three providers; per-provider sync tested against mocked HTTP.
- Task 4 built but explicitly unverified.

## After this
Plan 6 — sync polish: conflict-copy + surfacing, background interval, promote local→remote (create), label/status push for providers that support it.
