# Plan 3b — Svelte UI (Tailwind + Paraglide) + Vitest

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development. Checkboxes track steps.

**Goal:** A working task-list UI consuming the Tauri commands: add, complete/reopen, delete, filter — i18n (en/de) via Paraglide, styled with Tailwind v4, with Vitest tests on the logic layer.

**Architecture:** Logic lives in plain TS for testability — `src/lib/api.ts` (typed wrappers over Tauri `invoke`) and `src/lib/filter.ts` (pure helpers). `src/routes/+page.svelte` is a thin Svelte 5 (runes) view holding `$state`, calling `api`, using `filter` helpers and Paraglide `m` messages. Tests mock `@tauri-apps/api/core` and assert the logic.

**Tech Stack:** SvelteKit (adapter-static SPA), Svelte 5 runes, Tailwind v4, Paraglide JS 2.x (already set up: `src/lib/paraglide` generated, messages in `messages/{en,de}.json`), Vitest 4.

**Already done (Plan 3b setup):** Tailwind v4 + Paraglide Vite plugins wired in `vite.config.js`; `src/app.css` (`@import "tailwindcss";`) imported by `src/routes/+layout.svelte`; Vitest + @testing-library installed. Frontend builds clean.

---

## Task 1: API bridge + pure filter helpers (+ tests)

**Files:** Create `src/lib/api.ts`, `src/lib/filter.ts`, `src/lib/api.test.ts`, `src/lib/filter.test.ts`, `vitest.config.ts`; Modify `package.json` (add `test` script).

- [ ] **Step 1: `src/lib/api.ts`**
```ts
import { invoke } from "@tauri-apps/api/core";

export type TaskStatus = "open" | "done";

export interface TaskDto {
  id: string;
  title: string;
  body: string;
  status: TaskStatus;
  labels: string[];
  due_at: string | null;
  updated_at: string;
  source_url: string | null;
}

export interface ListTasksParams {
  status?: string | null;
  label?: string | null;
  query?: string | null;
}
export interface CreateTaskParams {
  title: string;
  body?: string | null;
  labels?: string[] | null;
  due_at?: string | null;
}
export interface UpdateTaskParams {
  id: string;
  title?: string | null;
  body?: string | null;
  status?: string | null;
  labels?: string[] | null;
  due_at?: string | null;
  clear_due?: boolean;
}

export const api = {
  listTasks: (params: ListTasksParams = {}) =>
    invoke<TaskDto[]>("list_tasks", { params }),
  createTask: (params: CreateTaskParams) =>
    invoke<TaskDto>("create_task", { params }),
  updateTask: (params: UpdateTaskParams) =>
    invoke<TaskDto>("update_task", { params }),
  completeTask: (id: string) => invoke<TaskDto>("complete_task", { id }),
  deleteTask: (id: string) => invoke<void>("delete_task", { id }),
};
```

- [ ] **Step 2: `src/lib/filter.ts`**
```ts
import type { TaskDto } from "./api";

export type Filter = "all" | "open" | "done";

export function filterTasks(tasks: TaskDto[], filter: Filter): TaskDto[] {
  if (filter === "all") return tasks;
  return tasks.filter((t) => t.status === filter);
}

export function countOpen(tasks: TaskDto[]): number {
  return tasks.filter((t) => t.status === "open").length;
}
```

- [ ] **Step 3: `vitest.config.ts`** (plain node env; logic tests need no Svelte compilation)
```ts
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
```
Add to `package.json` scripts: `"test": "vitest run"`.

