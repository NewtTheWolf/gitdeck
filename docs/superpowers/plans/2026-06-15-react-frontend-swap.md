# Phase 1 — Frontend swap: SvelteKit → React + Vite

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. This swaps the Tauri app's frontend framework. The Rust backend, Tauri commands, and Tailwind v4 stay; SvelteKit/Paraglide are replaced by React 19 + Vite + react-i18next (to reuse gitdeck's React code in Phase 2).

**Goal:** Replace the SvelteKit frontend of `apps/desktop` with a React 19 + Vite SPA that ports the current Todos + Settings features 1:1, calling the SAME Tauri commands. App must build (`bun run build`), test (`bun run test`), and the Tauri binary must build (`cargo build -p newt-todo-desktop`). Visual render is user-verified.

**Keep:** all Rust crates, all Tauri commands (`list_tasks/get_task/create_task/update_task/complete_task/delete_task`, `list_accounts/create_account/delete_account/sync_account`, `start_github_login`), Tailwind v4, `src/lib/api.ts` + `src/lib/filter.ts` (plain TS — port verbatim), the dmabuf fix in `main.rs`.

**Remove:** `@sveltejs/*`, `svelte`, `@inlang/paraglide-js`, `svelte.config.js`, `src/routes/*.svelte`, `src/routes/+layout.ts`, `src/app.html`, `static/`, `messages/`, `project.inlang/`, generated `src/lib/paraglide/`.

---

## Target structure
```
apps/desktop/
├─ index.html                 # Vite entry: <div id="root"></div> + module script to /src/main.tsx
├─ package.json               # react stack
├─ tsconfig.json              # react jsx
├─ vite.config.ts             # react() + tailwindcss(); server port 1420
├─ tauri.conf.json            # frontendDist "../dist"; beforeDevCommand "bun run dev"; beforeBuildCommand "bun run build"
└─ src/
   ├─ main.tsx                # ReactDOM root, i18n init, <HashRouter>
   ├─ App.tsx                 # routes: "/" → Tasks, "/settings" → Settings
   ├─ app.css                 # @import "tailwindcss";
   ├─ lib/{api.ts,filter.ts,api.test.ts,filter.test.ts}   # ported (api/filter unchanged)
   ├─ lib/i18n.ts             # i18next + react-i18next init (en default, de), resources from locales
   ├─ locales/{en.json,de.json}
   └─ pages/{Tasks.tsx,Settings.tsx}
```

---

## Task 1: deps + config (foundation)

- [ ] **Step 1: package.json** — replace the svelte/paraglide devDeps. Final `package.json`:
```json
{
  "name": "newt-todo",
  "version": "0.1.0",
  "description": "Newt Todo desktop app",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "tauri": "tauri"
  },
  "license": "MIT",
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-opener": "^2",
    "react": "^19",
    "react-dom": "^19",
    "react-router-dom": "^7",
    "i18next": "^25",
    "react-i18next": "^15"
  },
  "devDependencies": {
    "@tauri-apps/cli": "^2",
    "@vitejs/plugin-react": "^5",
    "@tailwindcss/vite": "^4",
    "tailwindcss": "^4",
    "@testing-library/react": "^16",
    "@testing-library/jest-dom": "^6",
    "jsdom": "^29",
    "typescript": "~5.6",
    "vite": "^6",
    "vitest": "^4",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "@types/node": "^22"
  }
}
```
Then `rm -rf node_modules bun.lock && bun install`. (Resolve actual latest within these majors.)

- [ ] **Step 2: vite.config.ts**
```ts
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a node global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
}));
```

- [ ] **Step 3: tsconfig.json** (React, bundler resolution):
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "types": ["node", "vitest/globals", "@testing-library/jest-dom"]
  },
  "include": ["src"]
}
```

- [ ] **Step 4: index.html** at `apps/desktop/index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Newt Todo</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 5: tauri.conf.json** — change `build.frontendDist` to `"../dist"` (Vite default out dir). Keep `beforeDevCommand: "bun run dev"`, `devUrl: "http://localhost:1420"`, `beforeBuildCommand: "bun run build"`. Leave productName/identifier/window as-is.

- [ ] **Step 6: delete SvelteKit artifacts** — `git rm -r` (or rm) `svelte.config.js`, `src/routes/`, `src/app.html`, `src/app.css` (will be recreated), `static/`, `messages/`, `project.inlang/`, and the generated `src/lib/paraglide/` (gitignored). Update `.gitignore`: remove `/.svelte-kit`, `/build`, paraglide lines; add `/dist`.

- [ ] **Step 7:** commit `chore(app): swap build tooling to react + vite (foundation)`. (Won't fully build until Task 2-3 add source — that's fine.)

---

## Task 2: i18n + app shell + Tasks page

