import {
  useMutation,
  useQuery,
  useQueryClient,
  type UseQueryOptions,
} from "@tanstack/react-query";
import {
  api,
  type Board,
  type CodeHit,
  type Contributor,
  type Digest,
  type Fork,
  type Issue,
  type Language,
  type Notification,
  type PullRequest,
  type Referrer,
  type Release,
  type Repo,
  type RepoDetail,
  type RepoPath,
  type SetIssueStateParams,
  type IssueLabelsParams,
  type RemoveLabelParams,
  type IssueAssigneesParams,
  type IssueCommentParams,
  type IssueDetail,
  type PullRequestDetail,
  type Comment,
  type Review,
  type User,
  type TaskDto,
  type Traffic,
  type WorkflowRun,
} from "../api";

/**
 * Centralized query-key factory. Every account-scoped key embeds the account id
 * (and any params) so switching accounts/params caches separately and refetches
 * on change. Keep keys stable and serializable for the localStorage persister.
 */
export const queryKeys = {
  repos: (accountId: string | null) => ["repos", accountId] as const,
  issues: (accountId: string | null) => ["issues", accountId] as const,
  pulls: (accountId: string | null) => ["pulls", accountId] as const,
  tasks: () => ["tasks"] as const,
  notifications: (accountId: string | null) =>
    ["notifications", accountId] as const,
  boards: () => ["boards"] as const,
  board: (boardId: string | undefined) => ["board", boardId] as const,
  digest: (accountId: string | null) => ["digest", accountId] as const,
  workflowRuns: (
    accountId: string | null,
    owner: string,
    repo: string,
    perPage?: number,
  ) => ["workflow-runs", accountId, owner, repo, perPage ?? null] as const,
  repoDetail: (accountId: string | null, owner: string, repo: string) =>
    ["repo-detail", accountId, owner, repo] as const,
  releases: (accountId: string | null, owner: string, repo: string) =>
    ["releases", accountId, owner, repo] as const,
  forks: (accountId: string | null, owner: string, repo: string) =>
    ["forks", accountId, owner, repo] as const,
  contributors: (accountId: string | null, owner: string, repo: string) =>
    ["contributors", accountId, owner, repo] as const,
  languages: (accountId: string | null, owner: string, repo: string) =>
    ["languages", accountId, owner, repo] as const,
  trafficViews: (accountId: string | null, owner: string, repo: string) =>
    ["traffic-views", accountId, owner, repo] as const,
  trafficClones: (accountId: string | null, owner: string, repo: string) =>
    ["traffic-clones", accountId, owner, repo] as const,
  referrers: (accountId: string | null, owner: string, repo: string) =>
    ["referrers", accountId, owner, repo] as const,
  paths: (accountId: string | null, owner: string, repo: string) =>
    ["paths", accountId, owner, repo] as const,
  searchCode: (accountId: string | null, query: string) =>
    ["search-code", accountId, query] as const,
  searchIssues: (accountId: string | null, query: string) =>
    ["search-issues", accountId, query] as const,
  issueDetail: (
    accountId: string | null,
    owner: string,
    repo: string,
    number: number,
  ) => ["issue-detail", accountId, owner, repo, number] as const,
  pullDetail: (
    accountId: string | null,
    owner: string,
    repo: string,
    number: number,
  ) => ["pull-detail", accountId, owner, repo, number] as const,
  issueComments: (
    accountId: string | null,
    owner: string,
    repo: string,
    number: number,
  ) => ["issue-comments", accountId, owner, repo, number] as const,
  pullReviews: (
    accountId: string | null,
    owner: string,
    repo: string,
    number: number,
  ) => ["pull-reviews", accountId, owner, repo, number] as const,
};

/** Staleness tiers. Lists change often; insights/traffic move slowly. */
const STALE_LIST = 60_000; // 1 min
const STALE_SLOW = 1000 * 60 * 10; // 10 min — traffic / insights / detail

// --- account-scoped list reads ---------------------------------------------

export function useRepos(accountId: string | null) {
  return useQuery({
    queryKey: queryKeys.repos(accountId),
    queryFn: () => api.listRepos(accountId as string),
    enabled: !!accountId,
    staleTime: STALE_LIST,
  });
}

export function useIssues(accountId: string | null) {
  return useQuery({
    queryKey: queryKeys.issues(accountId),
    queryFn: () => api.listIssues(accountId as string),
    enabled: !!accountId,
    staleTime: STALE_LIST,
  });
}

export function usePulls(accountId: string | null) {
  return useQuery({
    queryKey: queryKeys.pulls(accountId),
    queryFn: () => api.listPullRequests(accountId as string),
    enabled: !!accountId,
    staleTime: STALE_LIST,
  });
}

export function useTasks() {
  return useQuery({
    queryKey: queryKeys.tasks(),
    queryFn: () => api.listTasks(),
    staleTime: STALE_LIST,
  });
}

