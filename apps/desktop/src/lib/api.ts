import { invoke } from "@tauri-apps/api/core";
import { getGrpcClient, isRemote } from "./transport";

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

export interface CreateAccountParams {
  provider: ProviderKind;
  display_name: string;
  base_url?: string | null;
  config?: unknown;
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

// ===========================================================================
// gRPC-Web normalization
// ===========================================================================
//
// connect-es / protobuf-es v2 returns camelCase message objects (plus internal
// `$typeName`/`$unknown` keys). The rest of the app consumes the snake_case
// shapes declared above (e.g. `repo_name_with_owner`). `protoToSnake` deep-
// converts camelCase keys to snake_case so remote responses match the existing
// interfaces with no changes elsewhere.

const CAMEL_RE = /([a-z0-9])([A-Z])/g;

/** camelCase / PascalCase → snake_case for a single key. */
export function camelToSnake(key: string): string {
  return key.replace(CAMEL_RE, "$1_$2").toLowerCase();
}

/**
 * Recursively convert object keys from camelCase to snake_case. Primitives and
 * arrays pass through (arrays element-wise). Internal protobuf-es keys (those
 * starting with `$`, e.g. `$typeName`) are dropped.
 */
export function protoToSnake<T = unknown>(input: unknown): T {
  if (Array.isArray(input)) {
    return input.map((v) => protoToSnake(v)) as unknown as T;
  }
  if (input !== null && typeof input === "object") {
    const out: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(input as Record<string, unknown>)) {
      if (key.startsWith("$")) continue;
      out[camelToSnake(key)] = protoToSnake(value);
    }
    return out as T;
  }
  // proto uint64/int64 fields arrive as BigInt; our TS interfaces declare `number`
  // and the app formats/does math on them. Counts (stars, comments, deltas, …) are
  // well within Number.MAX_SAFE_INTEGER, so coerce to keep remote === local shape.
  if (typeof input === "bigint") {
    return Number(input) as T;
  }
  return input as T;
}

