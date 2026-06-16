import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useQueries } from "@tanstack/react-query";
import { Inbox } from "lucide-react";
import { api, type Repo, type WorkflowRun } from "../../lib/api";
import { formatRelativeTime } from "../../lib/format";
import { queryKeys, useRepos } from "../../lib/query/queries";
import { Skeleton } from "../../components/ui/Skeleton";
import { useAccounts } from "../../contexts/AccountContext";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
} from "../dashboard/sectionScaffold";

/** How many repos (most-recently-updated) we probe for their latest run. */
const REPO_CAP = 20;

type CiStatus = "passing" | "failing" | "running" | "none" | "unknown";

interface RepoCi {
  repo: Repo;
  status: CiStatus;
  run: WorkflowRun | null;
}

/** Map a workflow run's status+conclusion onto a coarse CI status. */
export function ciStatusOf(run: WorkflowRun | null): CiStatus {
  if (!run) return "none";
  if (run.status === "in_progress" || run.status === "queued") return "running";
  if (run.status === "completed") {
    const c = run.conclusion;
    if (c === "success") return "passing";
    if (c === "failure" || c === "timed_out" || c === "startup_failure")
      return "failing";
    // neutral / cancelled / skipped / action_required → treat as unknown
    return "unknown";
  }
  return "unknown";
}

const DOT: Record<CiStatus, string> = {
  passing: "bg-open",
  failing: "bg-closed",
  running: "bg-warn motion-safe:animate-pulse",
  none: "bg-text-faint",
  unknown: "bg-text-faint",
};

function splitFullName(fullName: string): { owner: string; repo: string } {
  const idx = fullName.indexOf("/");
  if (idx < 0) return { owner: fullName, repo: "" };
  return {
    owner: fullName.slice(0, idx),
    repo: fullName.slice(idx + 1),
  };
}

function statusLabel(status: CiStatus, t: (k: string) => string): string {
  switch (status) {
    case "passing":
      return t("ci_passing");
    case "failing":
      return t("ci_failing");
    case "running":
      return t("ci_running");
    case "none":
      return t("ci_no_runs");
    default:
      return t("ci_unknown");
  }
}

function RepoCiCard({ entry }: { entry: RepoCi }) {
  const { t, i18n } = useTranslation();
  const { repo, run, status } = entry;
  const hasRun = run !== null;

  function open() {
    if (run) void openUrl(run.html_url).catch(() => {});
  }

  return (
    <article
      className={
        "group flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4 " +
        "transition-[transform,color,background-color,border-color] duration-200 ease-out " +
        "hover:-translate-y-px hover:border-border-strong hover:bg-surface-2 " +
        "motion-reduce:transform-none motion-reduce:transition-none"
      }
    >
      <div className="flex items-center gap-2">
        <span aria-hidden className={"size-2.5 shrink-0 rounded-full " + DOT[status]} />
        <button
          type="button"
          onClick={open}
          disabled={!hasRun}
          title={repo.full_name}
          className={
            "min-w-0 flex-1 truncate text-left font-mono text-[13px] text-text " +
            "transition-colors duration-150 ease-out " +
            (hasRun ? "hover:text-accent cursor-pointer" : "cursor-default") +
            " outline-none focus-visible:ring-2 focus-visible:ring-accent " +
            "focus-visible:ring-offset-2 focus-visible:ring-offset-surface " +
            "motion-reduce:transition-none rounded-[--radius-sm]"
          }
        >
          {repo.full_name}
        </button>
        <span
          className={
            "shrink-0 text-[11px] font-medium uppercase tracking-wide " +
            (status === "passing"
              ? "text-open"
              : status === "failing"
                ? "text-closed"
                : status === "running"
                  ? "text-warn"
                  : "text-text-faint")
          }
        >
          {statusLabel(status, t)}
        </span>
      </div>

      {run ? (
        <>
          <p className="truncate text-[13px] text-text-muted">
            {run.name || repo.full_name}
          </p>
          <div className="mt-auto flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px] text-text-faint">
            {run.head_branch && (
              <span className="truncate font-mono" title={run.head_branch}>
                {run.head_branch}
              </span>
            )}
            {run.event && <span>{run.event}</span>}
            <span className="ml-auto whitespace-nowrap">
              {formatRelativeTime(run.updated_at, Date.now(), i18n.language)}
            </span>
          </div>
        </>
      ) : (
        <p className="mt-auto text-[12px] text-text-faint">{t("ci_no_runs")}</p>
      )}
    </article>
  );
}

function CiSkeleton({ count = 6 }: { count?: number }) {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {Array.from({ length: count }, (_, i) => (
        <div
          key={i}
          className="flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4"
        >
          <div className="flex items-center gap-2">
            <Skeleton className="size-2.5 rounded-full" />
            <Skeleton className="h-4 w-2/3" />
          </div>
          <Skeleton className="h-3 w-1/2" />
          <Skeleton className="h-3 w-1/3" />
        </div>
      ))}
    </div>
  );
}