export function useNotifications(accountId: string | null) {
  return useQuery({
    queryKey: queryKeys.notifications(accountId),
    queryFn: () => api.listNotifications(accountId as string),
    enabled: !!accountId,
    staleTime: STALE_LIST,
  });
}

export function useDigest(accountId: string | null) {
  return useQuery({
    queryKey: queryKeys.digest(accountId),
    // Record today's snapshot first (best-effort), then read the digest. A
    // capture failure never blocks showing whatever digest data exists.
    queryFn: async (): Promise<Digest> => {
      try {
        await api.captureSnapshots(accountId as string);
      } catch {
        // ignore — fall through to the digest read
      }
      return api.dailyDigest(accountId as string);
    },
    enabled: !!accountId,
    staleTime: STALE_SLOW,
  });
}

// --- boards -----------------------------------------------------------------

export function useBoards() {
  return useQuery({
    queryKey: queryKeys.boards(),
    queryFn: () => api.listBoards(),
    staleTime: STALE_LIST,
  });
}

export function useBoard(boardId: string | undefined) {
  return useQuery({
    queryKey: queryKeys.board(boardId),
    queryFn: () => api.getBoard(boardId as string),
    enabled: !!boardId,
    staleTime: STALE_LIST,
  });
}

// --- per-repo detail reads (each individually cached) ----------------------

export function useWorkflowRuns(
  accountId: string | null,
  owner: string,
  repo: string,
  perPage?: number,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.workflowRuns(accountId, owner, repo, perPage),
    queryFn: () =>
      api.listWorkflowRuns({
        id: accountId as string,
        owner,
        repo,
        per_page: perPage ?? null,
      }),
    enabled: !!accountId && !!owner && !!repo && options?.enabled !== false,
    staleTime: STALE_LIST,
  });
}

function repoDetailEnabled(
  accountId: string | null,
  owner: string,
  repo: string,
  enabled?: boolean,
): boolean {
  return !!accountId && !!owner && !!repo && enabled !== false;
}