- [ ] **Step 4: `src/lib/filter.test.ts`**
```ts
import { describe, expect, it } from "vitest";
import { countOpen, filterTasks, type Filter } from "./filter";
import type { TaskDto } from "./api";

const t = (id: string, status: "open" | "done"): TaskDto => ({
  id, title: id, body: "", status, labels: [], due_at: null,
  updated_at: "2026-06-15T00:00:00Z", source_url: null,
});

describe("filterTasks", () => {
  const tasks = [t("a", "open"), t("b", "done"), t("c", "open")];
  it("returns all when filter=all", () => {
    expect(filterTasks(tasks, "all" as Filter)).toHaveLength(3);
  });
  it("filters open", () => {
    expect(filterTasks(tasks, "open").map((x) => x.id)).toEqual(["a", "c"]);
  });
  it("filters done", () => {
    expect(filterTasks(tasks, "done").map((x) => x.id)).toEqual(["b"]);
  });
});

describe("countOpen", () => {
  it("counts open tasks", () => {
    expect(countOpen([t("a", "open"), t("b", "done"), t("c", "open")])).toBe(2);
  });
});
```

- [ ] **Step 5: `src/lib/api.test.ts`** (mock the Tauri invoke)
```ts
import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { api } from "./api";

describe("api", () => {
  beforeEach(() => invoke.mockReset().mockResolvedValue(undefined));

  it("listTasks passes params", async () => {
    await api.listTasks({ status: "open" });
    expect(invoke).toHaveBeenCalledWith("list_tasks", { params: { status: "open" } });
  });
  it("createTask passes params", async () => {
    await api.createTask({ title: "x" });
    expect(invoke).toHaveBeenCalledWith("create_task", { params: { title: "x" } });
  });
  it("completeTask passes id", async () => {
    await api.completeTask("id-1");
    expect(invoke).toHaveBeenCalledWith("complete_task", { id: "id-1" });
  });
  it("deleteTask passes id", async () => {
    await api.deleteTask("id-2");
    expect(invoke).toHaveBeenCalledWith("delete_task", { id: "id-2" });
  });
});
```

- [ ] **Step 6: Run tests**
Run: `bun run test`
Expected: all pass (filter + api suites). Adapt the mock form if vitest 4 needs it (the goal: assert invoke is called with the right command + args). Commit:
```bash
git add apps/desktop/src/lib apps/desktop/vitest.config.ts apps/desktop/package.json apps/desktop/bun.lock
git commit -m "feat(app): typed tauri api bridge + filter helpers with vitest tests"
```

---

## Task 2: The task-list view (`+page.svelte`)

**Files:** Replace `src/routes/+page.svelte`.

- [ ] **Step 1: Replace the page**
Replace `apps/desktop/src/routes/+page.svelte` with a Svelte 5 (runes) view. It must: load tasks on mount, add a task from the input, toggle complete/reopen, delete, switch filter, show the open count and an empty state — all strings via Paraglide `m`. Use Tailwind for a clean, focused layout (not a generic centered card — a real app shell: header with title + locale toggle, an add bar, filter tabs, and a task list). Handle invoke errors by surfacing a small inline error.

