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

export type ProviderKind = "github" | "codeberg" | "clickup";

export interface Account {
  id: string;
  provider: ProviderKind;
  display_name: string;
  base_url: string | null;
  config: unknown;
  created_at: string;
}

export interface SyncReport {
  pulled: number;
  pushed: number;
}

export interface Repo {
  id: string;
  full_name: string;
  description: string | null;
  html_url: string;
  stars: number;
  open_issues: number;
  language: string | null;
  is_private: boolean;
  is_fork: boolean;
  updated_at: string;
}

export interface Label {
  name: string;
  color: string;
}

export type IssueState = "open" | "closed";

export interface Issue {
  number: number;
  title: string;
  html_url: string;
  state: IssueState;
  author_login: string;
  author_avatar_url: string | null;
  repo_name_with_owner: string;
  created_at: string;
  updated_at: string;
  comments_count: number;
  labels: Label[];
  assignees: string[];
}

export interface PullRequest extends Issue {
  is_draft: boolean;
}

// --- notifications / inbox --------------------------------------------------

export interface Notification {
  id: string;
  repo_name_with_owner: string;
  subject_title: string;
  subject_type: string;
  subject_url: string | null;
  reason: string;
  unread: boolean;
  updated_at: string;
}

export interface MarkNotificationReadParams {
  id: string;
  thread_id: string;
}

// --- workflow runs / CI -----------------------------------------------------

export interface WorkflowRun {
  id: number;
  name: string | null;
  head_branch: string | null;
  status: string;
  conclusion: string | null;
  html_url: string;
  created_at: string;
  updated_at: string;
  event: string | null;
}

export interface ListWorkflowRunsParams {
  id: string;
  owner: string;
  repo: string;
  per_page?: number | null;
}

// --- repository details -----------------------------------------------------

/** Shared params for the repository-detail endpoints (account id + repo coords). */
export interface RepoRefParams {
  id: string;
  owner: string;
  repo: string;
  per_page?: number | null;
}

