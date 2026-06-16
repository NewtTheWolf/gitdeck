# Roadmap: gitdeck FULL — in our style, on our backend

Date: 2026-06-16 · Branch: `master`

> User goal: **"gitdeck Full in unseren [style] stark zu implementieren"** — port gitdeck's
> ENTIRE feature set, reworked in our minimal-sharp design, on our **mobile-capable Rust/Tauri
> backend (NO sidecar — gitdeck's Node backend is NOT used)**. Boards are our own superset of
> gitdeck's Kanban.

## STATUS 2026-06-16: FULL PARITY COMPLETE (runtime-unverified)
All views below built, committed on `master`, tested (≈93 Rust + 167 frontend, clippy clean):
Repos/Issues/PRs grids · Boards · Inbox · CI-Health · ⌘K palette · Repository-Detail (7 tabs:
Overview, Releases, Forks, Contributors, Actions, Traffic, Mentions) · Insights · Daily-Digest ·
Triage (write actions). Deferred: Dependents (no clean API), Pagination, Welcome/Changelog modals,
OpenAI digest narrative. NEXT: provider track (Forgejo/Codeberg + ClickUp). Needs a `tauri dev` walkthrough.

## Principle
For every gitdeck view we (1) build the equivalent data in our Rust `providers`/`service` + a
Tauri command (replacing the gitdeck `/api/*` endpoint), then (2) port the React view in OUR
design system. GitHub data comes from REST/GraphQL via our reqwest `GitHubProvider` (scope `repo`,
which covers traffic/notifications). Forgejo/Codeberg = parallel provider track.

## gitdeck `/api/*` → our backend mapping
| gitdeck endpoint | our backend (GitHub API) | view | status |
|---|---|---|---|
| /api/accounts(+activate/add-token), /api/auth/* | accounts CRUD + OAuth loopback | AccountSwitcher/AuthGate/Settings | ✅ done |
| /api/repos | `list_repos` (`/user/repos`) | RepoGrid | ✅ done |
| /api/issues | `list_issues` (search `involves:@me`) | IssueList | ✅ done (grid) |
| /api/prs | `list_pull_requests` | PullRequestList | ✅ done (grid) |
| /api/project(s)/move | (GitHub Projects v2) | KanbanView | ⏳ **superseded by our Boards** |
| /api/ci-health | `/repos/{o}/{r}/actions/runs` | CIHealthView | ⬜ Phase C |
| /api/notifications(+read/read-all) | `/notifications` (+ PATCH read) | InboxView | ⬜ Phase C |
| /api/repo-details | `/repos/{o}/{r}` + sub-resources | RepositoryDetailsModal | ⬜ Phase D |
| /api/forks | `/repos/{o}/{r}/forks` | repo modal · Forks | ⬜ Phase D |
| /api/stargazers | `/repos/{o}/{r}/stargazers` | repo modal | ⬜ Phase D |
| /api/mentions/code | `/search/code` | repo modal · Mentions | ⬜ Phase D |
| /api/mentions/issues | `/search/issues` | repo modal · Mentions | ⬜ Phase D |
| /api/mentions/referrers | `/repos/{o}/{r}/traffic/popular/referrers` | repo modal · Traffic | ⬜ Phase D |
| /api/mentions/dependents | dependents (GraphQL/scrape) | repo modal · Dependents | ⬜ Phase D (best-effort) |
| /api/repo-insights | computed: repos + traffic (+ snapshots) | InsightsView | ⬜ Phase E |
| /api/daily-digests | snapshots over time (+ optional OpenAI) | DailyDigestView | ⬜ Phase F |
| /api/repo-aliases | local store | alias/rename tracking | ⬜ Phase D-adjacent |
| /api/provider-configs | provider registry | multi-provider | ⬜ provider track |
| (triage actions: assign/label/close/comment) | issue/PR mutation commands | TriageWorkspace | ⬜ Phase G |

## Phases (after current Boards work)
**Boards (IN PROGRESS)** — B1 resolve engine + api (running), B2 board UI. = our Kanban, smart-views-first.

**Phase C — Inbox + CI Health + Command Palette.** Highest value, each is a single GitHub REST
endpoint (no history). Provider methods `list_notifications`/`mark_notification_read` (`/notifications`)
and `list_workflow_runs` (`/repos/{o}/{r}/actions/runs`, aggregated across repos) + Tauri commands.
Views: InboxView (notifications list, mark read/read-all), CIHealthView (per-repo latest run status:
success/failure/in-progress, recent runs). CommandPalette (⌘K, pure FE — jump to section/repo/issue,
run actions). Our style.

**Phase D — Repository Details modal (the per-repo deep-dive).** A modal/route with tabs:
Overview (`/repos/{o}/{r}`), Actions (workflow runs), PRs, Issues, Releases (`/releases`),
Forks (`/forks`), Contributors (`/contributors`), Languages (`/languages`), Traffic
(`/traffic/views`,`/clones`,`/popular/referrers`,`/popular/paths`), Mentions (search code/issues),
Dependents (best-effort). Each tab = a provider method + command; build tab-by-tab. RepositoryMetricModal
+ ContributorsModal fold in here. Repo aliases (track previous names for Mentions) = local store.

**Phase E — Insights.** Per-repo health: alerts ("issues need attention", "no push for N days"),
opportunities, traffic/activity correlation, status Strong/Watch/Risky. Mostly computable from the
repo list + traffic; richer trends use Phase F snapshots. Backend: `repo_insights` computation in
service; view InsightsView.

**Phase F — Snapshots + Daily Digest + trend charts.** New `snapshots` table capturing daily repo
metrics (stars/forks/open issues) on app open; `daily_digest` computation (per-repo day movement +
executive summary, copyable as Markdown); stars/forks history + language trend charts (lightweight SVG,
our style, no heavy chart dep). OpenAI narrative = optional, behind a user-supplied key (feature-flagged,
off by default).

**Phase G — Triage workspace.** Cross-repo triage with WRITE actions: assign, (un)label, close/reopen,
comment. New mutation provider methods + commands (first writes in the app). TriageWorkspace view.
Ties into Boards (triage → place on a board).

**Provider track (parallel/after) — Forgejo/Codeberg + ClickUp.** gitdeck supports Forgejo; our
`crates/providers` gets a Forgejo/Codeberg provider (Plan 5) so every view above is multi-provider.
ClickUp for todos/boards pool.

## Cross-cutting (our-style, applied as we go)
- Pagination (gitdeck has `Pagination.tsx`) for long lists.
- Filter sidebars consistent across Repos/Issues/PRs (org, language, visibility, forks/archived, sort).
- i18n: gitdeck ships en/de/es/fr/it/zh — we keep en/de (add more later); every new view adds keys to both.
- Caching: gitdeck caches API responses on disk; we add a lightweight cache layer (service or Store)
  to avoid rate limits, since each view hits GitHub.
- Empty/skeleton/error states + focus/a11y per our design system spec.

## Explicitly deferred / flagged
- OpenAI digest narrative (needs user API key) — off by default.
- Dependents (GitHub has no clean API) — best-effort.
- GitHub Projects v2 import — our Boards replace it; possible import later.
