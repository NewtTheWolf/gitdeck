import {
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowLeft,
  Eye,
  ExternalLink,
  GitFork,
  Inbox,
  Lock,
  Star,
  CircleDot,
  Scale,
  GitBranch,
  Archive,
} from "lucide-react";
import {
  type CodeHit,
  type Contributor,
  type Fork,
  type Issue,
  type Language,
  type Referrer,
  type Release,
  type RepoDetail,
  type RepoPath,
  type Traffic,
  type WorkflowRun,
} from "../../lib/api";
import {
  useContributors,
  useForks,
  useLanguages,
  usePaths,
  useReferrers,
  useReleases,
  useRepoDetail,
  useSearchCode,
  useSearchIssues,
  useTrafficClones,
  useTrafficViews,
  useWorkflowRuns,
} from "../../lib/query/queries";
import {
  formatRelativeTime,
  formatStars,
  languageColor,
} from "../../lib/format";
import { useAccounts } from "../../contexts/AccountContext";
import { Avatar } from "../../components/ui/Avatar";
import { Badge } from "../../components/ui/Badge";
import { Skeleton } from "../../components/ui/Skeleton";
import {
  NoAccount,
  SectionError,
  SectionShell,
} from "../dashboard/sectionScaffold";
import { IssueList } from "../../components/dashboard/IssueList";
import { ciStatusOf } from "../ci/CIHealthView";

type TabKey =
  | "overview"
  | "releases"
  | "forks"
  | "contributors"
  | "actions"
  | "traffic"
  | "mentions";

const TABS: TabKey[] = [
  "overview",
  "releases",
  "forks",
  "contributors",
  "actions",
  "traffic",
  "mentions",
];

const TAB_LABEL: Record<TabKey, string> = {
  overview: "tab_overview",
  releases: "tab_releases",
  forks: "tab_forks",
  contributors: "tab_contributors",
  actions: "tab_actions",
  traffic: "tab_traffic",
  mentions: "tab_mentions",
};

function openExternal(url: string) {
  void openUrl(url).catch(() => {});
}

/** A generic async cell: loading skeleton → error → content, never crashes. */
interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

function EmptyState({ message }: { message: string }) {
  return (
    <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
      <Inbox
        size={20}
        strokeWidth={1.75}
        aria-hidden
        className="text-text-faint"
      />
      <p className="text-sm text-text-muted">{message}</p>
    </div>
  );
}

function ListSkeleton({ count = 5 }: { count?: number }) {
  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {Array.from({ length: count }, (_, i) => (
        <div
          key={i}
          className="flex items-center gap-3 border-b border-border px-4 py-3 last:border-b-0"
        >
          <Skeleton className="size-7 rounded-full" />
          <Skeleton className="h-3 w-1/3" />
          <Skeleton className="ml-auto h-3 w-12" />
        </div>
      ))}
    </div>
  );
}

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

/** A row link in the dense lists (releases / forks / contributors / actions). */
function RowLink({
  url,
  children,
}: {
  url: string;
  children: ReactNode;
}) {
  return (
    <a
      href={url}
      onClick={(e) => {
        e.preventDefault();
        openExternal(url);
      }}
      className={
        "flex items-center gap-3 border-b border-border bg-surface px-4 py-3 last:border-b-0 " +
        "transition-colors duration-150 ease-out hover:bg-surface-2 " +
        "outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-accent " +
        "motion-reduce:transition-none"
      }
    >
      {children}
    </a>
  );
}

// --- stat cluster -----------------------------------------------------------

function Stat({
  icon,
  value,
  label,
}: {
  icon: ReactNode;
  value: ReactNode;
  label: string;
}) {
  return (
    <span className="flex items-center gap-1.5 text-[13px] text-text-muted" title={label}>
      {icon}
      <span className="tnum text-text">{value}</span>
      <span className="text-text-faint">{label}</span>
    </span>
  );
}

// --- language breakdown bar -------------------------------------------------