export interface RepoDetail {
  full_name: string;
  description: string | null;
  html_url: string;
  homepage: string | null;
  language: string | null;
  stars: number;
  forks: number;
  open_issues: number;
  watchers: number;
  default_branch: string;
  license: string | null;
  topics: string[];
  owner_login: string;
  owner_avatar_url: string | null;
  is_private: boolean;
  is_fork: boolean;
  is_archived: boolean;
  size: number;
  pushed_at: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface Release {
  id: number;
  tag_name: string;
  name: string | null;
  html_url: string;
  body: string | null;
  draft: boolean;
  prerelease: boolean;
  published_at: string | null;
  author_login: string | null;
}

export interface Fork {
  full_name: string;
  html_url: string;
  stars: number;
  pushed_at: string | null;
  owner_login: string;
}

export interface Contributor {
  login: string;
  avatar_url: string | null;
  html_url: string;
  contributions: number;
}

export interface Language {
  name: string;
  bytes: number;
}

// --- traffic ----------------------------------------------------------------

export interface TrafficDay {
  timestamp: string;
  count: number;
  uniques: number;
}

export interface Traffic {
  count: number;
  uniques: number;
  days: TrafficDay[];
}

export interface Referrer {
  referrer: string;
  count: number;
  uniques: number;
}

export interface RepoPath {
  path: string;
  title: string;
  count: number;
  uniques: number;
}

// --- search (Mentions tab) --------------------------------------------------

export interface CodeHit {
  repo_name_with_owner: string;
  path: string;
  html_url: string;
  name: string;
}

/** Params for the search endpoints (`search_code` / `search_issues`). */
export interface SearchParams {
  id: string;
  query: string;
  per_page?: number | null;
}

// --- boards -----------------------------------------------------------------

/**
 * A column's live smart filter. Stored opaquely by the backend (TEXT/JSON);
 * interpreted entirely in TS by the resolve engine (see lib/boards/resolve.ts).
 *
 * Every provided field is an AND constraint; an omitted/empty field is no
 * constraint. Membership fields (providers/account_ids/repos/labels/assignees/
 * item_types) are any-of. `{}` (all fields empty) ⇒ a manual-only column that
 * matches nothing on its own.
 */
export interface SmartFilter {
  /** Which kinds to include. Default (omitted/empty): all three. */
  item_types?: ("issue" | "pr" | "todo")[];
  /** Default (omitted): "open". "all" disables the state constraint. */
  state?: "open" | "closed" | "all";
  providers?: ("github" | "codeberg" | "clickup")[];
  account_ids?: string[];
  /** owner/name, exact match. */
  repos?: string[];
  /** any-of, case-insensitive. */
  labels?: string[];
  /** any-of logins. */
  assignees?: string[];
  /** free text over title/repo/author, case-insensitive substring. */
  text?: string;
}

export interface BoardCard {
  id: string;
  item_key: string;
  position: number;
}

export interface BoardColumn {
  id: string;
  name: string;
  position: number;
  filter: SmartFilter;
  created_at: string;
  cards: BoardCard[];
}

export interface Board {
  id: string;
  name: string;
  position: number;
  created_at: string;
  updated_at: string;
  /** Empty for `listBoards`; populated by `getBoard`. */
  columns: BoardColumn[];
}

// --- triage write actions ---------------------------------------------------

/** Shared issue coordinates for the triage write commands. */
export interface IssueRefParams {
  id: string;
  owner: string;
  repo: string;
  number: number;
}
export interface SetIssueStateParams extends IssueRefParams {
  state: IssueState;
}
export interface IssueLabelsParams extends IssueRefParams {
  labels: string[];
}
export interface RemoveLabelParams extends IssueRefParams {
  label: string;
}
export interface IssueAssigneesParams extends IssueRefParams {
  assignees: string[];
}
export interface IssueCommentParams extends IssueRefParams {
  body: string;
}

// --- in-app issue / PR detail -----------------------------------------------

/** A GitHub user reference (login + optional avatar). */
export interface User {
  login: string;
  avatar_url: string | null;
}

/** A single comment on an issue or PR (issue-comment timeline). */
export interface Comment {
  id: number;
  author_login: string;
  author_avatar_url: string | null;
  body: string;
  created_at: string;
  updated_at: string;
  html_url: string;
}

export type ReviewState =
  | "APPROVED"
  | "CHANGES_REQUESTED"
  | "COMMENTED"
  | "DISMISSED"
  | "PENDING";

/** A single PR review with its overall state. */
export interface Review {
  id: number;
  reviewer_login: string;
  reviewer_avatar_url: string | null;
  state: ReviewState;
  body: string | null;
  submitted_at: string | null;
  html_url: string;
}

/** Full detail for a single issue (in-app issue detail view). */
export interface IssueDetail {
  number: number;
  title: string;
  body: string | null;
  html_url: string;
  state: string;
  author_login: string;
  author_avatar_url: string | null;
  repo_name_with_owner: string;
  created_at: string;
  updated_at: string;
  closed_at: string | null;
  comments_count: number;
  labels: Label[];
  assignees: User[];
  milestone_title: string | null;
}

/** Full detail for a single pull request (in-app PR detail view). */
export interface PullRequestDetail extends IssueDetail {
  is_draft: boolean;
  merged: boolean;
  mergeable_state: string | null;
  base_ref: string;
  head_ref: string;
  additions: number | null;
  deletions: number | null;
  changed_files: number | null;
  commits: number | null;
  requested_reviewers: User[];
  requested_teams: string[];
}

// --- daily repo snapshots / digest ------------------------------------------

/** One persisted daily metric capture for a repository. */
export interface Snapshot {
  id: string;
  account_id: string;
  repo_full_name: string;
  /** Calendar day in `YYYY-MM-DD`. */
  day: string;
  stars: number;
  forks: number;
  open_issues: number;
  /** RFC3339 timestamp of when this snapshot was written. */
  captured_at: string;
}

/** A per-repo entry in a daily digest: current absolute numbers plus the delta
 * versus the previous captured day. Deltas are 0 when there is no previous day. */
export interface DigestEntry {
  repo_full_name: string;
  stars_delta: number;
  forks_delta: number;
  open_issues_delta: number;
  stars: number;
  forks: number;
  open_issues: number;
}

/** A daily digest: the latest captured `day`, the previous captured `day` (if
 * any), and one entry per repo present on the latest day. */
export interface Digest {
  /** Latest captured day, or null when the account has no snapshots at all. */
  day: string | null;
  previous_day: string | null;
  entries: DigestEntry[];
}

/** Params for `list_snapshots`. */
export interface ListSnapshotsParams {
  id: string;
  /** Optional inclusive lower bound (`YYYY-MM-DD`). */
  since_day?: string | null;
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

  listAccounts: () => invoke<Account[]>("list_accounts"),
  listRepos: (id: string) => invoke<Repo[]>("list_repos", { id }),
  listIssues: (id: string) => invoke<Issue[]>("list_issues", { id }),
  listPullRequests: (id: string) =>
    invoke<PullRequest[]>("list_pull_requests", { id }),
  deleteAccount: (id: string) => invoke<void>("delete_account", { id }),
  syncAccount: (id: string) => invoke<SyncReport>("sync_account", { id }),

  // --- daily snapshots / digest ---------------------------------------------
  // Bare-id commands take `{ id }`; `list_snapshots` carries a serde struct so
  // its fields stay snake_case under the `params` wrapper.
  captureSnapshots: (id: string) =>
    invoke<number>("capture_snapshots", { id }),
  dailyDigest: (id: string) => invoke<Digest>("daily_digest", { id }),
  listSnapshots: (params: ListSnapshotsParams) =>
    invoke<Snapshot[]>("list_snapshots", { params }),

  // --- triage write actions -------------------------------------------------
  // Each carries a serde params struct, so its fields stay snake_case under the
  // top-level `params` wrapper. These perform REAL writes to GitHub.
  setIssueState: (params: SetIssueStateParams) =>
    invoke<void>("set_issue_state", { params }),
  addIssueLabels: (params: IssueLabelsParams) =>
    invoke<void>("add_issue_labels", { params }),
  removeIssueLabel: (params: RemoveLabelParams) =>
    invoke<void>("remove_issue_label", { params }),
  addIssueAssignees: (params: IssueAssigneesParams) =>
    invoke<void>("add_issue_assignees", { params }),
  createIssueComment: (params: IssueCommentParams) =>
    invoke<string>("create_issue_comment", { params }),

