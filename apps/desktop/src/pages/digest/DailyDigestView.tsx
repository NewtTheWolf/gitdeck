import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { ArrowDown, ArrowUp, CalendarClock, ClipboardCopy, Check } from "lucide-react";
import { type Digest, type DigestEntry } from "../../lib/api";
import { useDigest } from "../../lib/query/queries";
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

/** A repo whose stars or open-issues changed since the previous captured day. */
function hasChange(e: DigestEntry): boolean {
  return e.stars_delta !== 0 || e.open_issues_delta !== 0;
}

/** Changed entries, sorted by combined absolute magnitude (desc). */
export function changedEntries(entries: DigestEntry[]): DigestEntry[] {
  return entries
    .filter(hasChange)
    .sort(
      (a, b) =>
        Math.abs(b.stars_delta) +
        Math.abs(b.open_issues_delta) -
        (Math.abs(a.stars_delta) + Math.abs(a.open_issues_delta)),
    );
}

export interface DigestSummary {
  starsGained: number;
  changedCount: number;
  biggest: DigestEntry | null;
}

export function summarize(entries: DigestEntry[]): DigestSummary {
  const changed = changedEntries(entries);
  const starsGained = entries.reduce(
    (sum, e) => sum + (e.stars_delta > 0 ? e.stars_delta : 0),
    0,
  );
  return {
    starsGained,
    changedCount: changed.length,
    biggest: changed[0] ?? null,
  };
}

/** Build a copy-paste Markdown digest from the changed entries. */
export function digestMarkdown(digest: Digest): string {
  const title = digest.previous_day
    ? `# Daily Digest — ${digest.day} vs ${digest.previous_day}`
    : `# Daily Digest — ${digest.day ?? ""}`;
  const lines = changedEntries(digest.entries).map((e) => {
    const parts: string[] = [];
    if (e.stars_delta !== 0) {
      parts.push(`${e.stars_delta > 0 ? "+" : ""}${e.stars_delta} stars`);
    }
    if (e.open_issues_delta !== 0) {
      parts.push(
        `${e.open_issues_delta > 0 ? "+" : ""}${e.open_issues_delta} open issues`,
      );
    }
    return `- ${e.repo_full_name}: ${parts.join(", ")}`;
  });
  return [title, "", ...(lines.length ? lines : ["- No changes since the previous day."])].join("\n");
}

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

/** A signed delta rendered with an arrow + tabular number; `goodWhenUp`
 * controls which direction is the "positive" (green) color. */
function DeltaNumber({
  delta,
  goodWhenUp,
}: {
  delta: number;
  goodWhenUp: boolean;
}) {
  if (delta === 0) {
    return <span className="tnum text-text-faint">0</span>;
  }
  const up = delta > 0;
  const good = up === goodWhenUp;
  const Arrow = up ? ArrowUp : ArrowDown;
  return (
    <span
      className={
        "inline-flex items-center gap-0.5 tnum font-medium " +
        (good ? "text-open" : "text-closed")
      }
    >
      <Arrow size={13} strokeWidth={2} aria-hidden />
      {Math.abs(delta)}
    </span>
  );
}

function DigestSkeleton() {
  return (
    <div className="flex flex-col gap-4">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        {Array.from({ length: 3 }, (_, i) => (
          <Skeleton key={i} className="h-16 rounded-[--radius-lg]" />
        ))}
      </div>
      <div className="flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-2">
        {Array.from({ length: 5 }, (_, i) => (
          <Skeleton key={i} className="h-9 w-full" />
        ))}
      </div>
    </div>
  );
}

function SummaryStat({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex flex-col gap-1 rounded-[--radius-lg] border border-border bg-surface px-4 py-3">
      <span className="text-[20px] font-semibold tracking-tight text-text tnum">
        {value}
      </span>
      <span className="text-[12px] text-text-muted">{label}</span>
    </div>
  );
}

