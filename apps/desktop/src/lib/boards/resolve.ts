import type {
  Board,
  BoardColumn,
  Issue,
  PullRequest,
  SmartFilter,
  TaskDto,
} from "../api";

/**
 * The smart-filter resolve engine.
 *
 * Boards are smart-views-first: each column carries a live `SmartFilter` over a
 * unified item pool (GitHub issues + PRs + local todos). Manual card placements
 * (`board_cards`) overlay on top — board-wide, an item appears in at most one
 * column, and a manual placement always wins over the smart result.
 *
 * Everything here is pure TS so it stays cheap, reactive, offline-friendly and
 * vitest-testable.
 */

export type BoardItemKind = "issue" | "pr" | "todo";

/**
 * A normalized item in the cross-source pool. `key` is the canonical identity
 * used by manual placement (`board_cards.item_key`) — keep it stable.
 */
export interface BoardItem {
  /** Canonical identity; see `itemKey` / `itemKeyFor`. */
  key: string;
  kind: BoardItemKind;
  title: string;
  /** owner/name for GitHub items; undefined for local todos. */
  repo?: string;
  state: "open" | "closed";
  labels: string[];
  author?: string;
  assignees: string[];
  updated_at: string;
  html_url?: string;
  provider: "github" | "codeberg" | "clickup" | "local";
  /** Owning account id for remote items; undefined for local todos. */
  account_id?: string;
  raw: Issue | PullRequest | TaskDto;
}

// --- item keys --------------------------------------------------------------
//
// Key scheme (MUST match the backend `item_key` used by place_card, and the UI):
//   todo:<id>
//   gh-issue:<account_id>:<repo_name_with_owner>#<number>
//   gh-pr:<account_id>:<repo_name_with_owner>#<number>

/** Build a key from raw parts. Centralized so the UI reuses the same scheme. */
export function itemKeyFor(
  kind: BoardItemKind,
  args: { id?: string; accountId?: string; repo?: string; number?: number },
): string {
  switch (kind) {
    case "todo":
      return `todo:${args.id}`;
    case "issue":
      return `gh-issue:${args.accountId}:${args.repo}#${args.number}`;
    case "pr":
      return `gh-pr:${args.accountId}:${args.repo}#${args.number}`;
  }
}

/** The canonical key for an already-built BoardItem. */
export function itemKey(item: BoardItem): string {
  return item.key;
}

// --- pool building ----------------------------------------------------------

function issueToItem(
  issue: Issue,
  accountId: string,
  kind: "issue" | "pr",
): BoardItem {
  return {
    key: itemKeyFor(kind, {
      accountId,
      repo: issue.repo_name_with_owner,
      number: issue.number,
    }),
    kind,
    title: issue.title,
    repo: issue.repo_name_with_owner,
    state: issue.state,
    labels: issue.labels.map((l) => l.name),
    author: issue.author_login,
    assignees: issue.assignees,
    updated_at: issue.updated_at,
    html_url: issue.html_url,
    provider: "github",
    account_id: accountId,
    raw: issue,
  };
}

function todoToItem(todo: TaskDto): BoardItem {
  return {
    key: itemKeyFor("todo", { id: todo.id }),
    kind: "todo",
    title: todo.title,
    repo: undefined,
    state: todo.status === "done" ? "closed" : "open",
    labels: todo.labels,
    author: undefined,
    assignees: [],
    updated_at: todo.updated_at,
    html_url: todo.source_url ?? undefined,
    provider: "local",
    account_id: undefined,
    raw: todo,
  };
}

/**
 * Normalize the three sources into a single `BoardItem[]` pool.
 *
 * Issues and PRs belong to `accountId`. Todos are local (no account, no repo).
 */
export function buildPool(opts: {
  accountId: string;
  issues: Issue[];
  pulls: PullRequest[];
  todos: TaskDto[];
}): BoardItem[] {
  return [
    ...opts.issues.map((i) => issueToItem(i, opts.accountId, "issue")),
    ...opts.pulls.map((p) => issueToItem(p, opts.accountId, "pr")),
    ...opts.todos.map(todoToItem),
  ];
}

// --- matching ---------------------------------------------------------------

function nonEmpty<T>(arr: T[] | undefined): arr is T[] {
  return Array.isArray(arr) && arr.length > 0;
}

function anyOf(values: string[], needles: string[]): boolean {
  return needles.some((n) => values.includes(n));
}

function anyOfCI(values: string[], needles: string[]): boolean {
  const lowered = values.map((v) => v.toLowerCase());
  return needles.some((n) => lowered.includes(n.toLowerCase()));
}

