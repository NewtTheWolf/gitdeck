import type { ReactNode } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { ExternalLink, MessageSquare } from "lucide-react";
import type { Issue } from "../../lib/api";
import { formatRelativeTime } from "../../lib/format";
import { issueDetailPath, pullDetailPath } from "../../lib/itemRoute";
import { Avatar } from "../ui/Avatar";
import { Skeleton } from "../ui/Skeleton";
import {
  LabelChip,
  MAX_ASSIGNEES,
  MAX_LABELS,
  openItem,
} from "./listRow";

/**
 * One card in the issues / pull-requests grid, shared by both views (like
 * DataRow is). `kind` is the leading state-dot + icon cluster supplied by the
 * caller (issue vs PR coloring); `extraMeta` lets PRs slot in a Draft badge next
 * to the labels. Visually rhymes with RepoCard: hairline border, surface fill,
 * lift-on-hover, focus-visible ring. Clicking the card NAVIGATES to the in-app
 * detail view (issue vs PR per `kindKey`); a small icon escapes to GitHub.
 */
export function IssueCard({
  item,
  kind,
  kindKey,
  extraMeta,
}: {
  item: Issue;
  kind: ReactNode;
  /** Which detail route to open on click — "issue" or "pr". */
  kindKey: "issue" | "pr";
  extraMeta?: ReactNode;
}) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const labels = item.labels || [];
  const assignees = item.assignees || [];
  const author = item.author_login || "unknown";
  const detailPath =
    kindKey === "pr"
      ? pullDetailPath(item.repo_name_with_owner, item.number)
      : issueDetailPath(item.repo_name_with_owner, item.number);

  return (
    <a
      href={detailPath}
      onClick={(e) => {
        e.preventDefault();
        navigate(detailPath);
      }}
      className={
        "group flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4 " +
        "transition-[transform,color,background-color,border-color] duration-200 ease-out " +
        "hover:-translate-y-px hover:border-border-strong hover:bg-surface-2 " +
        "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
        "focus-visible:ring-offset-2 focus-visible:ring-offset-surface " +
        "motion-reduce:transform-none motion-reduce:transition-none"
      }
    >
      {/* Top row: state dot + repo + #number + relative time */}
      <div className="flex items-center gap-2 text-[12px]">
        {kind}
        <span
          className="min-w-0 truncate font-mono text-text-muted"
          title={item.repo_name_with_owner}
        >
          {item.repo_name_with_owner}
        </span>
        <span className="tnum shrink-0 font-mono text-text-faint">
          #{item.number}
        </span>
        <span className="ml-auto flex shrink-0 items-center gap-2 whitespace-nowrap text-text-faint">
          {formatRelativeTime(item.updated_at, Date.now(), i18n.language)}
          <button
            type="button"
            onClick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              openItem(item.html_url);
            }}
            title={t("detail_open_on_github")}
            aria-label={t("detail_open_on_github")}
            className="rounded-[--radius-sm] p-0.5 text-text-faint opacity-0 transition-opacity duration-150 hover:text-text group-hover:opacity-100 focus-visible:opacity-100 outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <ExternalLink size={12} strokeWidth={1.75} aria-hidden />
          </button>
        </span>
      </div>

      {/* Title — clamp to 2 lines */}
      <p className="line-clamp-2 min-h-[2.5rem] text-[13px] font-medium leading-relaxed text-text">
        {item.title}
      </p>

      {/* Labels (+ optional Draft badge) */}
      {(extraMeta || labels.length > 0) && (
        <div className="flex flex-wrap items-center gap-1.5">
          {extraMeta}
          {labels.slice(0, MAX_LABELS).map((label) => (
            <LabelChip key={label.name} name={label.name} color={label.color} />
          ))}
          {labels.length > MAX_LABELS && (
            <span className="tnum text-[11px] text-text-faint">
              +{labels.length - MAX_LABELS}
            </span>
          )}
        </div>
      )}

      {/* Footer: author on the left, comments + assignees on the right */}
      <div className="mt-auto flex items-center gap-2 text-[12px]">
        <Avatar login={author} src={item.author_avatar_url} size={20} />
        <span className="min-w-0 truncate font-medium text-text-muted">
          {author}
        </span>
        <span className="ml-auto flex shrink-0 items-center gap-3">
          <span
            className="flex items-center gap-1 text-text-faint"
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
    </a>
  );
}

function CardSkeletonItem() {
  return (
    <div className="flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4">
      <div className="flex items-center gap-2">
        <Skeleton className="h-3 w-1/3" />
        <Skeleton className="ml-auto h-3 w-10" />
      </div>
      <div className="flex flex-col gap-1.5">
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-3/4" />
      </div>
      <div className="mt-auto flex items-center gap-2">
        <Skeleton className="size-5 rounded-full" />
        <Skeleton className="h-3 w-20" />
        <Skeleton className="ml-auto h-3 w-8" />
      </div>
    </div>
  );
}

/** Grid-shaped loading placeholder matching the card grid layout. */
export function CardSkeleton({ count = 6 }: { count?: number }) {
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {Array.from({ length: count }, (_, i) => (
        <CardSkeletonItem key={i} />
      ))}
    </div>
  );
}

export default IssueCard;