export function useRepoDetail(
  accountId: string | null,
  owner: string,
  repo: string,
) {
  return useQuery({
    queryKey: queryKeys.repoDetail(accountId, owner, repo),
    queryFn: () =>
      api.getRepoDetail({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo),
    staleTime: STALE_SLOW,
  });
}

export function useLanguages(
  accountId: string | null,
  owner: string,
  repo: string,
) {
  return useQuery({
    queryKey: queryKeys.languages(accountId, owner, repo),
    queryFn: () => api.getLanguages({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo),
    staleTime: STALE_SLOW,
  });
}

export function useReleases(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.releases(accountId, owner, repo),
    queryFn: () => api.listReleases({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

export function useForks(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.forks(accountId, owner, repo),
    queryFn: () => api.listForks({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

export function useContributors(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.contributors(accountId, owner, repo),
    queryFn: () =>
      api.listContributors({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

export function useTrafficViews(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.trafficViews(accountId, owner, repo),
    queryFn: () =>
      api.getTrafficViews({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

export function useTrafficClones(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.trafficClones(accountId, owner, repo),
    queryFn: () =>
      api.getTrafficClones({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

export function useReferrers(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.referrers(accountId, owner, repo),
    queryFn: () => api.listReferrers({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

export function usePaths(
  accountId: string | null,
  owner: string,
  repo: string,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.paths(accountId, owner, repo),
    queryFn: () => api.listPaths({ id: accountId as string, owner, repo }),
    enabled: repoDetailEnabled(accountId, owner, repo, options?.enabled),
    staleTime: STALE_SLOW,
  });
}

// --- search (Mentions tab) --------------------------------------------------

export function useSearchCode(
  accountId: string | null,
  query: string,
  options?: { enabled?: boolean; perPage?: number },
) {
  return useQuery({
    queryKey: queryKeys.searchCode(accountId, query),
    queryFn: () =>
      api.searchCode({
        id: accountId as string,
        query,
        per_page: options?.perPage ?? null,
      }),
    enabled: !!accountId && !!query && options?.enabled !== false,
    staleTime: STALE_SLOW,
  });
}

export function useSearchIssues(
  accountId: string | null,
  query: string,
  options?: { enabled?: boolean; perPage?: number },
) {
  return useQuery({
    queryKey: queryKeys.searchIssues(accountId, query),
    queryFn: () =>
      api.searchIssues({
        id: accountId as string,
        query,
        per_page: options?.perPage ?? null,
      }),
    enabled: !!accountId && !!query && options?.enabled !== false,
    staleTime: STALE_SLOW,
  });
}

// --- in-app issue / PR detail reads ----------------------------------------
//
// Each is individually cached on the SLOW (10-min) tier like the other detail
// reads, and gated on the account id + repo coords being present.

function detailEnabled(
  accountId: string | null,
  owner: string,
  repo: string,
  number: number,
): boolean {
  return !!accountId && !!owner && !!repo && Number.isFinite(number);
}

export function useIssueDetail(
  accountId: string | null,
  owner: string,
  repo: string,
  number: number,
) {
  return useQuery({
    queryKey: queryKeys.issueDetail(accountId, owner, repo, number),
    queryFn: () =>
      api.getIssue({ id: accountId as string, owner, repo, number }),
    enabled: detailEnabled(accountId, owner, repo, number),
    staleTime: STALE_SLOW,
  });
}

export function usePullRequestDetail(
  accountId: string | null,
  owner: string,
  repo: string,
  number: number,
) {
  return useQuery({
    queryKey: queryKeys.pullDetail(accountId, owner, repo, number),
    queryFn: () =>
      api.getPullRequest({ id: accountId as string, owner, repo, number }),
    enabled: detailEnabled(accountId, owner, repo, number),
    staleTime: STALE_SLOW,
  });
}

export function useIssueComments(
  accountId: string | null,
  owner: string,
  repo: string,
  number: number,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.issueComments(accountId, owner, repo, number),
    queryFn: () =>
      api.listIssueComments({ id: accountId as string, owner, repo, number }),
    enabled:
      detailEnabled(accountId, owner, repo, number) &&
      options?.enabled !== false,
    staleTime: STALE_SLOW,
  });
}

export function usePullReviews(
  accountId: string | null,
  owner: string,
  repo: string,
  number: number,
  options?: { enabled?: boolean },
) {
  return useQuery({
    queryKey: queryKeys.pullReviews(accountId, owner, repo, number),
    queryFn: () =>
      api.listPullReviews({ id: accountId as string, owner, repo, number }),
    enabled:
      detailEnabled(accountId, owner, repo, number) &&
      options?.enabled !== false,
    staleTime: STALE_SLOW,
  });
}

// --- triage mutations -------------------------------------------------------
//
// Each write invalidates the account's issues + pulls so the lists reflect the
// change on the next read. Triage keeps its own optimistic local state, so we
// invalidate (rather than optimistically mutate the cache) on success.

function useInvalidateIssueLists(accountId: string | null) {
  const qc = useQueryClient();
  return () => {
    void qc.invalidateQueries({ queryKey: queryKeys.issues(accountId) });
    void qc.invalidateQueries({ queryKey: queryKeys.pulls(accountId) });
    // Also refresh any open in-app detail view (and its comment/review
    // timelines) so a triage write from the detail re-renders the new state.
    // We invalidate the whole family (predicate on the key prefix) since a
    // mutation doesn't carry which specific number is on screen.
    for (const prefix of [
      "issue-detail",
      "pull-detail",
      "issue-comments",
      "pull-reviews",
    ]) {
      void qc.invalidateQueries({
        predicate: (q) =>
          q.queryKey[0] === prefix && q.queryKey[1] === accountId,
      });
    }
  };
}

export function useSetIssueState(accountId: string | null) {
  const invalidate = useInvalidateIssueLists(accountId);
  return useMutation({
    mutationFn: (params: SetIssueStateParams) => api.setIssueState(params),
    onSuccess: invalidate,
  });
}

export function useAddIssueLabels(accountId: string | null) {
  const invalidate = useInvalidateIssueLists(accountId);
  return useMutation({
    mutationFn: (params: IssueLabelsParams) => api.addIssueLabels(params),
    onSuccess: invalidate,
  });
}

export function useRemoveIssueLabel(accountId: string | null) {
  const invalidate = useInvalidateIssueLists(accountId);
  return useMutation({
    mutationFn: (params: RemoveLabelParams) => api.removeIssueLabel(params),
    onSuccess: invalidate,
  });
}

export function useAddIssueAssignees(accountId: string | null) {
  const invalidate = useInvalidateIssueLists(accountId);
  return useMutation({
    mutationFn: (params: IssueAssigneesParams) => api.addIssueAssignees(params),
    onSuccess: invalidate,
  });
}

export function useCreateIssueComment(accountId: string | null) {
  const invalidate = useInvalidateIssueLists(accountId);
  return useMutation({
    mutationFn: (params: IssueCommentParams) => api.createIssueComment(params),
    onSuccess: invalidate,
  });
}

export function useMarkNotificationRead(accountId: string | null) {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, threadId }: { id: string; threadId: string }) =>
      api.markNotificationRead(id, threadId),
    onSuccess: () => {
      void qc.invalidateQueries({
        queryKey: queryKeys.notifications(accountId),
      });
    },
  });
}

// Re-export types some callers may want alongside the hooks.
export type {
  Board,
  CodeHit,
  Comment,
  Contributor,
  Digest,
  Fork,
  Issue,
  IssueDetail,
  Language,
  PullRequestDetail,
  Review,
  User,
  Notification,
  PullRequest,
  Referrer,
  Release,
  Repo,
  RepoDetail,
  RepoPath,
  TaskDto,
  Traffic,
  WorkflowRun,
  UseQueryOptions,
};