function LanguageBar({ languages }: { languages: Language[] }) {
  const { t } = useTranslation();
  const total = languages.reduce((sum, l) => sum + Math.max(0, l.bytes), 0);
  if (total <= 0 || languages.length === 0) return null;

  // Legend caps at the top 8; the bar shows every segment.
  const withPct = languages.map((l) => ({
    ...l,
    pct: (l.bytes / total) * 100,
  }));
  const legend = withPct.slice(0, 8);

  return (
    <section className="flex flex-col gap-3">
      <h2 className="text-[13px] font-medium text-text-muted">
        {t("repo_languages")}
      </h2>
      <div
        className="flex h-2 w-full overflow-hidden rounded-full border border-border bg-surface-2"
        role="img"
        aria-label={t("repo_languages")}
      >
        {withPct.map((l) => (
          <span
            key={l.name}
            title={`${l.name} ${l.pct.toFixed(1)}%`}
            style={{ width: `${l.pct}%`, background: languageColor(l.name) }}
            className="h-full"
          />
        ))}
      </div>
      <div className="flex flex-wrap gap-x-4 gap-y-1.5">
        {legend.map((l) => (
          <span
            key={l.name}
            className="flex items-center gap-1.5 text-[12px] text-text-muted"
          >
            <span
              aria-hidden
              className="size-2.5 rounded-full"
              style={{ background: languageColor(l.name) }}
            />
            {l.name}
            <span className="tnum text-text-faint">{l.pct.toFixed(1)}%</span>
          </span>
        ))}
      </div>
    </section>
  );
}

// --- overview tab -----------------------------------------------------------

function MetaRow({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="flex flex-col gap-0.5">
      <dt className="text-[12px] text-text-faint">{label}</dt>
      <dd className="text-[13px] text-text">{children}</dd>
    </div>
  );
}

function OverviewTab({
  detail,
  languages,
}: {
  detail: RepoDetail;
  languages: AsyncState<Language[]>;
}) {
  const { t, i18n } = useTranslation();

  return (
    <div className="flex flex-col gap-6">
      <p className="text-sm leading-relaxed text-text-muted">
        {detail.description || t("repo_no_description")}
      </p>

      <dl className="grid grid-cols-2 gap-x-6 gap-y-4 sm:grid-cols-3">
        {detail.homepage && (
          <MetaRow label={t("repo_homepage")}>
            <a
              href={detail.homepage}
              onClick={(e) => {
                e.preventDefault();
                openExternal(detail.homepage as string);
              }}
              className={
                "truncate text-accent hover:underline " + focusRing
              }
            >
              {detail.homepage}
            </a>
          </MetaRow>
        )}
        <MetaRow label={t("repo_default_branch")}>
          <span className="inline-flex items-center gap-1 font-mono text-[12px]">
            <GitBranch size={13} strokeWidth={1.75} aria-hidden className="text-text-faint" />
            {detail.default_branch}
          </span>
        </MetaRow>
        {detail.license && (
          <MetaRow label={t("repo_license")}>
            <span className="inline-flex items-center gap-1">
              <Scale size={13} strokeWidth={1.75} aria-hidden className="text-text-faint" />
              {detail.license}
            </span>
          </MetaRow>
        )}
        {detail.created_at && (
          <MetaRow label={t("repo_created")}>
            {formatRelativeTime(detail.created_at, Date.now(), i18n.language)}
          </MetaRow>
        )}
        {detail.pushed_at && (
          <MetaRow label={t("repo_pushed")}>
            {formatRelativeTime(detail.pushed_at, Date.now(), i18n.language)}
          </MetaRow>
        )}
      </dl>

      {detail.topics.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 className="text-[13px] font-medium text-text-muted">
            {t("repo_topics")}
          </h2>
          <div className="flex flex-wrap gap-1.5">
            {detail.topics.map((topic) => (
              <span
                key={topic}
                className="rounded-full border border-border bg-surface-2 px-2.5 py-0.5 text-[12px] text-text-muted"
              >
                {topic}
              </span>
            ))}
          </div>
        </section>
      )}

      {languages.loading ? (
        <Skeleton className="h-2 w-full rounded-full" />
      ) : languages.error ? (
        <SectionError message={languages.error} />
      ) : (
        <LanguageBar languages={languages.data ?? []} />
      )}
    </div>
  );
}

// --- releases tab -----------------------------------------------------------

