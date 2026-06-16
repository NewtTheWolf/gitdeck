import { useCallback, useMemo, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  CircleDot,
  GitPullRequest,
  Inbox,
  MessagesSquare,
  Tag,
  Bell,
  Check,
} from "lucide-react";
import { type Notification } from "../../lib/api";
import { formatRelativeTime } from "../../lib/format";
import { issueDetailPath, pullDetailPath } from "../../lib/itemRoute";
import {
  useNotifications,
  useMarkNotificationRead,
} from "../../lib/query/queries";
import { Skeleton } from "../../components/ui/Skeleton";
import { useAccounts } from "../../contexts/AccountContext";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
} from "../dashboard/sectionScaffold";

/**
 * Best-effort conversion of a GitHub API subject URL to its web URL.
 * e.g. `https://api.github.com/repos/o/r/issues/123` →
 * `https://github.com/o/r/issues/123`, and `/pulls/` → `/pull/`.
 * Returns null when the input can't be confidently converted to a web link.
 */
export function notificationWebUrl(
  subjectUrl: string | null | undefined,
): string | null {
  if (!subjectUrl) return null;
  if (!subjectUrl.includes("api.github.com/repos/")) return null;
  return subjectUrl
    .replace("api.github.com/repos", "github.com")
    .replace("/pulls/", "/pull/");
}

/**
 * Derive the in-app detail path for an Issue / PullRequest notification from its
 * GitHub API subject URL, e.g.
 *   `https://api.github.com/repos/o/r/issues/123` → `/repos/o/r/issues/123`
 *   `https://api.github.com/repos/o/r/pulls/45`   → `/repos/o/r/pull/45`
 * Returns null for anything that isn't a parseable Issue/PR subject.
 */
export function notificationDetailPath(
  subjectType: string,
  subjectUrl: string | null | undefined,
): string | null {
  if (!subjectUrl) return null;
  if (subjectType !== "Issue" && subjectType !== "PullRequest") return null;
  const m = subjectUrl.match(
    /repos\/([^/]+)\/([^/]+)\/(?:issues|pulls)\/(\d+)/,
  );
  if (!m) return null;
  const [, owner, repo, num] = m;
  const number = Number(num);
  const fullName = `${owner}/${repo}`;
  return subjectType === "PullRequest"
    ? pullDetailPath(fullName, number)
    : issueDetailPath(fullName, number);
}

type Filter = "all" | "unread";

function SubjectIcon({ type }: { type: string }) {
  const cls = "shrink-0";
  const size = 15;
  const sw = 1.75;
  switch (type) {
    case "Issue":
      return <CircleDot size={size} strokeWidth={sw} aria-hidden className={cls + " text-open"} />;
    case "PullRequest":
      return <GitPullRequest size={size} strokeWidth={sw} aria-hidden className={cls + " text-merged"} />;
    case "Release":
      return <Tag size={size} strokeWidth={sw} aria-hidden className={cls + " text-accent"} />;
    case "Discussion":
      return <MessagesSquare size={size} strokeWidth={sw} aria-hidden className={cls + " text-text-muted"} />;
    default:
      return <Bell size={size} strokeWidth={sw} aria-hidden className={cls + " text-text-muted"} />;
  }
}

function ReasonBadge({ reason }: { reason: string }) {
  return (
    <span className="inline-flex items-center rounded-full border border-border bg-surface-2 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-text-faint">
      {reason.replace(/_/g, " ")}
    </span>
  );
}

function NotificationRow({
  item,
  onMarkRead,
}: {
  item: Notification;
  onMarkRead: (id: string) => void;
}) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const detailPath = notificationDetailPath(
    item.subject_type,
    item.subject_url,
  );
  const webUrl = notificationWebUrl(item.subject_url);
  const clickable = !!detailPath || !!webUrl;

  function open() {
    // Issue/PR subjects open the in-app detail; everything else (releases,
    // discussions, …) falls back to opening the web url in the browser.
    if (detailPath) navigate(detailPath);
    else if (webUrl) void openUrl(webUrl).catch(() => {});
  }

  return (
    <div
      className={
        "group flex items-start gap-3 border-b border-border px-4 py-3 last:border-b-0 " +
        "transition-colors duration-150 ease-out " +
        (item.unread
          ? "border-l-2 border-l-accent bg-surface-2"
          : "border-l-2 border-l-transparent bg-surface")
      }
    >
      <span className="mt-0.5 flex items-center gap-2">
        <span
          aria-hidden
          className={
            "size-1.5 shrink-0 rounded-full " +
            (item.unread ? "bg-accent" : "bg-transparent")
          }
        />
        <SubjectIcon type={item.subject_type} />
      </span>

      <button
        type="button"
        onClick={open}
        disabled={!clickable}
        className={
          "min-w-0 flex-1 text-left " +
          "outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent " +
          "rounded-[--radius-sm] motion-reduce:transition-none " +
          (clickable ? "cursor-pointer" : "cursor-default")
        }
      >
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[12px]">
          <span className="font-mono text-text-muted">
            {item.repo_name_with_owner}
          </span>
          <ReasonBadge reason={item.reason} />
          <span className="ml-auto whitespace-nowrap text-text-faint">
            {formatRelativeTime(item.updated_at, Date.now(), i18n.language)}
          </span>
        </div>
        <div
          className={
            "mt-1 truncate text-[13px] " +
            (item.unread ? "font-medium text-text" : "text-text-muted")
          }
        >
          {item.subject_title}
        </div>
      </button>

      {item.unread && (
        <button
          type="button"
          onClick={() => onMarkRead(item.id)}
          title={t("inbox_mark_read")}
          aria-label={t("inbox_mark_read")}
          className={
            "mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-[--radius] " +
            "text-text-faint opacity-0 transition-[opacity,color,background-color] duration-150 ease-out " +
            "hover:bg-surface-3 hover:text-text group-hover:opacity-100 focus-visible:opacity-100 " +
            "outline-none focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
          }
        >
          <Check size={15} strokeWidth={1.75} aria-hidden />
        </button>
      )}
    </div>
  );
}

