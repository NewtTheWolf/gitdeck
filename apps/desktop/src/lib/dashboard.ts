import type { Issue, IssueState, PullRequest } from "./api";

/**
 * Filter / sort logic for the Issues and Pull Requests sections.
 *
 * Ported from gitdeck's `src/utils/dashboard.ts` (MIT, debba/gitdeck) and
 * trimmed to the subset this app ships: state / repo / label / free-text
 * filtering and updated/created sorting. Kept as plain, generic, testable TS —
 * it operates over the fields shared by `Issue` and `PullRequest`, so the same
 * functions drive both sections.
 */

export type StateFilter = "all" | "open" | "closed";
export type SortKey = "updated" | "created";

/** The fields the filter/sort helpers actually read — shared by Issue and PR. */
export type DashboardItem = Pick<
  Issue,
  | "title"
  | "state"
  | "author_login"
  | "repo_name_with_owner"
  | "labels"
  | "assignees"
  | "created_at"
  | "updated_at"
>;

export interface DashboardFilters {
  state: StateFilter;
  /** A single `repo_name_with_owner`, or "" for all repos. */
  repo: string;
  /** A single label name, or "" for all labels. */
  label: string;
  /** Free-text query — matched against title / repo / author / labels. */
  search: string;
  sort: SortKey;
}

export const DEFAULT_FILTERS: DashboardFilters = {
  state: "all",
  repo: "",
  label: "",
  search: "",
  sort: "updated",
};

function matchesState(item: DashboardItem, state: StateFilter): boolean {
  if (state === "all") return true;
  return item.state === (state as IssueState);
}

/**
 * Filter items by state, repo, label and a free-text query. The query is
 * matched (case-insensitively) against the title, the `owner/repo` path, the
 * author login and the label names.
 */
export function filterItems<T extends DashboardItem>(
  items: T[],
  filters: DashboardFilters,
): T[] {
  const query = filters.search.trim().toLowerCase();
  return items.filter((item) => {
    if (!matchesState(item, filters.state)) return false;
    if (filters.repo && item.repo_name_with_owner !== filters.repo) return false;
    if (
      filters.label &&
      !(item.labels || []).some((label) => label.name === filters.label)
    ) {
      return false;
    }
    if (!query) return true;
    const haystack = [
      item.title,
      item.repo_name_with_owner,
      item.author_login || "",
      ...(item.labels || []).map((label) => label.name),
      ...(item.assignees || []),
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(query);
  });
}

/** Sort by updated/created, newest first. Returns a new array. */
export function sortItems<T extends DashboardItem>(
  items: T[],
  sort: SortKey,
): T[] {
  const key = sort === "created" ? "created_at" : "updated_at";
  return [...items].sort(
    (a, b) => Date.parse(b[key] || "") - Date.parse(a[key] || ""),
  );
}

/** Filter then sort, in one pass. */
export function filterAndSortItems<T extends DashboardItem>(
  items: T[],
  filters: DashboardFilters,
): T[] {
  return sortItems(filterItems(items, filters), filters.sort);
}

/** Distinct `owner/repo` paths present in the data, sorted alphabetically. */
export function distinctRepos(items: DashboardItem[]): string[] {
  const set = new Set<string>();
  for (const item of items) set.add(item.repo_name_with_owner);
  return [...set].sort((a, b) => a.localeCompare(b));
}

/** Distinct label names present in the data, sorted alphabetically. */
export function distinctLabels(items: DashboardItem[]): string[] {
  const set = new Set<string>();
  for (const item of items) {
    for (const label of item.labels || []) set.add(label.name);
  }
  return [...set].sort((a, b) => a.localeCompare(b));
}

// PR-only convenience type alias so callers can be explicit.
export type PullRequestItem = PullRequest;
