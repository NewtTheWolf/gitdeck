import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Star, CircleDot, ExternalLink, Inbox } from "lucide-react";
import { useRepos } from "../../lib/query/queries";
import {
  computeInsights,
  type InsightAlert,
  type InsightStatus,
  type RepoInsight,
} from "../../lib/insights";
import { formatStars } from "../../lib/format";
import { repoDetailPath } from "../../lib/repoRoute";
import { Skeleton } from "../../components/ui/Skeleton";
import { useAccounts } from "../../contexts/AccountContext";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
} from "../dashboard/sectionScaffold";

// TODO: traffic correlation — gitdeck cross-references views/clones to flag
// neglected-but-popular repos more precisely. We compute purely from the repo
// list here and skip per-repo traffic fetches in this MVP.

type StatusFilter = "all" | InsightStatus;

const STATUS_DOT: Record<InsightStatus, string> = {
  strong: "bg-open",
  watch: "bg-warn",
  risky: "bg-closed",
};

const STATUS_TEXT: Record<InsightStatus, string> = {
  strong: "text-open",
  watch: "text-warn",
  risky: "text-closed",
};

const SEVERITY_TEXT: Record<InsightAlert["severity"], string> = {
  info: "text-text-faint",
  warn: "text-warn",
  risk: "text-closed",
};

function openRepo(url: string) {
  void openUrl(url).catch(() => {});
}

function InsightCard({ item }: { item: RepoInsight }) {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { repo, status, daysSinceActivity, alerts, opportunities } = item;

  return (
    <article
      className={
        "group flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4 " +
        "transition-[transform,color,background-color,border-color] duration-200 ease-out " +
        "hover:-translate-y-px hover:border-border-strong hover:bg-surface-2 " +
        "motion-reduce:transform-none motion-reduce:transition-none"
      }
    >
      <div className="flex items-start gap-2">
        <span
          aria-hidden
          className={"mt-1 size-2.5 shrink-0 rounded-full " + STATUS_DOT[status]}
        />
        <button
          type="button"
          onClick={() => navigate(repoDetailPath(repo.full_name))}
          title={repo.full_name}
          className={
            "min-w-0 flex-1 truncate text-left font-mono text-[13px] text-text " +
            "transition-colors duration-150 ease-out hover:text-accent " +
            "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
            "focus-visible:ring-offset-2 focus-visible:ring-offset-surface " +
            "motion-reduce:transition-none rounded-[--radius-sm]"
          }
        >
          {repo.full_name}
        </button>
        <span
          className={
            "shrink-0 text-[11px] font-medium uppercase tracking-wide " +
            STATUS_TEXT[status]
          }
        >
          {t("insights_status_" + status)}
        </span>
        <button
          type="button"
          onClick={() => openRepo(repo.html_url)}
          title={t("repo_open_on_github")}
          aria-label={t("repo_open_on_github")}
          className={
            "shrink-0 rounded-[--radius-sm] p-0.5 text-text-faint " +
            "transition-colors duration-150 ease-out hover:text-text " +
            "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
            "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none"
          }
        >
          <ExternalLink size={13} strokeWidth={1.75} aria-hidden />
        </button>
      </div>

      <div className="flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-text-muted">
        <span className="whitespace-nowrap">
          {t("insights_active_days_ago", { count: daysSinceActivity })}
        </span>
        <span className="flex items-center gap-1.5" title={String(repo.stars)}>
          <Star size={14} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <span className="tnum">{formatStars(repo.stars)}</span>
        </span>
        <span
          className="flex items-center gap-1.5"
          title={t("repo_open_issues", { count: repo.open_issues })}
        >
          <CircleDot size={14} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <span className="tnum">{repo.open_issues}</span>
        </span>
      </div>

      {alerts.length > 0 && (
        <div className="flex flex-wrap gap-1.5">
          {alerts.map((alert, i) => (
            <span
              key={alert.kind + i}
              className={
                "inline-flex items-center gap-1 rounded-full border border-border " +
                "bg-surface-2 px-2 py-0.5 text-[11px] font-medium " +
                SEVERITY_TEXT[alert.severity]
              }
            >
              <span aria-hidden className="size-1.5 rounded-full bg-current" />
              {t(alert.messageKey, alert.values)}
            </span>
          ))}
        </div>
      )}

      {opportunities.length > 0 && (
        <p className="text-[12px] leading-relaxed text-text-faint">
          {opportunities.map((key) => t(key)).join(" · ")}
        </p>
      )}
    </article>
  );
}

function InsightsSkeleton({ count = 6 }: { count?: number }) {
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
          <div className="flex gap-4">
            <Skeleton className="h-3 w-16" />
            <Skeleton className="h-3 w-10" />
            <Skeleton className="h-3 w-10" />
          </div>
          <Skeleton className="h-4 w-1/2" />
        </div>
      ))}
    </div>
  );
}

export default function InsightsView() {
  const { t } = useTranslation();
  const { accounts, activeAccountId } = useAccounts();
  const account =
    accounts.find((a) => a.id === activeAccountId)?.display_name ??
    activeAccountId ??
    null;

  const [filter, setFilter] = useState<StatusFilter>("all");

  const query = useRepos(activeAccountId);
  const repos = useMemo(
    () => (Array.isArray(query.data) ? query.data : []),
    [query.data],
  );
  const loading = query.isPending && query.fetchStatus !== "idle";
  const error = query.isError
    ? query.error instanceof Error
      ? query.error.message
      : String(query.error)
    : null;

  const { items, summary } = useMemo(
    () => computeInsights(repos, Date.now()),
    [repos],
  );

  const visible = useMemo(() => {
    if (filter === "all") return items;
    return items.filter((i) => i.status === filter);
  }, [items, filter]);

  const FILTERS: StatusFilter[] = ["all", "risky", "watch", "strong"];
  const focusRing =
    "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
    "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

  return (
    <SectionShell>
      <SectionHeader title={t("insights_title")} account={account} />

      {accounts.length === 0 ? (
        <NoAccount />
      ) : loading ? (
        <InsightsSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : items.length === 0 ? (
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <Inbox size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <p className="text-sm text-text-muted">{t("dashboard_no_repos")}</p>
        </div>
      ) : (
        <>
          <div className="mb-4 flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-4 text-[13px]">
              <span className="flex items-center gap-1.5 text-text-muted">
                <span aria-hidden className="size-2 rounded-full bg-open" />
                <span className="tnum">{summary.strong}</span> {t("insights_status_strong")}
              </span>
              <span className="flex items-center gap-1.5 text-text-muted">
                <span aria-hidden className="size-2 rounded-full bg-warn" />
                <span className="tnum">{summary.watch}</span> {t("insights_status_watch")}
              </span>
              <span className="flex items-center gap-1.5 text-text-muted">
                <span aria-hidden className="size-2 rounded-full bg-closed" />
                <span className="tnum">{summary.risky}</span> {t("insights_status_risky")}
              </span>
              <span className="text-text-faint">
                {t("insights_total", { count: summary.total })}
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
                  {f === "all" ? t("insights_filter_all") : t("insights_status_" + f)}
                </button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
            {visible.map((item) => (
              <InsightCard key={item.repo.id} item={item} />
            ))}
          </div>
        </>
      )}
    </SectionShell>
  );
}