function ReleasesTab({ state }: { state: AsyncState<Release[]> }) {
  const { t, i18n } = useTranslation();
  if (state.loading) return <ListSkeleton />;
  if (state.error) return <SectionError message={state.error} />;
  const releases = state.data ?? [];
  if (releases.length === 0) return <EmptyState message={t("releases_empty")} />;

  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {releases.map((rel) => (
        <RowLink key={rel.id} url={rel.html_url}>
          <div className="min-w-0 flex-1">
            <div className="flex flex-wrap items-center gap-2">
              <span className="truncate text-[13px] font-medium text-text">
                {rel.name || rel.tag_name}
              </span>
              <span className="tnum font-mono text-[12px] text-text-faint">
                {rel.tag_name}
              </span>
              {rel.draft && <Badge>{t("release_draft")}</Badge>}
              {rel.prerelease && <Badge>{t("release_prerelease")}</Badge>}
            </div>
            <div className="mt-1 flex flex-wrap items-center gap-x-3 text-[12px] text-text-faint">
              {rel.author_login && (
                <span>{t("release_by", { author: rel.author_login })}</span>
              )}
              {rel.published_at && (
                <span>
                  {formatRelativeTime(
                    rel.published_at,
                    Date.now(),
                    i18n.language,
                  )}
                </span>
              )}
            </div>
          </div>
          <ExternalLink
            size={14}
            strokeWidth={1.75}
            aria-hidden
            className="shrink-0 text-text-faint"
          />
        </RowLink>
      ))}
    </div>
  );
}

// --- forks tab --------------------------------------------------------------

function ForksTab({ state }: { state: AsyncState<Fork[]> }) {
  const { t, i18n } = useTranslation();
  if (state.loading) return <ListSkeleton />;
  if (state.error) return <SectionError message={state.error} />;
  const forks = state.data ?? [];
  if (forks.length === 0) return <EmptyState message={t("forks_empty")} />;

  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {forks.map((fork) => (
        <RowLink key={fork.full_name} url={fork.html_url}>
          <span className="min-w-0 flex-1 truncate font-mono text-[13px] text-text">
            {fork.full_name}
          </span>
          <span className="flex items-center gap-1.5 text-[12px] text-text-faint">
            <Star size={13} strokeWidth={1.75} aria-hidden />
            <span className="tnum">{formatStars(fork.stars)}</span>
          </span>
          {fork.pushed_at && (
            <span className="whitespace-nowrap text-[12px] text-text-faint">
              {formatRelativeTime(fork.pushed_at, Date.now(), i18n.language)}
            </span>
          )}
        </RowLink>
      ))}
    </div>
  );
}

// --- contributors tab -------------------------------------------------------

function ContributorsTab({ state }: { state: AsyncState<Contributor[]> }) {
  const { t } = useTranslation();
  if (state.loading) return <ListSkeleton />;
  if (state.error) return <SectionError message={state.error} />;
  const contributors = state.data ?? [];
  if (contributors.length === 0)
    return <EmptyState message={t("contributors_empty")} />;

  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {contributors.map((c) => (
        <RowLink key={c.login} url={c.html_url}>
          <Avatar login={c.login} src={c.avatar_url} size={28} />
          <span className="min-w-0 flex-1 truncate text-[13px] text-text">
            {c.login}
          </span>
          <span className="tnum whitespace-nowrap text-[12px] text-text-faint">
            {t("contributions_count", { count: c.contributions })}
          </span>
        </RowLink>
      ))}
    </div>
  );
}

// --- actions tab ------------------------------------------------------------

const RUN_DOT: Record<string, string> = {
  passing: "bg-open",
  failing: "bg-closed",
  running: "bg-warn motion-safe:animate-pulse",
  none: "bg-text-faint",
  unknown: "bg-text-faint",
};

function ActionsTab({ state }: { state: AsyncState<WorkflowRun[]> }) {
  const { t, i18n } = useTranslation();
  if (state.loading) return <ListSkeleton />;
  if (state.error) return <SectionError message={state.error} />;
  const runs = state.data ?? [];
  if (runs.length === 0) return <EmptyState message={t("actions_empty")} />;

  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {runs.map((run) => {
        const status = ciStatusOf(run);
        return (
          <RowLink key={run.id} url={run.html_url}>
            <span
              aria-hidden
              className={"size-2.5 shrink-0 rounded-full " + RUN_DOT[status]}
            />
            <div className="min-w-0 flex-1">
              <span className="block truncate text-[13px] text-text">
                {run.name || t("tab_actions")}
              </span>
              <div className="mt-0.5 flex flex-wrap items-center gap-x-3 text-[12px] text-text-faint">
                {run.head_branch && (
                  <span className="truncate font-mono" title={run.head_branch}>
                    {run.head_branch}
                  </span>
                )}
                {run.event && <span>{run.event}</span>}
              </div>
            </div>
            <span className="whitespace-nowrap text-[12px] text-text-faint">
              {formatRelativeTime(run.updated_at, Date.now(), i18n.language)}
            </span>
          </RowLink>
        );
      })}
    </div>
  );
}