- [ ] **Step 1: app.css** at `src/app.css`: `@import "tailwindcss";`

- [ ] **Step 2: locales** — port the message keys. `src/locales/en.json` and `src/locales/de.json` with the SAME keys/values that were in the old `messages/{en,de}.json` (app_title, new_task_placeholder, add_task, filter_all/open/done, empty_state, complete_task, reopen_task, delete_task, tasks_remaining — for `tasks_remaining` use i18next interpolation `"{{count}} open"`).

- [ ] **Step 3: i18n.ts**
```ts
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../locales/en.json";
import de from "../locales/de.json";

i18n.use(initReactI18next).init({
  resources: { en: { translation: en }, de: { translation: de } },
  lng: localStorage.getItem("locale") ?? "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});
export default i18n;
```

- [ ] **Step 4: main.tsx**
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import { HashRouter } from "react-router-dom";
import "./app.css";
import "./lib/i18n";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <HashRouter>
      <App />
    </HashRouter>
  </React.StrictMode>,
);
```

- [ ] **Step 5: App.tsx** — a minimal shell with a header link to Settings, routes:
```tsx
import { Routes, Route } from "react-router-dom";
import Tasks from "./pages/Tasks";
import Settings from "./pages/Settings";

export default function App() {
  return (
    <div className="min-h-screen bg-neutral-950 text-neutral-100">
      <Routes>
        <Route path="/" element={<Tasks />} />
        <Route path="/settings" element={<Settings />} />
      </Routes>
    </div>
  );
}
```

- [ ] **Step 6: port `src/lib/api.ts` and `src/lib/filter.ts`** verbatim from the old files (same content — they're framework-agnostic TS). Port `api.test.ts` and `filter.test.ts` too (same assertions; vitest config below).

- [ ] **Step 7: vitest** — add a `vitest.config.ts` (jsdom env so React component tests could run later; api/filter tests run fine):
```ts
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";
export default defineConfig({
  plugins: [react()],
  test: { environment: "jsdom", globals: true, include: ["src/**/*.test.{ts,tsx}"] },
});
```

- [ ] **Step 8: `src/pages/Tasks.tsx`** — React port of the old `+page.svelte` todo list: `useState` for tasks/filter/newTitle/error, `useEffect` to load on mount, add/toggle/remove/setFilter handlers calling `api`, `useMemo` for visible/openCount via `filter.ts`, locale toggle via `i18n.changeLanguage` + `localStorage`, a `<Link to="/settings">` in the header. Same Tailwind dark styling and behavior as the Svelte version. Use `useTranslation()` (`const { t, i18n } = useTranslation()`), e.g. `t("app_title")`, `t("tasks_remaining", { count: openCount })`.

- [ ] **Step 9:** `bun run build` (tsc + vite) succeeds; `bun run test` passes (api + filter). Commit `feat(app): react i18n, shell, and tasks page`.

---

## Task 3: Settings page + final verify

- [ ] **Step 1: `src/pages/Settings.tsx`** — React port of the old settings page: connect-GitHub form (client_id/secret/owner/repo → `api.startGithubLogin`), account list (`api.listAccounts` on mount) each with Sync (`api.syncAccount`, show `{pulled,pushed}`) + Delete (`api.deleteAccount`), inline errors, a `<Link to="/">` back. (Ensure `api.ts` has `startGithubLogin/listAccounts/deleteAccount/syncAccount` — they were in the old api.ts; port them.) Add the settings i18n keys to `locales/{en,de}.json` (settings_title, gh_client_id, gh_client_secret, gh_owner, gh_repo, connect_github, sync_now, synced_report, no_accounts, delete_account — same as the old messages).

- [ ] **Step 2: verify everything**
  - `bun run build` → dist/ produced.
  - `bun run test` → all green (api wrappers + filter; add any React component test only if trivial).
  - From repo root: `cargo build -p newt-todo-desktop` → compiles (frontendDist `../dist` exists).
  - `cargo test --workspace` → still 43 (Rust untouched).

- [ ] **Step 3:** commit `feat(app): react settings page`.

---

## Done Criteria
- Tauri app frontend is React 19 + Vite (no Svelte/Paraglide remaining).
- Todos + Settings ported 1:1, calling the same Tauri commands; i18n en/de via react-i18next.
- `bun run build`, `bun run test`, `cargo build -p newt-todo-desktop` all pass. Render is user-verified.
- Foundation ready for Phase 2 (pull in gitdeck's React dashboard components).

## Next (Phase 2)
Bring gitdeck's React dashboard (shell + account switcher + repos/issues/PRs views, MIT-attributed), rewire its data layer to Tauri commands; extend providers/service/commands with dashboard data (`list_repos`, `list_pull_requests`). Then Plan 5 (Codeberg/ClickUp providers) folds in.
