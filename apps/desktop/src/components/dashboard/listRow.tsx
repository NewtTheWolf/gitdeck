import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import { MessageSquare } from "lucide-react";
import type { Issue } from "../../lib/api";
import { formatRelativeTime, labelChipStyle } from "../../lib/format";
import { Avatar } from "../ui/Avatar";
import { Skeleton } from "../ui/Skeleton";

export const MAX_LABELS = 4;
export const MAX_ASSIGNEES = 3;

/**
 * Open a url in the system browser. `openUrl` returns a promise; failures
 * (invalid url) are swallowed so a click never crashes the view. Shared by the
 * dense rows and the card grid.
 */
export function openItem(url: string) {
  void openUrl(url).catch(() => {});
}

/** Small colored label chip: a dot in the GitHub color + a subtle tinted chip. */
export function LabelChip({ name, color }: { name: string; color: string }) {
  const style = labelChipStyle(color);
  return (
    <span
      className="inline-flex items-center gap-1 rounded-full border px-1.5 py-0.5 text-[11px] text-text-muted"
      style={{ background: style.background, borderColor: style.borderColor }}
    >
      <span
        aria-hidden
        className="size-1.5 shrink-0 rounded-full"
        style={{ background: style.dot }}
      />
      {name}
    </span>
  );
}

/**
 * One dense data row, shared by IssueList and PullRequestList. `kind` is the
 * leading state-dot + icon cluster; `extraMeta` lets PRs slot in a Draft badge.
 */
export function DataRow({
  item,
  kind,
  extraMeta,
}: {
  item: Issue;
  kind: ReactNode;
  extraMeta?: ReactNode;
}) {
  const { t, i18n } = useTranslation();
  const labels = item.labels || [];
  const assignees = item.assignees || [];
  const author = item.author_login || "unknown";

  return (
    <a
      href={item.html_url}
      onClick={(e) => {
        e.preventDefault();
        openItem(item.html_url);
      }}
      className={
        "group flex gap-3 border-b border-border bg-surface px-4 py-3 last:border-b-0 " +
        "transition-colors duration-150 ease-out hover:bg-surface-2 " +
        "outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent " +
        "motion-reduce:transition-none"
      }
    >
      <Avatar
        login={author}
        src={item.author_avatar_url}
        size={32}
        className="mt-0.5"
      />
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-x-2 gap-y-0.5 text-[12px]">
          {kind}
          <span className="font-medium text-text">{author}</span>
          <span className="font-mono text-text-muted">
            {item.repo_name_with_owner}
          </span>
          <span className="tnum font-mono text-text-faint">#{item.number}</span>
          <span className="text-text-faint">
            {formatRelativeTime(item.updated_at, Date.now(), i18n.language)}
          </span>
        </div>

        <div className="mt-1 truncate text-[13px] text-text">{item.title}</div>

        <div className="mt-1.5 flex flex-wrap items-center gap-1.5">
          {extraMeta}
          {labels.slice(0, MAX_LABELS).map((label) => (
            <LabelChip key={label.name} name={label.name} color={label.color} />
          ))}
          {labels.length > MAX_LABELS && (
            <span className="tnum text-[11px] text-text-faint">
              +{labels.length - MAX_LABELS}
            </span>
          )}

          <span className="ml-auto flex items-center gap-3">
            <span
              className="flex items-center gap-1 text-[12px] text-text-faint"
              title={t("comments_count", { count: item.comments_count })}
            >
              <MessageSquare size={13} strokeWidth={1.75} aria-hidden />
              <span className="tnum">{item.comments_count}</span>
            </span>
            {assignees.length > 0 && (
              <span className="flex -space-x-1.5">
                {assignees.slice(0, MAX_ASSIGNEES).map((login) => (
                  <Avatar
                    key={login}
                    login={login}
                    size={18}
                    className="ring-1 ring-surface"
                  />
                ))}
              </span>
            )}
          </span>
        </div>
      </div>
    </a>
  );
}

/** A leading dot in a semantic color (open/closed/merged). */
export function StateDot({ className }: { className: string }) {
  return (
    <span
      aria-hidden
      className={"size-2 shrink-0 rounded-full " + className}
    />
  );
}

export function ListSkeleton({ count = 8 }: { count?: number }) {
  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {Array.from({ length: count }, (_, i) => (
        <div
          key={i}
          className="flex gap-3 border-b border-border px-4 py-3 last:border-b-0"
        >
          <Skeleton className="size-8 rounded-full" />
          <div className="flex-1 space-y-2">
            <Skeleton className="h-3 w-1/2" />
            <Skeleton className="h-3 w-3/4" />
          </div>
        </div>
      ))}
    </div>
  );
}