  // --- in-app issue / PR detail ---------------------------------------------
  // Each carries an IssueRefParams serde struct under the `params` wrapper, so
  // its fields stay snake_case. The detail reads can return null (not found).
  getIssue: (params: IssueRefParams) =>
    invoke<IssueDetail | null>("get_issue", { params }),
  getPullRequest: (params: IssueRefParams) =>
    invoke<PullRequestDetail | null>("get_pull_request", { params }),
  listIssueComments: (params: IssueRefParams) =>
    invoke<Comment[]>("list_issue_comments", { params }),
  listPullReviews: (params: IssueRefParams) =>
    invoke<Review[]>("list_pull_reviews", { params }),

  // --- inbox / notifications ------------------------------------------------
  // Bare-id command; the `params`-wrapped command keeps its serde field names.
  listNotifications: (id: string) =>
    invoke<Notification[]>("list_notifications", { id }),
  markNotificationRead: (id: string, threadId: string) =>
    invoke<void>("mark_notification_read", {
      params: { id, thread_id: threadId },
    }),

  // --- CI / workflow runs ---------------------------------------------------
  listWorkflowRuns: (params: ListWorkflowRunsParams) =>
    invoke<WorkflowRun[]>("list_workflow_runs", { params }),

  // --- repository details ---------------------------------------------------
  // All take an account id + repo coords; `per_page` is ignored by the detail
  // and languages endpoints.
  getRepoDetail: (params: RepoRefParams) =>
    invoke<RepoDetail>("get_repo_detail", { params }),
  listReleases: (params: RepoRefParams) =>
    invoke<Release[]>("list_releases", { params }),
  listForks: (params: RepoRefParams) =>
    invoke<Fork[]>("list_forks", { params }),
  listContributors: (params: RepoRefParams) =>
    invoke<Contributor[]>("list_contributors", { params }),
  getLanguages: (params: RepoRefParams) =>
    invoke<Language[]>("get_languages", { params }),

  // --- traffic (push-access required; the views/clones endpoints return null
  // when the account lacks push access to the repository) --------------------
  getTrafficViews: (params: RepoRefParams) =>
    invoke<Traffic | null>("get_traffic_views", { params }),
  getTrafficClones: (params: RepoRefParams) =>
    invoke<Traffic | null>("get_traffic_clones", { params }),
  listReferrers: (params: RepoRefParams) =>
    invoke<Referrer[]>("list_referrers", { params }),
  listPaths: (params: RepoRefParams) =>
    invoke<RepoPath[]>("list_paths", { params }),

  // --- search (Mentions tab) ------------------------------------------------
  searchCode: (params: SearchParams) =>
    invoke<CodeHit[]>("search_code", { params }),
  searchIssues: (params: SearchParams) =>
    invoke<Issue[]>("search_issues", { params }),
  // Tauri auto-converts command arg names to camelCase, so the Rust params
  // `client_id`/`client_secret` are addressed as `clientId`/`clientSecret`.
  startGithubLogin: (clientId: string, clientSecret: string) =>
    invoke<Account>("start_github_login", { clientId, clientSecret }),

  // --- boards ---------------------------------------------------------------
  // The `params`-wrapped commands carry a serde struct, so its fields stay
  // snake_case (only the top-level command arg names get camelCased by Tauri).
  // Bare-id commands take `{ id }`.
  listBoards: () => invoke<Board[]>("list_boards"),
  getBoard: (id: string) => invoke<Board | null>("get_board", { id }),
  createBoard: (name: string) =>
    invoke<Board>("create_board", { params: { name } }),
  renameBoard: (id: string, name: string) =>
    invoke<Board>("rename_board", { params: { id, name } }),
  deleteBoard: (id: string) => invoke<void>("delete_board", { id }),
  reorderBoards: (orderedIds: string[]) =>
    invoke<void>("reorder_boards", { params: { ordered_ids: orderedIds } }),

  createColumn: (boardId: string, name: string, filter?: SmartFilter) =>
    invoke<BoardColumn>("create_column", {
      params: { board_id: boardId, name, filter },
    }),
  updateColumn: (id: string, name: string, filter?: SmartFilter) =>
    invoke<BoardColumn>("update_column", { params: { id, name, filter } }),
  deleteColumn: (id: string) => invoke<void>("delete_column", { id }),
  reorderColumns: (boardId: string, orderedIds: string[]) =>
    invoke<void>("reorder_columns", {
      params: { board_id: boardId, ordered_ids: orderedIds },
    }),

  placeCard: (boardId: string, columnId: string, itemKey: string) =>
    invoke<BoardCard>("place_card", {
      params: { board_id: boardId, column_id: columnId, item_key: itemKey },
    }),
  removeCard: (boardId: string, itemKey: string) =>
    invoke<void>("remove_card", {
      params: { board_id: boardId, item_key: itemKey },
    }),
};
