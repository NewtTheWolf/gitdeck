import { describe, expect, it } from "vitest";
import type {
  Board,
  BoardColumn,
  Issue,
  PullRequest,
  SmartFilter,
  TaskDto,
} from "../api";
import {
  buildPool,
  itemKeyFor,
  matchFilter,
  resolveBoard,
  type BoardItem,
} from "./resolve";

const ACCT = "acc-1";

function issue(over: Partial<Issue> = {}): Issue {
  return {
    number: 1,
    title: "An issue",
    html_url: "https://github.com/o/r/issues/1",
    state: "open",
    author_login: "alice",
    author_avatar_url: null,
    repo_name_with_owner: "o/r",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    comments_count: 0,
    labels: [],
    assignees: [],
    ...over,
  };
}

function pull(over: Partial<PullRequest> = {}): PullRequest {
  return { ...issue(over), is_draft: false, ...over } as PullRequest;
}

function todo(over: Partial<TaskDto> = {}): TaskDto {
  return {
    id: "t1",
    title: "A todo",
    body: "",
    status: "open",
    labels: [],
    due_at: null,
    updated_at: "2026-01-01T00:00:00Z",
    source_url: null,
    ...over,
  };
}

// Build a minimal BoardItem directly (for filter unit tests).
function bi(over: Partial<BoardItem> = {}): BoardItem {
  return {
    key: "k",
    kind: "issue",
    title: "title",
    repo: "o/r",
    state: "open",
    labels: [],
    author: "alice",
    assignees: [],
    updated_at: "2026-01-01T00:00:00Z",
    provider: "github",
    account_id: ACCT,
    raw: issue(),
    ...over,
  };
}

function column(over: Partial<BoardColumn> = {}): BoardColumn {
  return {
    id: "c1",
    name: "Col",
    position: 0,
    filter: {},
    created_at: "2026-01-01T00:00:00Z",
    cards: [],
    ...over,
  };
}

function board(columns: BoardColumn[]): Board {
  return {
    id: "b1",
    name: "Board",
    position: 0,
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-01T00:00:00Z",
    columns,
  };
}

// --- itemKey + buildPool ----------------------------------------------------

describe("itemKeyFor", () => {
  it("builds todo/issue/pr keys in the documented scheme", () => {
    expect(itemKeyFor("todo", { id: "42" })).toBe("todo:42");
    expect(
      itemKeyFor("issue", { accountId: ACCT, repo: "o/r", number: 7 }),
    ).toBe("gh-issue:acc-1:o/r#7");
    expect(itemKeyFor("pr", { accountId: ACCT, repo: "o/r", number: 9 })).toBe(
      "gh-pr:acc-1:o/r#9",
    );
  });
});

describe("buildPool", () => {
  const pool = buildPool({
    accountId: ACCT,
    issues: [issue({ number: 7, repo_name_with_owner: "o/r" })],
    pulls: [pull({ number: 9, repo_name_with_owner: "o/r" })],
    todos: [todo({ id: "42", labels: ["chore"], status: "done" })],
  });

  it("normalizes all three kinds", () => {
    expect(pool.map((p) => p.kind).sort()).toEqual(["issue", "pr", "todo"]);
  });

  it("uses the canonical key format for each kind", () => {
    expect(pool.find((p) => p.kind === "issue")!.key).toBe("gh-issue:acc-1:o/r#7");
    expect(pool.find((p) => p.kind === "pr")!.key).toBe("gh-pr:acc-1:o/r#9");
    expect(pool.find((p) => p.kind === "todo")!.key).toBe("todo:42");
  });

  it("maps todo done->closed, provider local, no repo, labels carried", () => {
    const t = pool.find((p) => p.kind === "todo")!;
    expect(t.state).toBe("closed");
    expect(t.provider).toBe("local");
    expect(t.repo).toBeUndefined();
    expect(t.account_id).toBeUndefined();
    expect(t.labels).toEqual(["chore"]);
  });

  it("maps issue/pr provider github with account_id and label names", () => {
    const i = buildPool({
      accountId: ACCT,
      issues: [issue({ labels: [{ name: "bug", color: "f00" }] })],
      pulls: [],
      todos: [],
    })[0];
    expect(i.provider).toBe("github");
    expect(i.account_id).toBe(ACCT);
    expect(i.labels).toEqual(["bug"]);
    expect(i.author).toBe("alice");
  });
});