```svelte
<script lang="ts">
  import { onMount } from "svelte";
  import { api, type TaskDto } from "$lib/api";
  import { countOpen, filterTasks, type Filter } from "$lib/filter";
  import { m } from "$lib/paraglide/messages.js";
  import { getLocale, setLocale } from "$lib/paraglide/runtime.js";

  let tasks = $state<TaskDto[]>([]);
  let filter = $state<Filter>("all");
  let newTitle = $state("");
  let error = $state<string | null>(null);

  const visible = $derived(filterTasks(tasks, filter));
  const openCount = $derived(countOpen(tasks));

  async function load() {
    try { tasks = await api.listTasks(); error = null; }
    catch (e) { error = String(e); }
  }
  onMount(load);

  async function add() {
    const title = newTitle.trim();
    if (!title) return;
    try { await api.createTask({ title }); newTitle = ""; await load(); }
    catch (e) { error = String(e); }
  }

  async function toggle(task: TaskDto) {
    try {
      if (task.status === "open") await api.completeTask(task.id);
      else await api.updateTask({ id: task.id, status: "open" });
      await load();
    } catch (e) { error = String(e); }
  }

  async function remove(task: TaskDto) {
    try { await api.deleteTask(task.id); await load(); }
    catch (e) { error = String(e); }
  }

  function toggleLocale() {
    setLocale(getLocale() === "en" ? "de" : "en");
  }

  const filters: Filter[] = ["all", "open", "done"];
  function filterLabel(f: Filter) {
    return f === "all" ? m.filter_all() : f === "open" ? m.filter_open() : m.filter_done();
  }
</script>

<div class="min-h-screen bg-neutral-950 text-neutral-100">
  <div class="mx-auto max-w-2xl px-5 py-8">
    <header class="mb-6 flex items-center justify-between">
      <h1 class="text-2xl font-semibold tracking-tight">{m.app_title()}</h1>
      <button
        class="rounded-md border border-neutral-700 px-2.5 py-1 text-xs uppercase tracking-wide text-neutral-300 hover:bg-neutral-800"
        onclick={toggleLocale}>{getLocale()}</button>
    </header>

    <form class="mb-4 flex gap-2" onsubmit={(e) => { e.preventDefault(); add(); }}>
      <input
        class="flex-1 rounded-lg border border-neutral-700 bg-neutral-900 px-3 py-2 text-sm outline-none placeholder:text-neutral-500 focus:border-emerald-500"
        placeholder={m.new_task_placeholder()}
        bind:value={newTitle} />
      <button
        class="rounded-lg bg-emerald-600 px-4 py-2 text-sm font-medium hover:bg-emerald-500 disabled:opacity-40"
        type="submit" disabled={!newTitle.trim()}>{m.add_task()}</button>
    </form>

    <div class="mb-3 flex items-center justify-between">
      <div class="flex gap-1">
        {#each filters as f (f)}
          <button
            class="rounded-md px-3 py-1 text-sm {filter === f ? 'bg-neutral-800 text-neutral-100' : 'text-neutral-400 hover:text-neutral-200'}"
            onclick={() => (filter = f)}>{filterLabel(f)}</button>
        {/each}
      </div>
      <span class="text-xs text-neutral-500">{m.tasks_remaining({ count: openCount })}</span>
    </div>

    {#if error}
      <p class="mb-3 rounded-md border border-red-900 bg-red-950/50 px-3 py-2 text-sm text-red-300">{error}</p>
    {/if}

    {#if visible.length === 0}
      <p class="py-12 text-center text-sm text-neutral-500">{m.empty_state()}</p>
    {:else}
      <ul class="space-y-1.5">
        {#each visible as task (task.id)}
          <li class="group flex items-center gap-3 rounded-lg border border-neutral-800 bg-neutral-900 px-3 py-2.5">
            <input type="checkbox" class="size-4 accent-emerald-500"
              checked={task.status === "done"} onchange={() => toggle(task)} />
            <span class="flex-1 text-sm {task.status === 'done' ? 'text-neutral-500 line-through' : ''}">{task.title}</span>
            <button
              class="rounded px-2 py-1 text-xs text-neutral-500 opacity-0 transition hover:text-red-400 group-hover:opacity-100"
              onclick={() => remove(task)}>{m.delete_task()}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</div>
```

- [ ] **Step 2: Type-check + build**
Run: `bun run check` (svelte-check) — fix any type errors.
Run: `bun run build` — frontend builds (Paraglide regenerates, Tailwind compiles).

- [ ] **Step 3: Rust still builds**
Run (from repo root): `cargo build -p newt-todo-desktop` — still compiles (frontendDist embeds the new build).

- [ ] **Step 4: Commit**
```bash
git add apps/desktop/src/routes/+page.svelte
git commit -m "feat(app): task-list UI with i18n, filters, and Tailwind"
```

---

## Done Criteria
- `bun run test` passes (api + filter suites).
- `bun run build` and `cargo build -p newt-todo-desktop` succeed.
- The page renders a working todo list wired to the five Tauri commands, with en/de toggle.

## Verification (coordinator)
After the tasks, run the app (`cargo tauri dev` from `apps/desktop`, or a built binary) and confirm: add a task → appears; check it → strikes through + count drops; filter Open/Done works; delete removes it; locale toggle switches en/de.

## Next
Merge Plan 3 (3a+3b) to master. Then Plan 4: GitHub provider (Device Flow OAuth + sync).
