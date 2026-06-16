import { describe, expect, it } from "vitest";
import type { Issue, PullRequest } from "./api";
import {
  DEFAULT_FILTERS,
  distinctLabels,
  distinctRepos,
  filterAndSortItems,
  filterItems,
  sortItems,
} from "./dashboard";

function issue(over: Partial<Issue>): Issue {
  return {
    number: 1,
    title: "Fix the thing",
    html_url: "https://example.com/1",
    state: "open",
    author_login: "octocat",
    author_avatar_url: null,
    repo_name_with_owner: "acme/web",
    created_at: "2026-01-01T00:00:00Z",
    updated_at: "2026-01-02T00:00:00Z",
    comments_count: 0,
    labels: [],
    assignees: [],
    ...over,
  };
}

const items: Issue[] = [
  issue({
    number: 1,
    title: "Login bug",
    state: "open",
    repo_name_with_owner: "acme/web",
    author_login: "alice",
    labels: [{ name: "bug", color: "d73a4a" }],
    assignees: ["bob"],
    created_at: "2026-02-01T00:00:00Z",
    updated_at: "2026-03-01T00:00:00Z",
  }),
  issue({
    number: 2,
    title: "Add docs",
    state: "closed",
    repo_name_with_owner: "acme/docs",
    author_login: "carol",
    labels: [{ name: "docs", color: "0075ca" }],
    created_at: "2026-01-15T00:00:00Z",
    updated_at: "2026-04-01T00:00:00Z",
  }),
  issue({
    number: 3,
    title: "Refactor parser",
    state: "open",
    repo_name_with_owner: "acme/web",
    author_login: "alice",
    labels: [{ name: "bug", color: "d73a4a" }, { name: "perf", color: "00ff00" }],
    created_at: "2026-03-10T00:00:00Z",
    updated_at: "2026-03-15T00:00:00Z",
  }),
];

describe("filterItems — state", () => {
  it("returns everything for 'all'", () => {
    expect(filterItems(items, { ...DEFAULT_FILTERS, state: "all" })).toHaveLength(3);
  });
  it("keeps only open", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, state: "open" });
    expect(r.map((i) => i.number)).toEqual([1, 3]);
  });
  it("keeps only closed", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, state: "closed" });
    expect(r.map((i) => i.number)).toEqual([2]);
  });
});

describe("filterItems — repo & label", () => {
  it("filters by repo", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, repo: "acme/docs" });
    expect(r.map((i) => i.number)).toEqual([2]);
  });
  it("filters by label", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, label: "bug" });
    expect(r.map((i) => i.number)).toEqual([1, 3]);
  });
  it("combines repo + state", () => {
    const r = filterItems(items, {
      ...DEFAULT_FILTERS,
      repo: "acme/web",
      state: "open",
    });
    expect(r.map((i) => i.number)).toEqual([1, 3]);
  });
});

describe("filterItems — free-text search", () => {
  it("matches the title", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, search: "login" });
    expect(r.map((i) => i.number)).toEqual([1]);
  });
  it("matches the repo path", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, search: "docs" });
    // "docs" hits both acme/docs and the 'docs' label, plus 'Add docs' title
    expect(r.map((i) => i.number)).toEqual([2]);
  });
  it("matches the author", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, search: "carol" });
    expect(r.map((i) => i.number)).toEqual([2]);
  });
  it("matches a label name", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, search: "perf" });
    expect(r.map((i) => i.number)).toEqual([3]);
  });
  it("is case-insensitive and trims", () => {
    const r = filterItems(items, { ...DEFAULT_FILTERS, search: "  LOGIN " });
    expect(r.map((i) => i.number)).toEqual([1]);
  });
  it("returns empty for no match", () => {
    expect(filterItems(items, { ...DEFAULT_FILTERS, search: "zzz" })).toHaveLength(0);
  });
});

describe("sortItems", () => {
  it("sorts by updated, newest first", () => {
    const r = sortItems(items, "updated");
    expect(r.map((i) => i.number)).toEqual([2, 3, 1]);
  });
  it("sorts by created, newest first", () => {
    const r = sortItems(items, "created");
    expect(r.map((i) => i.number)).toEqual([3, 1, 2]);
  });
  it("does not mutate the input", () => {
    const before = items.map((i) => i.number);
    sortItems(items, "created");
    expect(items.map((i) => i.number)).toEqual(before);
  });
});

describe("filterAndSortItems", () => {
  it("filters then sorts", () => {
    const r = filterAndSortItems(items, {
      ...DEFAULT_FILTERS,
      state: "open",
      sort: "created",
    });
    expect(r.map((i) => i.number)).toEqual([3, 1]);
  });
});

describe("distinct facets", () => {
  it("lists distinct repos sorted", () => {
    expect(distinctRepos(items)).toEqual(["acme/docs", "acme/web"]);
  });
  it("lists distinct labels sorted", () => {
    expect(distinctLabels(items)).toEqual(["bug", "docs", "perf"]);
  });
});

describe("PullRequest compatibility", () => {
  it("filters PRs through the same generic helpers", () => {
    const prs: PullRequest[] = [
      { ...issue({ number: 10, title: "Draft PR" }), is_draft: true },
      { ...issue({ number: 11, title: "Ready PR", state: "closed" }), is_draft: false },
    ];
    const open = filterItems(prs, { ...DEFAULT_FILTERS, state: "open" });
    expect(open.map((p) => p.number)).toEqual([10]);
  });
});