/**
 * Apply a smart filter to a single item. Each provided field is an AND
 * constraint; omitted/empty fields impose no constraint. Membership fields are
 * any-of.
 *
 * State: default (omitted) is "open" — only open items match. "closed" matches
 * only closed; "all" disables the state constraint.
 */
export function matchFilter(item: BoardItem, filter: SmartFilter): boolean {
  // item_types (any-of). Empty/omitted ⇒ all kinds.
  if (nonEmpty(filter.item_types) && !filter.item_types.includes(item.kind)) {
    return false;
  }

  // state — default open.
  const state = filter.state ?? "open";
  if (state !== "all" && item.state !== state) {
    return false;
  }

  // providers (any-of).
  if (nonEmpty(filter.providers)) {
    // local items have provider "local" which is never in SmartFilter.providers,
    // so constraining providers excludes todos — that's intentional.
    if (!(filter.providers as string[]).includes(item.provider)) return false;
  }

  // account_ids (any-of).
  if (nonEmpty(filter.account_ids)) {
    if (!item.account_id || !filter.account_ids.includes(item.account_id)) {
      return false;
    }
  }

  // repos (any-of, exact owner/name).
  if (nonEmpty(filter.repos)) {
    if (!item.repo || !filter.repos.includes(item.repo)) return false;
  }

  // labels (any-of, case-insensitive).
  if (nonEmpty(filter.labels)) {
    if (!anyOfCI(item.labels, filter.labels)) return false;
  }

  // assignees (any-of logins).
  if (nonEmpty(filter.assignees)) {
    if (!anyOf(item.assignees, filter.assignees)) return false;
  }

  // text — case-insensitive substring over title + repo + author.
  if (filter.text && filter.text.trim() !== "") {
    const needle = filter.text.toLowerCase();
    const hay = [item.title, item.repo ?? "", item.author ?? ""]
      .join(" ")
      .toLowerCase();
    if (!hay.includes(needle)) return false;
  }

  return true;
}

// --- board resolution -------------------------------------------------------

export interface ResolvedColumn {
  column: BoardColumn;
  items: BoardItem[];
}

/**
 * Resolve a full board (columns + manual cards) against a pool.
 *
 * Semantics:
 * - For each column, start with the smart matches (pool items where
 *   `matchFilter` is true).
 * - A manual card (`board_cards.item_key`) placed in THIS column pulls its item
 *   in even if the filter doesn't match.
 * - A manual card placed in ANOTHER column removes that item from this column's
 *   smart results — manual placement wins board-wide, so an item appears in at
 *   most one column.
 * - Manual-placed items that can't be resolved from the pool (e.g. now closed
 *   and filtered out, or no longer fetched) are DROPPED. (We have no standalone
 *   snapshot of a card to reconstruct a BoardItem from.)
 * - De-duped by key within a column.
 *
 * Ordering within a column: manual cards first (by `card.position` asc), then
 * smart items (by `updated_at` desc). A manually placed item is rendered in the
 * manual block, not the smart block, even if it also matches the filter.
 */
export function resolveBoard(board: Board, pool: BoardItem[]): ResolvedColumn[] {
  const byKey = new Map<string, BoardItem>();
  for (const item of pool) byKey.set(item.key, item);

  // Board-wide set of every manually placed key (used to subtract from smart
  // results of OTHER columns).
  const manualKeys = new Set<string>();
  for (const col of board.columns) {
    for (const card of col.cards) manualKeys.add(card.item_key);
  }

  return board.columns.map((column) => {
    const seen = new Set<string>();

    // 1. Manual cards for this column, ordered by position.
    const manual = [...column.cards]
      .sort((a, b) => a.position - b.position)
      .map((card) => byKey.get(card.item_key))
      .filter((it): it is BoardItem => it !== undefined)
      .filter((it) => {
        if (seen.has(it.key)) return false;
        seen.add(it.key);
        return true;
      });

    // 2. Smart matches, minus anything manually placed anywhere (incl. here).
    const smart = pool
      .filter((it) => matchFilter(it, column.filter))
      .filter((it) => !manualKeys.has(it.key))
      .filter((it) => {
        if (seen.has(it.key)) return false;
        seen.add(it.key);
        return true;
      })
      .sort((a, b) => (a.updated_at < b.updated_at ? 1 : a.updated_at > b.updated_at ? -1 : 0));

    return { column, items: [...manual, ...smart] };
  });
}