// --- matchFilter ------------------------------------------------------------

describe("matchFilter", () => {
  it("empty filter matches all OPEN items (state defaults to open)", () => {
    expect(matchFilter(bi({ state: "open" }), {})).toBe(true);
    expect(matchFilter(bi({ state: "closed" }), {})).toBe(false);
  });

  it("state: closed matches only closed", () => {
    const f: SmartFilter = { state: "closed" };
    expect(matchFilter(bi({ state: "closed" }), f)).toBe(true);
    expect(matchFilter(bi({ state: "open" }), f)).toBe(false);
  });

  it("state: all disables the state constraint", () => {
    const f: SmartFilter = { state: "all" };
    expect(matchFilter(bi({ state: "open" }), f)).toBe(true);
    expect(matchFilter(bi({ state: "closed" }), f)).toBe(true);
  });

  it("item_types is any-of membership", () => {
    const f: SmartFilter = { item_types: ["pr", "todo"] };
    expect(matchFilter(bi({ kind: "pr" }), f)).toBe(true);
    expect(matchFilter(bi({ kind: "todo", state: "open" }), f)).toBe(true);
    expect(matchFilter(bi({ kind: "issue" }), f)).toBe(false);
  });

  it("empty item_types array imposes no constraint", () => {
    expect(matchFilter(bi({ kind: "issue" }), { item_types: [] })).toBe(true);
  });

  it("providers any-of; constraining to github excludes local todos", () => {
    const f: SmartFilter = { providers: ["github"] };
    expect(matchFilter(bi({ provider: "github" }), f)).toBe(true);
    expect(
      matchFilter(bi({ kind: "todo", provider: "local", state: "open" }), f),
    ).toBe(false);
  });

  it("account_ids any-of", () => {
    const f: SmartFilter = { account_ids: ["acc-1"] };
    expect(matchFilter(bi({ account_id: "acc-1" }), f)).toBe(true);
    expect(matchFilter(bi({ account_id: "acc-2" }), f)).toBe(false);
    expect(matchFilter(bi({ account_id: undefined }), f)).toBe(false);
  });

  it("repos exact any-of", () => {
    const f: SmartFilter = { repos: ["o/r"] };
    expect(matchFilter(bi({ repo: "o/r" }), f)).toBe(true);
    expect(matchFilter(bi({ repo: "o/other" }), f)).toBe(false);
    expect(matchFilter(bi({ repo: undefined }), f)).toBe(false);
  });

  it("labels any-of, case-insensitive", () => {
    const f: SmartFilter = { labels: ["Bug"] };
    expect(matchFilter(bi({ labels: ["bug"] }), f)).toBe(true);
    expect(matchFilter(bi({ labels: ["BUG", "x"] }), f)).toBe(true);
    expect(matchFilter(bi({ labels: ["feature"] }), f)).toBe(false);
    expect(matchFilter(bi({ labels: [] }), f)).toBe(false);
  });

  it("assignees any-of logins", () => {
    const f: SmartFilter = { assignees: ["bob"] };
    expect(matchFilter(bi({ assignees: ["bob"] }), f)).toBe(true);
    expect(matchFilter(bi({ assignees: ["carol"] }), f)).toBe(false);
  });

  it("text is case-insensitive substring over title+repo+author", () => {
    expect(
      matchFilter(bi({ title: "Fix the Login bug" }), { text: "login" }),
    ).toBe(true);
    expect(matchFilter(bi({ repo: "octo/webapp" }), { text: "webapp" })).toBe(
      true,
    );
    expect(matchFilter(bi({ author: "Octocat" }), { text: "octo" })).toBe(true);
    expect(matchFilter(bi({ title: "nothing" }), { text: "absent" })).toBe(
      false,
    );
  });

  it("blank text imposes no constraint", () => {
    expect(matchFilter(bi({}), { text: "   " })).toBe(true);
  });

  it("combines fields with AND", () => {
    const f: SmartFilter = { item_types: ["issue"], labels: ["bug"], state: "open" };
    expect(
      matchFilter(bi({ kind: "issue", labels: ["bug"], state: "open" }), f),
    ).toBe(true);
    // wrong kind fails despite matching labels/state
    expect(
      matchFilter(bi({ kind: "pr", labels: ["bug"], state: "open" }), f),
    ).toBe(false);
  });
});

