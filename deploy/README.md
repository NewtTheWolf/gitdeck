# Hosting `gitdeck-server`

`gitdeck-server` (crate `newt-todo-api`) exposes the full `TaskService` over **gRPC** and
**gRPC-Web** (so a browser/Tauri webview can call it directly — no Envoy). Run it embedded
(the desktop app does this implicitly via Tauri) or **standalone** to back one or more clients.

## Run with Docker (recommended)

```bash
# from the repo root
export GITHUB_TOKEN=ghp_xxx          # token used for GitHub-backed calls
# export GITDECK_API_KEY=some-secret # optional: require a bearer key from clients
docker compose -f deploy/docker-compose.yml up -d --build
```

The server listens on `:50061`. Point the desktop app at it: **Settings → Server →** mode
`Remote`, URL `http://<host>:50061` (and the API key if you set one).

## Run from source

```bash
GITHUB_TOKEN=ghp_xxx cargo run --release -p newt-todo-api --bin gitdeck-server
```

## Configuration (env)

| Variable          | Default              | Purpose                                                        |
| ----------------- | -------------------- | -------------------------------------------------------------- |
| `GITDECK_BIND`    | `127.0.0.1:50061`    | Address/port to bind. Use `0.0.0.0:50061` in a container.      |
| `GITDECK_DB`      | `./gitdeck.db`       | SQLite path **or** a `postgres://…` DSN (SeaORM picks the backend). |
| `GITHUB_TOKEN`    | —                    | GitHub token for all GitHub-backed RPCs (single-token for now).|
| `GITDECK_API_KEY` | — (off)              | If set, clients must send `Authorization: Bearer <key>`.       |

## Notes & roadmap

- **Auth model (now):** one `GITHUB_TOKEN` is used for every GitHub call. Per-account server
  tokens come later. Local todos/boards live in the server's `GITDECK_DB`.
- **Transport:** gRPC for native clients, gRPC-Web (with permissive CORS) for the desktop webview.
- **Postgres (SeaORM, done):** point `GITDECK_DB` at a `postgres://…` DSN and start the bundled
  service: `docker compose -f deploy/docker-compose.yml --profile postgres up -d` (also uncomment the
  postgres `GITDECK_DB` line + `depends_on` in the compose file). SQLite stays the zero-config default.
- **Phase K (sync):** the server becomes the source of truth for boards/todos across devices,
  with server-streaming for live updates.
- Put a TLS-terminating reverse proxy (Caddy/nginx/Traefik) in front for anything beyond localhost;
  gRPC-Web rides on HTTP/1.1 so it proxies cleanly.