// --- traffic tab ------------------------------------------------------------

/**
 * A hairline bar sparkline of a daily traffic series. Hand-rolled SVG (no chart
 * library): each day is a thin accent bar scaled to the series max, with a faint
 * baseline. Static — no animation — so it's inherently motion-reduce safe.
 */
function Sparkline({
  days,
  label,
}: {
  days: { timestamp: string; count: number }[];
  label: string;
}) {
  const values = days.map((d) => Math.max(0, d.count));
  const max = Math.max(1, ...values);
  const W = 240;
  const H = 36;
  const n = Math.max(values.length, 1);
  const gap = 2;
  const barW = Math.max(1, (W - gap * (n - 1)) / n);

  return (
    <svg
      viewBox={`0 0 ${W} ${H}`}
      width="100%"
      height={H}
      preserveAspectRatio="none"
      role="img"
      aria-label={label}
      className="block text-accent"
    >
      <line
        x1={0}
        y1={H - 0.5}
        x2={W}
        y2={H - 0.5}
        stroke="currentColor"
        strokeOpacity={0.18}
        strokeWidth={1}
        vectorEffect="non-scaling-stroke"
      />
      {values.map((v, i) => {
        const h = max > 0 ? (v / max) * (H - 3) : 0;
        const x = i * (barW + gap);
        return (
          <rect
            key={i}
            x={x}
            y={H - h - 0.5}
            width={barW}
            height={Math.max(h, v > 0 ? 1 : 0)}
            rx={0.5}
            fill="currentColor"
            fillOpacity={0.85}
          />
        );
      })}
    </svg>
  );
}

function TrafficSummary({
  title,
  traffic,
}: {
  title: string;
  traffic: Traffic;
}) {
  const { t } = useTranslation();
  return (
    <section className="flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4">
      <div className="flex items-baseline justify-between gap-3">
        <h3 className="text-[13px] font-medium text-text">{title}</h3>
        <span className="text-[12px] text-text-faint">{t("last_14_days")}</span>
      </div>
      <div className="flex items-baseline gap-4">
        <span className="tnum text-[22px] font-semibold leading-none text-text">
          {formatStars(traffic.count)}
        </span>
        <span className="text-[12px] text-text-muted">
          <span className="tnum text-text">{formatStars(traffic.uniques)}</span>{" "}
          {t("traffic_uniques")}
        </span>
      </div>
      <Sparkline days={traffic.days} label={title} />
    </section>
  );
}

function CountTable<T>({
  title,
  rows,
  emptyMessage,
  keyOf,
  primary,
  secondary,
}: {
  title: string;
  rows: T[];
  emptyMessage: string;
  keyOf: (row: T) => string;
  primary: (row: T) => ReactNode;
  secondary?: (row: T) => ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-[13px] font-medium text-text-muted">{title}</h3>
      {rows.length === 0 ? (
        <EmptyState message={emptyMessage} />
      ) : (
        <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
          {rows.map((row) => (
            <div
              key={keyOf(row)}
              className="flex items-center gap-3 border-b border-border px-4 py-2.5 last:border-b-0"
            >
              <span className="min-w-0 flex-1 truncate text-[13px] text-text">
                {primary(row)}
              </span>
              {secondary && (
                <span className="tnum shrink-0 text-[12px] text-text-faint">
                  {secondary(row)}
                </span>
              )}
            </div>
          ))}
        </div>
      )}
    </section>
  );
}

interface TrafficState {
  views: Traffic | null;
  clones: Traffic | null;
  referrers: Referrer[];
  paths: RepoPath[];
}