// --- resolveBoard -----------------------------------------------------------

describe("resolveBoard", () => {
  it("smart-only: fills each column with its matches", () => {
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 1, labels: [{ name: "bug", color: "f00" }] })],
      pulls: [pull({ number: 2 })],
      todos: [todo({ id: "t1" })],
    });
    const b = board([
      column({ id: "issues", filter: { item_types: ["issue"] } }),
      column({ id: "prs", filter: { item_types: ["pr"] } }),
    ]);
    const res = resolveBoard(b, pool);
    expect(res[0].items.map((i) => i.kind)).toEqual(["issue"]);
    expect(res[1].items.map((i) => i.kind)).toEqual(["pr"]);
  });

  it("manual override pulls a non-matching item into the column", () => {
    // A closed issue would not match the default open filter, but a manual card
    // pulls it in.
    const closedKey = itemKeyFor("issue", {
      accountId: ACCT,
      repo: "o/r",
      number: 5,
    });
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 5, state: "closed" })],
      pulls: [],
      todos: [],
    });
    const b = board([
      column({
        id: "doing",
        filter: { item_types: ["issue"] }, // default open ⇒ wouldn't match closed
        cards: [{ id: "card1", item_key: closedKey, position: 0 }],
      }),
    ]);
    const res = resolveBoard(b, pool);
    expect(res[0].items.map((i) => i.key)).toEqual([closedKey]);
  });

  it("manual placement in column A removes the item from column B's smart results", () => {
    const key = itemKeyFor("issue", { accountId: ACCT, repo: "o/r", number: 1 });
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 1 })],
      pulls: [],
      todos: [],
    });
    // Both columns' filters would match the issue, but it's manually placed in A.
    const b = board([
      column({
        id: "A",
        filter: { item_types: ["issue"] },
        cards: [{ id: "c", item_key: key, position: 0 }],
      }),
      column({ id: "B", filter: { item_types: ["issue"] } }),
    ]);
    const res = resolveBoard(b, pool);
    expect(res[0].items.map((i) => i.key)).toEqual([key]);
    expect(res[1].items).toEqual([]);
  });

  it("an item appears in at most one column board-wide", () => {
    const key = itemKeyFor("issue", { accountId: ACCT, repo: "o/r", number: 1 });
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 1 })],
      pulls: [],
      todos: [],
    });
    const b = board([
      column({ id: "A", filter: { item_types: ["issue"] }, cards: [{ id: "c", item_key: key, position: 0 }] }),
      column({ id: "B", filter: { item_types: ["issue"] } }),
      column({ id: "C", filter: { item_types: ["issue"] } }),
    ]);
    const res = resolveBoard(b, pool);
    const totalAppearances = res.reduce(
      (n, c) => n + c.items.filter((i) => i.key === key).length,
      0,
    );
    expect(totalAppearances).toBe(1);
  });

  it("smart items are ordered by updated_at desc", () => {
    const pool = buildPool({
      accountId: ACCT,
      issues: [
        issue({ number: 1, updated_at: "2026-01-01T00:00:00Z" }),
        issue({ number: 2, updated_at: "2026-03-01T00:00:00Z" }),
        issue({ number: 3, updated_at: "2026-02-01T00:00:00Z" }),
      ],
      pulls: [],
      todos: [],
    });
    const b = board([column({ filter: { item_types: ["issue"] } })]);
    const res = resolveBoard(b, pool);
    expect(res[0].items.map((i) => (i.raw as Issue).number)).toEqual([2, 3, 1]);
  });

  it("manual cards come first (by position), then smart items", () => {
    const k2 = itemKeyFor("issue", { accountId: ACCT, repo: "o/r", number: 2 });
    const k3 = itemKeyFor("issue", { accountId: ACCT, repo: "o/r", number: 3 });
    const pool = buildPool({
      accountId: ACCT,
      issues: [
        issue({ number: 1, updated_at: "2026-05-01T00:00:00Z" }), // smart, newest
        issue({ number: 2, updated_at: "2026-01-01T00:00:00Z" }), // manual
        issue({ number: 3, updated_at: "2026-02-01T00:00:00Z" }), // manual
      ],
      pulls: [],
      todos: [],
    });
    const b = board([
      column({
        filter: { item_types: ["issue"] },
        // position puts #3 before #2
        cards: [
          { id: "ca", item_key: k2, position: 1 },
          { id: "cb", item_key: k3, position: 0 },
        ],
      }),
    ]);
    const res = resolveBoard(b, pool);
    expect(res[0].items.map((i) => (i.raw as Issue).number)).toEqual([3, 2, 1]);
  });

  it("drops a manual card whose item is not resolvable from the pool", () => {
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 1 })],
      pulls: [],
      todos: [],
    });
    const b = board([
      column({
        // manual-only filter (matches nothing on its own) isolates the drop
        filter: { item_types: ["pr"] },
        cards: [{ id: "c", item_key: "gh-issue:acc-1:o/r#999", position: 0 }],
      }),
    ]);
    const res = resolveBoard(b, pool);
    // unresolvable manual card dropped, no PRs to fill smart ⇒ empty
    expect(res[0].items).toEqual([]);
  });

  it("de-dups within a column when a manual card also matches the filter", () => {
    const key = itemKeyFor("issue", { accountId: ACCT, repo: "o/r", number: 1 });
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 1 })],
      pulls: [],
      todos: [],
    });
    const b = board([
      column({
        filter: { item_types: ["issue"] }, // would also match #1
        cards: [{ id: "c", item_key: key, position: 0 }],
      }),
    ]);
    const res = resolveBoard(b, pool);
    expect(res[0].items.map((i) => i.key)).toEqual([key]); // exactly once
  });

  it("empty-filter ({}) column matches all OPEN pool items (manual first)", () => {
    // Per the task spec, an empty filter is NOT manual-only: it matches all open
    // items (state defaults to "open"). The manual card still leads, then smart.
    const todoKey = itemKeyFor("todo", { id: "t1" });
    const issueKey = itemKeyFor("issue", {
      accountId: ACCT,
      repo: "o/r",
      number: 1,
    });
    const pool = buildPool({
      accountId: ACCT,
      issues: [issue({ number: 1, updated_at: "2026-02-01T00:00:00Z" })],
      pulls: [],
      todos: [todo({ id: "t1", updated_at: "2026-01-01T00:00:00Z" })],
    });
    const b = board([
      column({
        id: "all",
        filter: {},
        cards: [{ id: "c", item_key: todoKey, position: 0 }],
      }),
    ]);
    const res = resolveBoard(b, pool);
    // todo is manual (leads), issue is the remaining open smart match.
    expect(res[0].items.map((i) => i.key)).toEqual([todoKey, issueKey]);
  });
});
