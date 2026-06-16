import type { Repo } from "./api";

/**
 * Insights — a client-side health overview of a set of repositories.
 *
 * We compute everything from the cheap repo *list* (no per-repo detail / traffic
 * fetches): each repo gets a coarse status (Strong / Watch / Risky), a set of
 * alerts (an i18n key + values, so the view stays translatable), and a few
 * opportunity hints.
 *
 * The only activity signal the list carries is `updated_at`, so that is our
 * activity timestamp. When the backend later exposes `pushed_at` it should be
 * preferred here.
 *
 * // TODO: traffic correlation — gitdeck cross-references views/clones to flag
 * "popular but neglected" more precisely; we approximate with stars for now and
 * skip per-repo traffic fetches in this MVP.
 */

// --- thresholds (documented constants) --------------------------------------

/** Activity older than this many days is a *warn*-level stale alert. */
export const STALE_WARN_DAYS = 60;
/** Activity older than this many days escalates the stale alert to *risk*. */
export const STALE_RISK_DAYS = 180;

/** Open-issue count at/above this is a *warn*-level "issues need attention". */
export const ISSUES_WARN_COUNT = 20;
/** Open-issue count at/above this escalates the issues alert to *risk*. */
export const ISSUES_RISK_COUNT = 50;

/** A repo this popular that has gone stale is flagged as an opportunity. */
export const POPULAR_STARS = 50;

const MS_PER_DAY = 86_400_000;

// --- types ------------------------------------------------------------------

export type InsightStatus = "strong" | "watch" | "risky";
export type AlertSeverity = "info" | "warn" | "risk";
export type AlertKind = "stale" | "issues";

export interface InsightAlert {
  kind: AlertKind;
  severity: AlertSeverity;
  /** i18n key the view translates (NOT a hardcoded English string). */
  messageKey: string;
  /** Interpolation values for the i18n string. */
  values?: Record<string, unknown>;
}

export interface RepoInsight {
  repo: Repo;
  status: InsightStatus;
  /** Whole days since the activity timestamp (clamped to >= 0). */
  daysSinceActivity: number;
  alerts: InsightAlert[];
  /** i18n keys for opportunity hints (no values needed today). */
  opportunities: string[];
}

export interface InsightsSummary {
  strong: number;
  watch: number;
  risky: number;
  total: number;
}

export interface InsightsResult {
  items: RepoInsight[];
  summary: InsightsSummary;
}

// --- helpers ----------------------------------------------------------------

/**
 * Whole days between an ISO activity timestamp and `now`. Invalid / missing
 * timestamps collapse to 0 (treated as "just active") so a bad date never makes
 * a repo look artificially stale. Future timestamps clamp to 0.
 */
export function daysSinceActivity(iso: string | null, now: number): number {
  if (!iso) return 0;
  const ms = new Date(iso).getTime();
  if (!Number.isFinite(ms)) return 0;
  const diff = now - ms;
  if (diff <= 0) return 0;
  return Math.floor(diff / MS_PER_DAY);
}

const SEVERITY_RANK: Record<AlertSeverity, number> = {
  info: 0,
  warn: 1,
  risk: 2,
};

/** The status implied by a repo's worst alert. */
function statusFromAlerts(alerts: InsightAlert[]): InsightStatus {
  let worst: AlertSeverity = "info";
  for (const a of alerts) {
    if (SEVERITY_RANK[a.severity] > SEVERITY_RANK[worst]) worst = a.severity;
  }
  if (worst === "risk") return "risky";
  if (worst === "warn") return "watch";
  return "strong";
}

const STATUS_RANK: Record<InsightStatus, number> = {
  risky: 0,
  watch: 1,
  strong: 2,
};

// --- per-repo computation ---------------------------------------------------

export function computeRepoInsight(repo: Repo, now: number): RepoInsight {
  const days = daysSinceActivity(repo.updated_at, now);
  const alerts: InsightAlert[] = [];
  const opportunities: string[] = [];

  // Stale: graded by how long since the last activity.
  if (days > STALE_RISK_DAYS) {
    alerts.push({
      kind: "stale",
      severity: "risk",
      messageKey: "insights_alert_stale",
      values: { days },
    });
  } else if (days > STALE_WARN_DAYS) {
    alerts.push({
      kind: "stale",
      severity: "warn",
      messageKey: "insights_alert_stale",
      values: { days },
    });
  }

  // Issues need attention: graded by open-issue volume.
  if (repo.open_issues >= ISSUES_RISK_COUNT) {
    alerts.push({
      kind: "issues",
      severity: "risk",
      messageKey: "insights_alert_issues",
      values: { count: repo.open_issues },
    });
  } else if (repo.open_issues >= ISSUES_WARN_COUNT) {
    alerts.push({
      kind: "issues",
      severity: "warn",
      messageKey: "insights_alert_issues",
      values: { count: repo.open_issues },
    });
  }

  // Opportunities — gentle nudges, not alerts.
  const isStale = days > STALE_WARN_DAYS;
  if (repo.stars >= POPULAR_STARS && isStale) {
    opportunities.push("insights_opp_popular_neglected");
  }

  return {
    repo,
    status: statusFromAlerts(alerts),
    daysSinceActivity: days,
    alerts,
    opportunities,
  };
}

// --- aggregate --------------------------------------------------------------

/**
 * Compute insights for a list of repos. Items are sorted risky → watch →
 * strong, then by stars descending (a stable tiebreak by full_name keeps the
 * order deterministic).
 */
export function computeInsights(repos: Repo[], now: number): InsightsResult {
  const items = (Array.isArray(repos) ? repos : []).map((r) =>
    computeRepoInsight(r, now),
  );

  items.sort((a, b) => {
    const byStatus = STATUS_RANK[a.status] - STATUS_RANK[b.status];
    if (byStatus !== 0) return byStatus;
    const byStars = b.repo.stars - a.repo.stars;
    if (byStars !== 0) return byStars;
    return a.repo.full_name.localeCompare(b.repo.full_name);
  });

  const summary: InsightsSummary = {
    strong: 0,
    watch: 0,
    risky: 0,
    total: items.length,
  };
  for (const item of items) summary[item.status] += 1;

  return { items, summary };
}