function TrafficTab({ state }: { state: AsyncState<TrafficState> }) {
  const { t } = useTranslation();
  if (state.loading) return <ListSkeleton count={4} />;
  if (state.error) return <SectionError message={state.error} />;
  const data = state.data;
  if (!data) return <EmptyState message={t("traffic_empty")} />;

  // Both null ⇒ the account lacks push access. Render as a clear info state,
  // never as an error.
  if (data.views === null && data.clones === null) {
    return (
      <div className="flex items-start gap-3 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
        <Lock
          size={18}
          strokeWidth={1.75}
          aria-hidden
          className="mt-0.5 shrink-0 text-text-faint"
        />
        <p className="text-sm text-text-muted">{t("traffic_no_access")}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6">
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        {data.views && (
          <TrafficSummary title={t("traffic_views")} traffic={data.views} />
        )}
        {data.clones && (
          <TrafficSummary title={t("traffic_clones")} traffic={data.clones} />
        )}
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <CountTable
          title={t("traffic_referrers")}
          rows={data.referrers}
          emptyMessage={t("traffic_referrers_empty")}
          keyOf={(r) => r.referrer}
          primary={(r) => <span className="truncate">{r.referrer}</span>}
          secondary={(r) => (
            <>
              {formatStars(r.count)} · {formatStars(r.uniques)}
            </>
          )}
        />
        <CountTable
          title={t("traffic_paths")}
          rows={data.paths}
          emptyMessage={t("traffic_paths_empty")}
          keyOf={(p) => p.path}
          primary={(p) => (
            <span className="block min-w-0">
              <span className="block truncate text-text">{p.title || p.path}</span>
              <span className="block truncate font-mono text-[11px] text-text-faint">
                {p.path}
              </span>
            </span>
          )}
          secondary={(p) => (
            <>
              {formatStars(p.count)} · {formatStars(p.uniques)}
            </>
          )}
        />
      </div>
    </div>
  );
}

// --- mentions tab -----------------------------------------------------------

interface MentionsState {
  code: CodeHit[];
  issues: Issue[];
}

function CodeMentions({ hits }: { hits: CodeHit[] }) {
  const { t } = useTranslation();
  if (hits.length === 0) return <EmptyState message={t("mentions_code_empty")} />;
  return (
    <div className="overflow-hidden rounded-[--radius-lg] border border-border bg-surface">
      {hits.map((hit, i) => (
        <RowLink key={`${hit.html_url}-${i}`} url={hit.html_url}>
          <div className="min-w-0 flex-1">
            <span className="block truncate font-mono text-[12px] text-text-muted">
              {hit.repo_name_with_owner}
            </span>
            <span className="block truncate font-mono text-[13px] text-text">
              {hit.path}
            </span>
          </div>
          <ExternalLink
            size={14}
            strokeWidth={1.75}
            aria-hidden
            className="shrink-0 text-text-faint"
          />
        </RowLink>
      ))}
    </div>
  );
}

function MentionsTab({ state }: { state: AsyncState<MentionsState> }) {
  const { t } = useTranslation();
  if (state.loading)
    return (
      <div className="flex flex-col gap-6">
        <ListSkeleton count={3} />
        <ListSkeleton count={3} />
      </div>
    );
  if (state.error) return <SectionError message={state.error} />;
  const data = state.data ?? { code: [], issues: [] };

  return (
    <div className="flex flex-col gap-6">
      <section className="flex flex-col gap-2">
        <h3 className="text-[13px] font-medium text-text-muted">
          {t("mentions_code")}
        </h3>
        <CodeMentions hits={data.code} />
      </section>
      <section className="flex flex-col gap-2">
        <h3 className="text-[13px] font-medium text-text-muted">
          {t("mentions_issues")}
        </h3>
        {data.issues.length === 0 ? (
          <EmptyState message={t("mentions_issues_empty")} />
        ) : (
          <IssueList issues={data.issues} />
        )}
      </section>
    </div>
  );
}

// --- page -------------------------------------------------------------------

/** Map a single react-query result into the tabs' `AsyncState<T>` shape. */
function asAsync<T>(q: {
  data: T | null | undefined;
  isPending: boolean;
  fetchStatus: string;
  isError: boolean;
  error: unknown;
}): AsyncState<T> {
  return {
    data: (q.data ?? null) as T | null,
    loading: q.isPending && q.fetchStatus !== "idle",
    error: q.isError
      ? q.error instanceof Error
        ? q.error.message
        : String(q.error)
      : null,
  };
}

export default function RepositoryDetail() {
  const { t } = useTranslation();
  const params = useParams<{ owner: string; repo: string }>();
  const owner = params.owner ?? "";
  const repo = params.repo ?? "";
  const { accounts, activeAccountId } = useAccounts();

  const [tab, setTab] = useState<TabKey>("overview");

  // Overview (detail + languages) loads eagerly; every other tab's query is
  // gated on its tab being active (`enabled`), so it lazy-loads on first visit
  // and then stays cached — re-activating a tab paints instantly.
  const detailQuery = useRepoDetail(activeAccountId, owner, repo);
  const languagesQuery = useLanguages(activeAccountId, owner, repo);
  const releasesQuery = useReleases(activeAccountId, owner, repo, {
    enabled: tab === "releases",
  });
  const forksQuery = useForks(activeAccountId, owner, repo, {
    enabled: tab === "forks",
  });
  const contributorsQuery = useContributors(activeAccountId, owner, repo, {
    enabled: tab === "contributors",
  });
  const runsQuery = useWorkflowRuns(activeAccountId, owner, repo, 20, {
    enabled: tab === "actions",
  });

  const trafficEnabled = tab === "traffic";
  const viewsQuery = useTrafficViews(activeAccountId, owner, repo, {
    enabled: trafficEnabled,
  });
  const clonesQuery = useTrafficClones(activeAccountId, owner, repo, {
    enabled: trafficEnabled,
  });
  const referrersQuery = useReferrers(activeAccountId, owner, repo, {
    enabled: trafficEnabled,
  });
  const pathsQuery = usePaths(activeAccountId, owner, repo, {
    enabled: trafficEnabled,
  });

  const fullName = detailQuery.data?.full_name ?? `${owner}/${repo}`;
  // Quote the full_name so GitHub treats "owner/repo" as an exact phrase.
  const mentionsQuery = `"${fullName}"`;
  const mentionsEnabled = tab === "mentions";
  const searchCodeQuery = useSearchCode(activeAccountId, mentionsQuery, {
    enabled: mentionsEnabled,
    perPage: 20,
  });
  const searchIssuesQuery = useSearchIssues(activeAccountId, mentionsQuery, {
    enabled: mentionsEnabled,
    perPage: 20,
  });

  const detail = asAsync<RepoDetail>(detailQuery);
  const languages = asAsync<Language[]>(languagesQuery);
  const releases = asAsync<Release[]>(releasesQuery);
  const forks = asAsync<Fork[]>(forksQuery);
  const contributors = asAsync<Contributor[]>(contributorsQuery);
  const runs = asAsync<WorkflowRun[]>(runsQuery);

  const traffic: AsyncState<TrafficState> = useMemo(() => {
    const loading =
      [viewsQuery, clonesQuery, referrersQuery, pathsQuery].some(
        (q) => q.isPending && q.fetchStatus !== "idle",
      );
    const failed = [viewsQuery, clonesQuery, referrersQuery, pathsQuery].find(
      (q) => q.isError,
    );
    if (loading) return { data: null, loading: true, error: null };
    if (failed)
      return {
        data: null,
        loading: false,
        error:
          failed.error instanceof Error
            ? failed.error.message
            : String(failed.error),
      };
    if (!trafficEnabled) return { data: null, loading: false, error: null };
    return {
      data: {
        views: viewsQuery.data ?? null,
        clones: clonesQuery.data ?? null,
        referrers: referrersQuery.data ?? [],
        paths: pathsQuery.data ?? [],
      },
      loading: false,
      error: null,
    };
  }, [viewsQuery, clonesQuery, referrersQuery, pathsQuery, trafficEnabled]);

  const mentions: AsyncState<MentionsState> = useMemo(() => {
    const loading =
      [searchCodeQuery, searchIssuesQuery].some(
        (q) => q.isPending && q.fetchStatus !== "idle",
      );
    const failed = [searchCodeQuery, searchIssuesQuery].find((q) => q.isError);
    if (loading) return { data: null, loading: true, error: null };
    if (failed)
      return {
        data: null,
        loading: false,
        error:
          failed.error instanceof Error
            ? failed.error.message
            : String(failed.error),
      };
    if (!mentionsEnabled) return { data: null, loading: false, error: null };
    return {
      data: {
        code: searchCodeQuery.data ?? [],
        issues: searchIssuesQuery.data ?? [],
      },
      loading: false,
      error: null,
    };
  }, [searchCodeQuery, searchIssuesQuery, mentionsEnabled]);

  return (
    <SectionShell>
      <Link
        to="/repos"
        className={
          "mb-5 inline-flex items-center gap-1.5 text-[13px] text-text-muted " +
          "transition-colors duration-150 ease-out hover:text-text " +
          focusRing
        }
      >
        <ArrowLeft size={14} strokeWidth={1.75} aria-hidden />
        {t("repo_back")}
      </Link>

      {accounts.length === 0 ? (
        <NoAccount />
      ) : (
        <>
          <header className="mb-5 flex flex-col gap-3">
            <div className="flex flex-wrap items-center gap-2">
              <h1 className="font-mono text-[20px] font-semibold tracking-tight text-text">
                {fullName}
              </h1>
              {detail.data?.is_private && (
                <Badge icon={<Lock size={11} strokeWidth={1.75} aria-hidden />}>
                  {t("repo_private")}
                </Badge>
              )}
              {detail.data?.is_fork && (
                <Badge icon={<GitFork size={11} strokeWidth={1.75} aria-hidden />}>
                  {t("repo_fork")}
                </Badge>
              )}
              {detail.data?.is_archived && (
                <Badge icon={<Archive size={11} strokeWidth={1.75} aria-hidden />}>
                  {t("repo_archived")}
                </Badge>
              )}
              <a
                href={detail.data?.html_url ?? `https://github.com/${fullName}`}
                onClick={(e) => {
                  e.preventDefault();
                  openExternal(
                    detail.data?.html_url ?? `https://github.com/${fullName}`,
                  );
                }}
                title={t("repo_open_on_github")}
                aria-label={t("repo_open_on_github")}
                className={
                  "ml-auto inline-flex items-center gap-1.5 rounded-[--radius] border border-border " +
                  "bg-surface px-2.5 py-1 text-[13px] text-text-muted " +
                  "transition-colors duration-150 ease-out hover:border-border-strong hover:text-text " +
                  focusRing
                }
              >
                <ExternalLink size={13} strokeWidth={1.75} aria-hidden />
                {t("repo_open_on_github")}
              </a>
            </div>

            <div className="flex flex-wrap items-center gap-x-5 gap-y-2">
              <Stat
                icon={<Star size={14} strokeWidth={1.75} aria-hidden className="text-text-faint" />}
                value={formatStars(detail.data?.stars ?? 0)}
                label={t("repo_stars")}
              />
              <Stat
                icon={<GitFork size={14} strokeWidth={1.75} aria-hidden className="text-text-faint" />}
                value={formatStars(detail.data?.forks ?? 0)}
                label={t("repo_forks")}
              />
              <Stat
                icon={<CircleDot size={14} strokeWidth={1.75} aria-hidden className="text-text-faint" />}
                value={detail.data?.open_issues ?? 0}
                label={t("repo_open_issues_short")}
              />
              <Stat
                icon={<Eye size={14} strokeWidth={1.75} aria-hidden className="text-text-faint" />}
                value={formatStars(detail.data?.watchers ?? 0)}
                label={t("repo_watchers")}
              />
            </div>
          </header>

          <div
            role="tablist"
            aria-label={fullName}
            className="mb-5 flex gap-0.5 overflow-x-auto border-b border-border"
          >
            {TABS.map((key) => (
              <button
                key={key}
                type="button"
                role="tab"
                aria-selected={tab === key}
                onClick={() => setTab(key)}
                className={
                  "-mb-px whitespace-nowrap border-b-2 px-3 py-2 text-[13px] " +
                  "transition-[color,border-color] duration-150 ease-out " +
                  focusRing +
                  " " +
                  (tab === key
                    ? "border-accent font-medium text-text"
                    : "border-transparent text-text-muted hover:text-text")
                }
              >
                {t(TAB_LABEL[key])}
              </button>
            ))}
          </div>

          <div role="tabpanel">
            {detail.loading && tab === "overview" ? (
              <div className="flex flex-col gap-4">
                <Skeleton className="h-4 w-3/4" />
                <Skeleton className="h-4 w-1/2" />
                <Skeleton className="h-2 w-full rounded-full" />
              </div>
            ) : detail.error && tab === "overview" ? (
              <SectionError message={detail.error} />
            ) : tab === "overview" ? (
              detail.data && (
                <OverviewTab detail={detail.data} languages={languages} />
              )
            ) : tab === "releases" ? (
              <ReleasesTab state={releases} />
            ) : tab === "forks" ? (
              <ForksTab state={forks} />
            ) : tab === "contributors" ? (
              <ContributorsTab state={contributors} />
            ) : tab === "actions" ? (
              <ActionsTab state={runs} />
            ) : tab === "traffic" ? (
              <TrafficTab state={traffic} />
            ) : (
              <MentionsTab state={mentions} />
            )}
          </div>
        </>
      )}
    </SectionShell>
  );
}