/** Parse an opaque JSON string field, tolerating empty/invalid input. */
function parseJson<T>(raw: string | undefined, fallback: T): T {
  if (!raw) return fallback;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

/** RepoRefParams → the camelCase RepoRefRequest fields for the remote client. */
function repoRefReq(params: RepoRefParams) {
  return {
    id: params.id,
    owner: params.owner,
    repo: params.repo,
    perPage: params.per_page ?? undefined,
  };
}

/** proto AccountMessage → Account (parses config_json → config). */
function normalizeAccount(msg: { configJson?: string }): Account {
  const snake = protoToSnake<Record<string, unknown>>(msg);
  const { config_json, ...rest } = snake as Record<string, unknown> & {
    config_json?: string;
  };
  return { ...rest, config: parseJson<unknown>(config_json, {}) } as Account;
}

export const api = {
  // Tasks are a SYNCED entity: always local-first (the embedded store), in both
  // local and remote mode. The K3 sync engine propagates them to/from the server
  // in the background — remote mode no longer means "thin client" for tasks.
  listTasks: async (params: ListTasksParams = {}) => {
    return invoke<TaskDto[]>("list_tasks", { params });
  },
  createTask: async (params: CreateTaskParams) => {
    return invoke<TaskDto>("create_task", { params });
  },
  updateTask: async (params: UpdateTaskParams) => {
    return invoke<TaskDto>("update_task", { params });
  },
  completeTask: async (id: string) => {
    return invoke<TaskDto>("complete_task", { id });
  },
  deleteTask: async (id: string) => {
    return invoke<void>("delete_task", { id });
  },

  listAccounts: async () => {
    if (isRemote()) {
      const res = await getGrpcClient().listAccounts({});
      return res.accounts.map(normalizeAccount);
    }
    return invoke<Account[]>("list_accounts");
  },
  createAccount: async (params: CreateAccountParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().createAccount({
        provider: params.provider,
        displayName: params.display_name,
        baseUrl: params.base_url ?? undefined,
        configJson: JSON.stringify(params.config ?? {}),
      });
      return normalizeAccount(res);
    }
    // Local: no dedicated create_account command yet (the desktop uses the
    // OAuth flow via start_github_login). Kept for remote parity.
    return invoke<Account>("create_account", { params });
  },
  listRepos: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().listRepos({ id });
      return protoToSnake<Repo[]>(res.repos);
    }
    return invoke<Repo[]>("list_repos", { id });
  },
  listIssues: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().listIssues({ id });
      return protoToSnake<Issue[]>(res.issues);
    }
    return invoke<Issue[]>("list_issues", { id });
  },
  listPullRequests: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().listPullRequests({ id });
      return protoToSnake<PullRequest[]>(res.pullRequests);
    }
    return invoke<PullRequest[]>("list_pull_requests", { id });
  },
  deleteAccount: async (id: string) => {
    if (isRemote()) {
      await getGrpcClient().deleteAccount({ id });
      return;
    }
    return invoke<void>("delete_account", { id });
  },
  syncAccount: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().syncAccount({ id });
      return protoToSnake<SyncReport>(res);
    }
    return invoke<SyncReport>("sync_account", { id });
  },

  // --- daily snapshots / digest ---------------------------------------------
  // Bare-id commands take `{ id }`; `list_snapshots` carries a serde struct so
  // its fields stay snake_case under the `params` wrapper.
  captureSnapshots: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().captureSnapshots({ id });
      return Number(res.captured);
    }
    return invoke<number>("capture_snapshots", { id });
  },
  dailyDigest: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().dailyDigest({ id });
      return protoToSnake<Digest>(res);
    }
    return invoke<Digest>("daily_digest", { id });
  },
  listSnapshots: async (params: ListSnapshotsParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listSnapshots({
        id: params.id,
        sinceDay: params.since_day ?? undefined,
      });
      return protoToSnake<Snapshot[]>(res.snapshots);
    }
    return invoke<Snapshot[]>("list_snapshots", { params });
  },

  // --- triage write actions -------------------------------------------------
  // Each carries a serde params struct, so its fields stay snake_case under the
  // top-level `params` wrapper. These perform REAL writes to GitHub. The remote
  // request carries `number` as uint64, so it is widened to a bigint here.
  setIssueState: async (params: SetIssueStateParams) => {
    if (isRemote()) {
      await getGrpcClient().setIssueState({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
        state: params.state,
      });
      return;
    }
    return invoke<void>("set_issue_state", { params });
  },
  addIssueLabels: async (params: IssueLabelsParams) => {
    if (isRemote()) {
      await getGrpcClient().addIssueLabels({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
        labels: params.labels,
      });
      return;
    }
    return invoke<void>("add_issue_labels", { params });
  },
  removeIssueLabel: async (params: RemoveLabelParams) => {
    if (isRemote()) {
      await getGrpcClient().removeIssueLabel({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
        label: params.label,
      });
      return;
    }
    return invoke<void>("remove_issue_label", { params });
  },
  addIssueAssignees: async (params: IssueAssigneesParams) => {
    if (isRemote()) {
      await getGrpcClient().addIssueAssignees({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
        assignees: params.assignees,
      });
      return;
    }
    return invoke<void>("add_issue_assignees", { params });
  },
  createIssueComment: async (params: IssueCommentParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().createIssueComment({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
        body: params.body,
      });
      return res.htmlUrl;
    }
    return invoke<string>("create_issue_comment", { params });
  },

  // --- in-app issue / PR detail ---------------------------------------------
  // Each carries an IssueRefParams serde struct under the `params` wrapper, so
  // its fields stay snake_case. The detail reads can return null (not found);
  // the remote request carries `number` as uint64 (widened to a bigint here).
  getIssue: async (params: IssueRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().getIssue({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
      });
      return res.issue ? protoToSnake<IssueDetail>(res.issue) : null;
    }
    return invoke<IssueDetail | null>("get_issue", { params });
  },
  getPullRequest: async (params: IssueRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().getPullRequest({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
      });
      return res.pullRequest
        ? protoToSnake<PullRequestDetail>(res.pullRequest)
        : null;
    }
    return invoke<PullRequestDetail | null>("get_pull_request", { params });
  },
  listIssueComments: async (params: IssueRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listIssueComments({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
      });
      return protoToSnake<Comment[]>(res.comments);
    }
    return invoke<Comment[]>("list_issue_comments", { params });
  },
  listPullReviews: async (params: IssueRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listPullReviews({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        number: BigInt(params.number),
      });
      return protoToSnake<Review[]>(res.reviews);
    }
    return invoke<Review[]>("list_pull_reviews", { params });
  },

  // --- inbox / notifications ------------------------------------------------
  // Bare-id command; the `params`-wrapped command keeps its serde field names.
  listNotifications: async (id: string) => {
    if (isRemote()) {
      const res = await getGrpcClient().listNotifications({ id });
      return protoToSnake<Notification[]>(res.notifications);
    }
    return invoke<Notification[]>("list_notifications", { id });
  },
  markNotificationRead: async (id: string, threadId: string) => {
    if (isRemote()) {
      await getGrpcClient().markNotificationRead({ id, threadId });
      return;
    }
    return invoke<void>("mark_notification_read", {
      params: { id, thread_id: threadId },
    });
  },

  // --- CI / workflow runs ---------------------------------------------------
  listWorkflowRuns: async (params: ListWorkflowRunsParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listWorkflowRuns({
        id: params.id,
        owner: params.owner,
        repo: params.repo,
        perPage: params.per_page ?? undefined,
      });
      return protoToSnake<WorkflowRun[]>(res.runs);
    }
    return invoke<WorkflowRun[]>("list_workflow_runs", { params });
  },

  // --- repository details ---------------------------------------------------
  // All take an account id + repo coords; `per_page` is ignored by the detail
  // and languages endpoints. GetRepoDetail returns the message directly (no
  // wrapper).
  getRepoDetail: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().getRepoDetail(repoRefReq(params));
      return protoToSnake<RepoDetail>(res);
    }
    return invoke<RepoDetail>("get_repo_detail", { params });
  },
  listReleases: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listReleases(repoRefReq(params));
      return protoToSnake<Release[]>(res.releases);
    }
    return invoke<Release[]>("list_releases", { params });
  },
  listForks: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listForks(repoRefReq(params));
      return protoToSnake<Fork[]>(res.forks);
    }
    return invoke<Fork[]>("list_forks", { params });
  },
  listContributors: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listContributors(repoRefReq(params));
      return protoToSnake<Contributor[]>(res.contributors);
    }
    return invoke<Contributor[]>("list_contributors", { params });
  },
  getLanguages: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().getLanguages(repoRefReq(params));
      return protoToSnake<Language[]>(res.languages);
    }
    return invoke<Language[]>("get_languages", { params });
  },

  // --- traffic (push-access required; the views/clones endpoints return null
  // when the account lacks push access to the repository) --------------------
  getTrafficViews: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().getTrafficViews(repoRefReq(params));
      return res.traffic ? protoToSnake<Traffic>(res.traffic) : null;
    }
    return invoke<Traffic | null>("get_traffic_views", { params });
  },
  getTrafficClones: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().getTrafficClones(repoRefReq(params));
      return res.traffic ? protoToSnake<Traffic>(res.traffic) : null;
    }
    return invoke<Traffic | null>("get_traffic_clones", { params });
  },
  listReferrers: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listReferrers(repoRefReq(params));
      return protoToSnake<Referrer[]>(res.referrers);
    }
    return invoke<Referrer[]>("list_referrers", { params });
  },
  listPaths: async (params: RepoRefParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().listPaths(repoRefReq(params));
      return protoToSnake<RepoPath[]>(res.paths);
    }
    return invoke<RepoPath[]>("list_paths", { params });
  },

  // --- search (Mentions tab) ------------------------------------------------
  searchCode: async (params: SearchParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().searchCode({
        id: params.id,
        query: params.query,
        perPage: params.per_page ?? undefined,
      });
      return protoToSnake<CodeHit[]>(res.hits);
    }
    return invoke<CodeHit[]>("search_code", { params });
  },
  searchIssues: async (params: SearchParams) => {
    if (isRemote()) {
      const res = await getGrpcClient().searchIssues({
        id: params.id,
        query: params.query,
        perPage: params.per_page ?? undefined,
      });
      return protoToSnake<Issue[]>(res.issues);
    }
    return invoke<Issue[]>("search_issues", { params });
  },
  // Tauri auto-converts command arg names to camelCase, so the Rust params
  // `client_id`/`client_secret` are addressed as `clientId`/`clientSecret`.
  startGithubLogin: (clientId: string, clientSecret: string) =>
    invoke<Account>("start_github_login", { clientId, clientSecret }),

  // --- boards ---------------------------------------------------------------
  // Boards/columns/cards are SYNCED entities: always local-first (the embedded
  // store), in both local and remote mode. The K3 sync engine reconciles them
  // with the server in the background. The `params`-wrapped commands carry a
  // serde struct, so its fields stay snake_case (only the top-level command arg
  // names get camelCased by Tauri). Bare-id commands take `{ id }`.
  listBoards: async () => {
    return invoke<Board[]>("list_boards");
  },
  getBoard: async (id: string) => {
    return invoke<Board | null>("get_board", { id });
  },
  createBoard: async (name: string) => {
    return invoke<Board>("create_board", { params: { name } });
  },
  renameBoard: async (id: string, name: string) => {
    return invoke<Board>("rename_board", { params: { id, name } });
  },
  deleteBoard: async (id: string) => {
    return invoke<void>("delete_board", { id });
  },
  reorderBoards: async (orderedIds: string[]) => {
    return invoke<void>("reorder_boards", { params: { ordered_ids: orderedIds } });
  },

  createColumn: async (boardId: string, name: string, filter?: SmartFilter) => {
    return invoke<BoardColumn>("create_column", {
      params: { board_id: boardId, name, filter },
    });
  },
  updateColumn: async (id: string, name: string, filter?: SmartFilter) => {
    return invoke<BoardColumn>("update_column", { params: { id, name, filter } });
  },
  deleteColumn: async (id: string) => {
    return invoke<void>("delete_column", { id });
  },
  reorderColumns: async (boardId: string, orderedIds: string[]) => {
    return invoke<void>("reorder_columns", {
      params: { board_id: boardId, ordered_ids: orderedIds },
    });
  },

  placeCard: async (boardId: string, columnId: string, itemKey: string) => {
    return invoke<BoardCard>("place_card", {
      params: { board_id: boardId, column_id: columnId, item_key: itemKey },
    });
  },
  removeCard: async (boardId: string, itemKey: string) => {
    return invoke<void>("remove_card", {
      params: { board_id: boardId, item_key: itemKey },
    });
  },

  // --- server auth (remote-only) --------------------------------------------
  // These only make sense against a multi-user gitdeck-server, so they always
  // call the remote client directly (never the embedded local service). The
  // gRPC client returns camelCase; serverGetMe normalizes back to snake_case to
  // match the rest of the app's shapes.
  serverRegister: async (username: string, password: string) => {
    const res = await getGrpcClient().register({ username, password });
    return { user_id: res.userId };
  },
  serverLogin: async (username: string, password: string): Promise<string> => {
    const res = await getGrpcClient().login({ username, password });
    return res.token;
  },
  serverSetGithubToken: async (githubToken: string) => {
    await getGrpcClient().setGithubToken({ githubToken });
  },
  serverGetMe: async (): Promise<{
    user_id: string;
    username: string;
    has_github_token: boolean;
  }> => {
    const res = await getGrpcClient().getMe({});
    return {
      user_id: res.userId,
      username: res.username,
      has_github_token: res.hasGithubToken,
    };
  },
};
