# Newt Todo — a Tauri/Rust rewrite of gitdeck

A personal, cross-platform (Linux + Android focus) project-management hub: a unified
todo list plus a full git dashboard, with an MCP server so an agent can manage your todos.

This branch (`refactor/newt-todo-tauri`) **replaces the original gitdeck Node/React app with a
ground-up rewrite** that keeps gitdeck's feature set but runs on a **Tauri 2 + Rust** core
(no Node sidecar — so it works on mobile/Android) with a **React 19** frontend reworked in a
minimal-sharp design system.

> Attribution: the feature set and UX are inspired by **gitdeck** by debba
> (https://github.com/debba/gitdeck), MIT-licensed (see `LICENSE`). All GitHub/Forgejo data
> access was reimplemented in Rust; gitdeck's Node backend is not used.

## Architecture

Cargo workspace + a Tauri desktop app:

- `crates/core` — pure-Rust domain + SQLite store (dependency-light: no reqwest/tauri).
- `crates/providers` — GitHub provider (reqwest, wiremock-tested).
- `crates/service` — shared `TaskService` + DTOs over core/providers.
- `crates/auth` — OAuth (Authorization-Code + loopback) + token storage (keyring).
- `crates/mcp` — stdio MCP server (rmcp) exposing the todo tools.
- `apps/desktop` — Tauri 2 shell (`src-tauri`, Rust commands) + React 19 + Vite frontend
  (react-router, Tailwind v4, react-i18next en/de, TanStack Query caching).

## Features (gitdeck parity, our style)

Repos · Issues · Pull Requests (card grids) · **Boards** (custom columns with live smart-filters
+ manual override) · **Triage** (cross-repo issue actions: close/reopen, labels, assign, comment) ·
**Inbox** (notifications) · **CI Health** (workflow runs) · **Insights** (repo health) ·
**Daily Digest** (snapshot diff + copy-as-markdown) · **⌘K** command palette ·
**Repository detail** (Overview, Releases, Forks, Contributors, Actions, Traffic, Mentions) ·
**In-app Issue/PR detail** with markdown, comments, and PR review status (requested reviewers,
per-reviewer decisions). Account-wide via the GitHub Search API; stale-while-revalidate caching.

## Develop

```bash
# Rust workspace
cargo test --workspace
cargo build -p newt-todo-desktop

# Frontend (bun)
cd apps/desktop && bun install && bun run test && bun run build

# Run the app
cd apps/desktop && bun run tauri dev
```

## License

MIT — see `LICENSE`. Inspired by and attributed to gitdeck (debba).