export default function DailyDigestView() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { accounts, activeAccountId } = useAccounts();
  const account =
    accounts.find((a) => a.id === activeAccountId)?.display_name ??
    activeAccountId ??
    null;

  const [showAll, setShowAll] = useState(false);
  const [copied, setCopied] = useState(false);
  const activeRef = useRef(true);
  const copyTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const query = useDigest(activeAccountId);
  const digest: Digest | null = query.data ?? null;
  const loading = query.isPending && query.fetchStatus !== "idle";
  const error = query.isError
    ? query.error instanceof Error
      ? query.error.message
      : String(query.error)
    : null;

  useEffect(
    () => () => {
      if (copyTimer.current) clearTimeout(copyTimer.current);
    },
    [],
  );

  const summary = useMemo(
    () => (digest ? summarize(digest.entries) : null),
    [digest],
  );
  const changed = useMemo(
    () => (digest ? changedEntries(digest.entries) : []),
    [digest],
  );
  const rows = showAll ? (digest?.entries ?? []) : changed;

  function copyMarkdown() {
    if (!digest) return;
    const text = digestMarkdown(digest);
    const clip = navigator.clipboard;
    if (!clip || typeof clip.writeText !== "function") return;
    void clip.writeText(text).then(() => {
      if (!activeRef.current) return;
      setCopied(true);
      if (copyTimer.current) clearTimeout(copyTimer.current);
      copyTimer.current = setTimeout(() => setCopied(false), 1800);
    });
  }

  const isFirstRun = !!digest && digest.previous_day === null && digest.day !== null;
  const isEmpty = !!digest && digest.day === null;

  return (
    <SectionShell>
      <div className="mb-5 flex items-start justify-between gap-4">
        <div className="min-w-0">
          <SectionHeader title={t("digest_title")} account={account} />
          {digest && digest.day && (
            <p className="-mt-3 font-mono text-[13px] text-text-faint tnum">
              {digest.previous_day
                ? t("digest_range", {
                    day: digest.day,
                    previous: digest.previous_day,
                  })
                : t("digest_today", { day: digest.day })}
            </p>
          )}
        </div>
        {digest && digest.day && (
          <button
            type="button"
            onClick={copyMarkdown}
            className={
              "flex shrink-0 items-center gap-1.5 rounded-[--radius] border border-border " +
              "bg-surface px-3 py-1.5 text-[13px] text-text-muted " +
              "transition-[color,background-color,border-color] duration-150 ease-out " +
              "hover:border-border-strong hover:text-text " +
              focusRing
            }
          >
            {copied ? (
              <Check size={14} strokeWidth={2} aria-hidden className="text-open" />
            ) : (
              <ClipboardCopy size={14} strokeWidth={1.75} aria-hidden />
            )}
            {copied ? t("digest_copied") : t("digest_copy")}
          </button>
        )}
      </div>

      {accounts.length === 0 ? (
        <NoAccount />
      ) : loading ? (
        <DigestSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : isEmpty ? (
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <CalendarClock size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <p className="text-sm text-text-muted">{t("digest_no_repos")}</p>
        </div>
      ) : isFirstRun ? (
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-10">
          <CalendarClock size={20} strokeWidth={1.75} aria-hidden className="text-accent" />
          <p className="text-sm font-medium text-text">{t("digest_first_run_title")}</p>
          <p className="max-w-md text-[13px] leading-relaxed text-text-muted">
            {t("digest_first_run_body")}
          </p>
          {digest && digest.entries.length > 0 && (
            <dl className="mt-2 flex flex-wrap gap-x-6 gap-y-1 text-[13px] text-text-muted">
              <div className="flex items-center gap-1.5">
                <dt className="text-text-faint">{t("digest_col_stars")}</dt>
                <dd className="tnum text-text">
                  {formatStars(
                    digest.entries.reduce((s, e) => s + e.stars, 0),
                  )}
                </dd>
              </div>
              <div className="flex items-center gap-1.5">
                <dt className="text-text-faint">{t("digest_col_issues")}</dt>
                <dd className="tnum text-text">
                  {digest.entries.reduce((s, e) => s + e.open_issues, 0)}
                </dd>
              </div>
            </dl>
          )}
        </div>
      ) : (
        <>
          {summary && (
            <div className="mb-5 grid grid-cols-1 gap-3 sm:grid-cols-3">
              <SummaryStat
                label={t("digest_summary_stars_label")}
                value={`+${summary.starsGained}`}
              />
              <SummaryStat
                label={t("digest_summary_changed_label")}
                value={String(summary.changedCount)}
              />
              <div className="flex flex-col gap-1 rounded-[--radius-lg] border border-border bg-surface px-4 py-3">
                <span className="truncate font-mono text-[14px] font-medium text-text">
                  {summary.biggest ? summary.biggest.repo_full_name : "—"}
                </span>
                <span className="text-[12px] text-text-muted">
                  {summary.biggest
                    ? t("digest_summary_biggest_label")
                    : t("digest_summary_no_change")}
                </span>
              </div>
            </div>
          )}

          {changed.length === 0 ? (
            <p className="rounded-[--radius-lg] border border-border bg-surface px-4 py-6 text-sm text-text-muted">
              {t("digest_summary_no_change")}
            </p>
          ) : (
            <div className="overflow-hidden rounded-[--radius-lg] border border-border">
              <div className="grid grid-cols-[1fr_auto_auto] items-center gap-x-6 border-b border-border bg-surface-2 px-4 py-2 text-[11px] font-medium uppercase tracking-wider text-text-faint">
                <span>{t("digest_col_repo")}</span>
                <span className="text-right">{t("digest_col_stars")}</span>
                <span className="text-right">{t("digest_col_issues")}</span>
              </div>
              <ul>
                {rows.map((e) => (
                  <li
                    key={e.repo_full_name}
                    className="grid grid-cols-[1fr_auto_auto] items-center gap-x-6 border-b border-border bg-surface px-4 py-2.5 last:border-b-0"
                  >
                    <button
                      type="button"
                      onClick={() => navigate(repoDetailPath(e.repo_full_name))}
                      title={e.repo_full_name}
                      className={
                        "min-w-0 truncate text-left font-mono text-[13px] text-text " +
                        "transition-colors duration-150 ease-out hover:text-accent " +
                        focusRing +
                        " rounded-[--radius-sm]"
                      }
                    >
                      {e.repo_full_name}
                    </button>
                    <span className="flex items-center justify-end gap-1.5 text-[13px]">
                      <DeltaNumber delta={e.stars_delta} goodWhenUp={true} />
                      <span className="tnum text-text-faint">
                        ({formatStars(e.stars)})
                      </span>
                    </span>
                    <span className="flex items-center justify-end gap-1.5 text-[13px]">
                      <DeltaNumber delta={e.open_issues_delta} goodWhenUp={false} />
                      <span className="tnum text-text-faint">({e.open_issues})</span>
                    </span>
                  </li>
                ))}
              </ul>
            </div>
          )}

          {digest && digest.entries.length > changed.length && (
            <button
              type="button"
              onClick={() => setShowAll((v) => !v)}
              className={
                "mt-3 rounded-[--radius] px-2.5 py-1 text-[13px] text-text-muted " +
                "transition-colors duration-150 ease-out hover:text-text " +
                focusRing
              }
            >
              {showAll ? t("digest_show_changed") : t("digest_show_all")}
            </button>
          )}
        </>
      )}
    </SectionShell>
  );
}