export default function CIHealthView() {
  const { t } = useTranslation();
  const { accounts, activeAccountId } = useAccounts();
  const account =
    accounts.find((a) => a.id === activeAccountId)?.display_name ??
    activeAccountId ??
    null;

  const [filter, setFilter] = useState<"all" | "passing" | "failing">("all");

  // The repo list itself is a cached query.
  const reposQuery = useRepos(activeAccountId);
  const allRepos = useMemo(
    () => (Array.isArray(reposQuery.data) ? reposQuery.data : []),
    [reposQuery.data],
  );
  const totalRepos = allRepos.length;

  // Most-recently-updated repos, capped — each gets its own latest-run query.
  const slice = useMemo(
    () =>
      [...allRepos]
        .sort((a, b) => b.updated_at.localeCompare(a.updated_at))
        .slice(0, REPO_CAP),
    [allRepos],
  );

  // PARALLEL async win: one query per repo, all fired together and each cached
  // individually under its own ['workflow-runs', accountId, owner, repo] key.
  // Revisiting a repo elsewhere (e.g. its detail Actions tab) reuses this cache.
  const runQueries = useQueries({
    queries: slice.map((repo) => {
      const { owner, repo: name } = splitFullName(repo.full_name);
      return {
        queryKey: queryKeys.workflowRuns(activeAccountId, owner, name, 1),
        queryFn: () =>
          api.listWorkflowRuns({
            id: activeAccountId as string,
            owner,
            repo: name,
            per_page: 1,
          }),
        enabled: !!activeAccountId,
        staleTime: 60_000,
      };
    }),
  });

  const entries = useMemo<RepoCi[]>(
    () =>
      slice.map((repo, i) => {
        const q = runQueries[i];
        if (q?.isError) return { repo, run: null, status: "unknown" };
        const runs = q?.data;
        const run = Array.isArray(runs) && runs.length > 0 ? runs[0] : null;
        return { repo, run, status: ciStatusOf(run) };
      }),
    [slice, runQueries],
  );

  // Loading while the repo list loads, or while any per-repo run query is still
  // fetching for the first time.
  const reposLoading =
    reposQuery.isPending && reposQuery.fetchStatus !== "idle";
  const runsLoading = runQueries.some((q) => q.isPending && q.fetchStatus !== "idle");
  const loading = reposLoading || (slice.length > 0 && runsLoading);
  const error = reposQuery.isError
    ? reposQuery.error instanceof Error
      ? reposQuery.error.message
      : String(reposQuery.error)
    : null;

  const counts = useMemo(() => {
    let passing = 0;
    let failing = 0;
    let running = 0;
    for (const e of entries) {
      if (e.status === "passing") passing += 1;
      else if (e.status === "failing") failing += 1;
      else if (e.status === "running") running += 1;
    }
    return { passing, failing, running };
  }, [entries]);

  const visible = useMemo(() => {
    if (filter === "passing") return entries.filter((e) => e.status === "passing");
    if (filter === "failing") return entries.filter((e) => e.status === "failing");
    return entries;
  }, [entries, filter]);

  const FILTERS: ("all" | "passing" | "failing")[] = ["all", "passing", "failing"];
  const focusRing =
    "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
    "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

  return (
    <SectionShell>
      <SectionHeader title={t("ci_title")} account={account} />

      {accounts.length === 0 ? (
        <NoAccount />
      ) : loading ? (
        <CiSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : entries.length === 0 ? (
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <Inbox size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <p className="text-sm text-text-muted">{t("ci_empty")}</p>
        </div>
      ) : (
        <>
          <div className="mb-4 flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-4 text-[13px]">
              <span className="flex items-center gap-1.5 text-text-muted">
                <span aria-hidden className="size-2 rounded-full bg-open" />
                <span className="tnum">{counts.passing}</span> {t("ci_passing")}
              </span>
              <span className="flex items-center gap-1.5 text-text-muted">
                <span aria-hidden className="size-2 rounded-full bg-closed" />
                <span className="tnum">{counts.failing}</span> {t("ci_failing")}
              </span>
              <span className="flex items-center gap-1.5 text-text-muted">
                <span aria-hidden className="size-2 rounded-full bg-warn" />
                <span className="tnum">{counts.running}</span> {t("ci_running")}
              </span>
            </div>

            <div
              role="group"
              aria-label={t("filter_state")}
              className="ml-auto flex gap-0.5 rounded-[--radius] border border-border bg-surface p-0.5"
            >
              {FILTERS.map((f) => (
                <button
                  key={f}
                  type="button"
                  aria-pressed={filter === f}
                  onClick={() => setFilter(f)}
                  className={
                    "rounded-[--radius-sm] px-2.5 py-1 text-[13px] " +
                    "transition-[color,background-color] duration-150 ease-out " +
                    focusRing +
                    " " +
                    (filter === f
                      ? "bg-surface-2 font-medium text-text"
                      : "text-text-muted hover:text-text")
                  }
                >
                  {f === "all"
                    ? t("ci_filter_all")
                    : f === "passing"
                      ? t("ci_filter_passing")
                      : t("ci_filter_failing")}
                </button>
              ))}
            </div>
          </div>

          {totalRepos > entries.length && (
            <p className="mb-3 text-[12px] text-text-faint">
              {t("ci_showing_n_of_m", {
                shown: entries.length,
                total: totalRepos,
              })}
            </p>
          )}

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
            {visible.map((entry) => (
              <RepoCiCard key={entry.repo.id} entry={entry} />
            ))}
          </div>
        </>
      )}
    </SectionShell>
  );
}