function InboxSkeleton({ count = 8 }: { count?: number }) {
  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {Array.from({ length: count }, (_, i) => (
        <div
          key={i}
          className="flex gap-3 border-b border-border px-4 py-3 last:border-b-0"
        >
          <Skeleton className="size-4 rounded-full" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-3 w-1/3" />
            <Skeleton className="h-3 w-3/4" />
          </div>
        </div>
      ))}
    </div>
  );
}

export default function InboxView() {
  const { t } = useTranslation();
  const { accounts, activeAccountId } = useAccounts();
  const account =
    accounts.find((a) => a.id === activeAccountId)?.display_name ??
    activeAccountId ??
    null;

  const [filter, setFilter] = useState<Filter>("all");
  // Ids optimistically marked read locally — layered over the cached query data
  // so the row updates instantly while the write (and eventual refetch) runs.
  const [readIds, setReadIds] = useState<Set<string>>(new Set());

  const query = useNotifications(activeAccountId);
  const markReadMutation = useMarkNotificationRead(activeAccountId);

  const loading = query.isPending && query.fetchStatus !== "idle";
  const error = query.isError
    ? query.error instanceof Error
      ? query.error.message
      : String(query.error)
    : null;

  const items = useMemo<Notification[]>(() => {
    const base = Array.isArray(query.data) ? query.data : [];
    if (readIds.size === 0) return base;
    return base.map((n) => (readIds.has(n.id) ? { ...n, unread: false } : n));
  }, [query.data, readIds]);

  const markRead = useCallback(
    (id: string) => {
      if (!activeAccountId) return;
      setReadIds((prev) => new Set(prev).add(id));
      markReadMutation.mutate({ id: activeAccountId, threadId: id });
    },
    [activeAccountId, markReadMutation],
  );

  const markAllRead = useCallback(() => {
    if (!activeAccountId) return;
    const unread = items.filter((n) => n.unread);
    setReadIds((prev) => {
      const next = new Set(prev);
      for (const n of unread) next.add(n.id);
      return next;
    });
    for (const n of unread) {
      markReadMutation.mutate({ id: activeAccountId, threadId: n.id });
    }
  }, [activeAccountId, items, markReadMutation]);

  const unreadCount = useMemo(
    () => items.filter((n) => n.unread).length,
    [items],
  );

  const visible = useMemo(() => {
    const list = filter === "unread" ? items.filter((n) => n.unread) : items;
    // Unread first, then by updated_at desc.
    return [...list].sort((a, b) => {
      if (a.unread !== b.unread) return a.unread ? -1 : 1;
      return b.updated_at.localeCompare(a.updated_at);
    });
  }, [items, filter]);

  const FILTERS: Filter[] = ["all", "unread"];
  const focusRing =
    "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
    "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

  return (
    <SectionShell>
      <SectionHeader title={t("inbox_title")} account={account} />

      {accounts.length === 0 ? (
        <NoAccount />
      ) : loading ? (
        <InboxSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : (
        <>
          <div className="mb-4 flex flex-wrap items-center gap-2">
            <div
              role="group"
              aria-label={t("filter_state")}
              className="flex gap-0.5 rounded-[--radius] border border-border bg-surface p-0.5"
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
                  {f === "all" ? t("inbox_filter_all") : t("inbox_filter_unread")}
                </button>
              ))}
            </div>

            {unreadCount > 0 && (
              <span className="tnum text-[12px] text-text-faint">
                {t("inbox_unread_count", { count: unreadCount })}
              </span>
            )}

            <button
              type="button"
              onClick={markAllRead}
              disabled={unreadCount === 0}
              className={
                "ml-auto inline-flex items-center gap-1.5 rounded-[--radius] border border-border " +
                "px-2.5 py-1.5 text-[13px] text-text-muted " +
                "transition-[color,border-color,background-color] duration-150 ease-out " +
                "hover:border-border-strong hover:text-text " +
                "disabled:cursor-not-allowed disabled:opacity-50 " +
                focusRing
              }
            >
              <Check size={14} strokeWidth={1.75} aria-hidden />
              {t("inbox_mark_all_read")}
            </button>
          </div>

          {visible.length === 0 ? (
            <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
              <Inbox size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
              <p className="text-sm text-text-muted">{t("inbox_empty")}</p>
            </div>
          ) : (
            <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
              {visible.map((item) => (
                <NotificationRow
                  key={item.id}
                  item={item}
                  onMarkRead={markRead}
                />
              ))}
            </div>
          )}
        </>
      )}
    </SectionShell>
  );
}
