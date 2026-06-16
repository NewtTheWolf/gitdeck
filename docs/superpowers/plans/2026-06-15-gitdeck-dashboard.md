# Phase 2 — gitdeck-structured dashboard (our style, our Rust backend)

> REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Reference clone of gitdeck's real frontend at `/tmp/gitdeck-src` (branch feat/multi-account-provider, MIT). We port gitdeck's STRUCTURE + behavior + data shapes + filter/sort logic, but render with OUR Tailwind tokens + Lucide (the minimal-sharp design system in `2026-06-15-ui-design-system.md`), wired to OUR Tauri commands + Rust providers. NO gitdeck Node backend, NO sidecar (must run on Android).

**Goal:** Turn the app into a gitdeck-style multi-account dashboard: a sidebar that switches between **Repos / Issues / Pull Requests / Todos**, each a dense, filterable list/grid, in our minimal style, fed by our Rust backend. Keep todos + MCP.

## Data contract (match gitdeck's view needs)
- **RemoteRepo** (exists): full_name, description, stars, open_issues, language, is_private, is_fork, html_url, updated_at. (Add `pushed_at`, `is_archived`, `owner_avatar_url` if cheap.)
- **RemoteIssue** (new): `{ number, title, html_url, state, author_login, author_avatar_url, repo_name_with_owner, created_at, updated_at, comments_count, labels: [{name, color}], assignees: [login] }`.
- **RemotePullRequest** (new): RemoteIssue fields + `{ is_draft }`. (gitdeck also has additions/deletions/reviewDecision via GraphQL — DEFER; the view degrades gracefully without them.)

## GitHub data source (REST Search API — one call each, rich enough)
- Issues: `GET {base}/search/issues?q=is:issue+involves:{login}&sort=updated&per_page=50`
- PRs: `GET {base}/search/issues?q=is:pr+involves:{login}&sort=updated&per_page=50`
- Search result item → our DTO: `number`, `title`, `html_url`, `state`, `user.login`/`user.avatar_url`, `comments`, `labels[].{name,color}`, `assignees[].login`, `created_at`, `updated_at`, `repository_url` → derive `repo_name_with_owner` (last two path segments), `draft` (PRs). The login comes from the account (fetched at connect; store it in account config as `login`).

---

## Task 1: Backend — RemoteIssue/RemotePullRequest + provider + service + commands
- [ ] `core::provider`: add `RemoteIssue`, `RemotePullRequest` (serde Serialize), and default trait methods `async fn list_issues(&self) -> Result<Vec<RemoteIssue>, ProviderError> { Ok(vec![]) }` and `list_pull_requests(...) { Ok(vec![]) }`. Re-export.
- [ ] `providers/github.rs`: override both via the Search API (the GitHubProvider holds the token; it needs the `login` — add a `login: String` field to `GitHubProvider::new(client, base_url, token, owner, repo, login)` OR derive the query with `involves:@me` which GitHub resolves to the token's user — PREFER `involves:@me` so no login is needed). Parse the search response `{ items: [GhSearchItem] }`; map per the contract; `draft` field for PRs; derive `repo_name_with_owner` from `repository_url`. wiremock tests for both (issue mapping; PR mapping incl. is_draft; repo_name_with_owner derivation).
- [ ] `service`: `list_issues(token_store, account_id)` + `list_pull_requests(...)` mirroring `list_repos`. wiremock test (account → search endpoint → mapped result).
- [ ] Tauri commands `list_issues(id)` + `list_pull_requests(id)` in `apps/desktop/src-tauri/src/lib.rs`, registered.
- [ ] Gates: `cargo test --workspace` (grows), clippy clean, fmt, `cargo build -p newt-todo-desktop`.

## Task 2: Frontend — dashboard sections (Repos/Issues/PRs) ported + restyled
Reference `/tmp/gitdeck-src/src/components/views/{RepoGrid,IssueList,PullRequestList}.tsx` and `utils/dashboard.ts` (filters/sort — plain TS, port the logic) and `utils/format.ts` (formatRelativeTime, formatNumber).
- [ ] `api.ts`: add `Issue`/`PullRequest` TS types + `listIssues(id)`/`listPullRequests(id)` wrappers.
- [ ] Restructure the sidebar nav into gitdeck-style sections: **Repos / Issues / Pull Requests / Todos / Settings** (Lucide icons). The active account (from AccountContext) drives all dashboard sections.
- [ ] Dashboard routes/sections: `pages/dashboard/{Repos,Issues,PullRequests}.tsx` using ported views — `RepoGrid` (have it), `IssueList` (dense rows: avatar, author, repo, #num, relative time, title, labels with their colors, comments count, assignees), `PullRequestList` (same + draft badge). Render with OUR tokens (panel/border/text-muted, hairlines, Lucide `CircleDot`/`GitPullRequest`/`Star`), tabular-nums, hover row. Labels use the GitHub label color as a small dot/chip (semantic color only as accents, per our design rules).
- [ ] Port filter/sort: a shared filter bar (state open/closed, repo, label, text search, sort by updated/created) using gitdeck's `utils/dashboard.ts` logic adapted to our types. Keep it as plain testable TS (`src/lib/dashboard.ts` + vitest).
- [ ] Loading skeletons + empty states (our style) for each section.
- [ ] i18n keys for all new strings (en + de).
- [ ] Gates: `bun run build`, `bun run test`, `cargo build -p newt-todo-desktop`. User verifies render.

## Out of scope (defer): CIHealth, Insights, DailyDigest (AI), Inbox/notifications, Kanban, Triage, command palette, repo-detail modals. Add later if wanted.

## "Do it right"
- MIT attribution for gitdeck in `apps/desktop/NOTICE` (already present) — extend to mention ported view structure + filter logic.
- Keep ported logic modular/clean. The eventual "PR to gitdeck" (if pursued) would be a separate effort (e.g. a Tauri/mobile build or our theme contributed upstream) — not blocked by this.
