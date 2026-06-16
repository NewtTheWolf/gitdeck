import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  Star,
  CircleDot,
  Lock,
  GitFork,
  ExternalLink,
} from "lucide-react";
import type { Repo } from "../../lib/api";
import { formatStars, languageColor } from "../../lib/format";
import { repoDetailPath } from "../../lib/repoRoute";
import { Badge } from "../ui/Badge";
import { Skeleton } from "../ui/Skeleton";

function openRepo(url: string) {
  // openUrl returns a promise; failures (e.g. invalid url) are swallowed so a
  // click never crashes the view.
  void openUrl(url).catch(() => {});
}

function RepoCard({ repo }: { repo: Repo }) {
  const { t } = useTranslation();
  const navigate = useNavigate();

  return (
    <article
      className={
        "group flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4 " +
        "transition-[transform,color,background-color,border-color] duration-200 ease-out " +
        "hover:-translate-y-px hover:border-border-strong hover:bg-surface-2 motion-reduce:transform-none motion-reduce:transition-none"
      }
    >
      <div className="flex items-start justify-between gap-2">
        <button
          type="button"
          onClick={() => navigate(repoDetailPath(repo.full_name))}
          title={repo.full_name}
          className={
            "min-w-0 truncate text-left font-mono text-[13px] text-text " +
            "transition-colors duration-150 ease-out hover:text-accent " +
            "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
            "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none rounded-[--radius-sm]"
          }
        >
          {repo.full_name}
        </button>
        <div className="flex shrink-0 items-center gap-1.5">
          {repo.is_private && (
            <Badge icon={<Lock size={11} strokeWidth={1.75} aria-hidden />}>
              {t("repo_private")}
            </Badge>
          )}
          {repo.is_fork && (
            <Badge icon={<GitFork size={11} strokeWidth={1.75} aria-hidden />}>
              {t("repo_fork")}
            </Badge>
          )}
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
      </div>

      <p className="line-clamp-2 min-h-[2.5rem] text-xs leading-relaxed text-text-muted">
        {repo.description || ""}
      </p>

      <div className="mt-auto flex flex-wrap items-center gap-x-4 gap-y-1.5 text-xs text-text-muted">
        {repo.language && (
          <span className="flex items-center gap-1.5">
            <span
              aria-hidden
              className="size-2.5 rounded-full"
              style={{ background: languageColor(repo.language) }}
            />
            {repo.language}
          </span>
        )}
        <span className="flex items-center gap-1.5" title={String(repo.stars)}>
          <Star
            size={14}
            strokeWidth={1.75}
            aria-hidden
            className="text-text-faint"
          />
          <span className="tnum">{formatStars(repo.stars)}</span>
        </span>
        <span
          className="flex items-center gap-1.5"
          title={t("repo_open_issues", { count: repo.open_issues })}
        >
          <CircleDot
            size={14}
            strokeWidth={1.75}
            aria-hidden
            className="text-text-faint"
          />
          <span className="tnum">{repo.open_issues}</span>
        </span>
      </div>
    </article>
  );
}

function RepoCardSkeleton() {
  return (
    <div className="flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4">
      <Skeleton className="h-4 w-2/3" />
      <div className="flex flex-col gap-1.5">
        <Skeleton className="h-3 w-full" />
        <Skeleton className="h-3 w-4/5" />
      </div>
      <div className="mt-auto flex items-center gap-4">
        <Skeleton className="h-3 w-16" />
        <Skeleton className="h-3 w-10" />
        <Skeleton className="h-3 w-10" />
      </div>
    </div>
  );
}

export function RepoGridSkeleton({ count = 6 }: { count?: number }) {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {Array.from({ length: count }, (_, i) => (
        <RepoCardSkeleton key={i} />
      ))}
    </div>
  );
}

export default function RepoGrid({ repos }: { repos: Repo[] }) {
  return (
    <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-3">
      {repos.map((repo) => (
        <RepoCard key={repo.id} repo={repo} />
      ))}
    </div>
  );
}
