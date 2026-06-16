import { describe, expect, it } from "vitest";
import type { Repo } from "./api";
import {
  computeInsights,
  computeRepoInsight,
  daysSinceActivity,
  ISSUES_RISK_COUNT,
  ISSUES_WARN_COUNT,
  POPULAR_STARS,
  STALE_RISK_DAYS,
  STALE_WARN_DAYS,
} from "./insights";

/** A fixed "now" so day math is deterministic. */
const NOW = Date.parse("2026-06-16T00:00:00Z");
const MS_PER_DAY = 86_400_000;

/** ISO timestamp `days` whole days before NOW. */
function isoDaysAgo(days: number): string {
  return new Date(NOW - days * MS_PER_DAY).toISOString();
}

function repo(over: Partial<Repo>): Repo {
  return {
    id: over.id ?? "r-1",
    full_name: over.full_name ?? "acme/web",
    description: null,
    html_url: "https://github.com/acme/web",
    stars: 0,
    open_issues: 0,
    language: null,
    is_private: false,
    is_fork: false,
    updated_at: isoDaysAgo(1),
    ...over,
  };
}

describe("daysSinceActivity", () => {
  it("computes whole days between an ISO timestamp and now", () => {
    expect(daysSinceActivity(isoDaysAgo(10), NOW)).toBe(10);
  });
  it("floors partial days", () => {
    expect(daysSinceActivity(new Date(NOW - 2.9 * MS_PER_DAY).toISOString(), NOW)).toBe(2);
  });
  it("clamps a future timestamp to 0", () => {
    expect(daysSinceActivity(isoDaysAgo(-5), NOW)).toBe(0);
  });
  it("treats null as 0 (just active)", () => {
    expect(daysSinceActivity(null, NOW)).toBe(0);
  });
  it("treats an invalid date as 0", () => {
    expect(daysSinceActivity("not-a-date", NOW)).toBe(0);
  });
});

describe("computeRepoInsight — stale thresholds", () => {
  it("is strong with no stale alert just under the warn boundary", () => {
    const r = computeRepoInsight(repo({ updated_at: isoDaysAgo(STALE_WARN_DAYS) }), NOW);
    expect(r.alerts.some((a) => a.kind === "stale")).toBe(false);
    expect(r.status).toBe("strong");
  });
  it("warns just over the stale warn boundary", () => {
    const r = computeRepoInsight(repo({ updated_at: isoDaysAgo(STALE_WARN_DAYS + 1) }), NOW);
    const stale = r.alerts.find((a) => a.kind === "stale");
    expect(stale?.severity).toBe("warn");
    expect(stale?.messageKey).toBe("insights_alert_stale");
    expect(stale?.values).toEqual({ days: STALE_WARN_DAYS + 1 });
    expect(r.status).toBe("watch");
  });
  it("does not warn exactly at the warn boundary (uses strict >)", () => {
    const r = computeRepoInsight(repo({ updated_at: isoDaysAgo(STALE_WARN_DAYS) }), NOW);
    expect(r.alerts.some((a) => a.kind === "stale")).toBe(false);
  });
  it("escalates to risk just over the stale risk boundary", () => {
    const r = computeRepoInsight(repo({ updated_at: isoDaysAgo(STALE_RISK_DAYS + 1) }), NOW);
    expect(r.alerts.find((a) => a.kind === "stale")?.severity).toBe("risk");
    expect(r.status).toBe("risky");
  });
});

describe("computeRepoInsight — issue thresholds", () => {
  it("has no issues alert below the warn count", () => {
    const r = computeRepoInsight(repo({ open_issues: ISSUES_WARN_COUNT - 1 }), NOW);
    expect(r.alerts.some((a) => a.kind === "issues")).toBe(false);
    expect(r.status).toBe("strong");
  });
  it("warns at exactly the warn count (uses >=)", () => {
    const r = computeRepoInsight(repo({ open_issues: ISSUES_WARN_COUNT }), NOW);
    const issues = r.alerts.find((a) => a.kind === "issues");
    expect(issues?.severity).toBe("warn");
    expect(issues?.messageKey).toBe("insights_alert_issues");
    expect(issues?.values).toEqual({ count: ISSUES_WARN_COUNT });
    expect(r.status).toBe("watch");
  });
  it("escalates to risk at exactly the risk count", () => {
    const r = computeRepoInsight(repo({ open_issues: ISSUES_RISK_COUNT }), NOW);
    expect(r.alerts.find((a) => a.kind === "issues")?.severity).toBe("risk");
    expect(r.status).toBe("risky");
  });
});

describe("computeRepoInsight — status derivation", () => {
  it("is strong with no alerts", () => {
    const r = computeRepoInsight(repo({ updated_at: isoDaysAgo(3), open_issues: 2 }), NOW);
    expect(r.status).toBe("strong");
    expect(r.alerts).toHaveLength(0);
  });
  it("a single risk alert wins over a warn alert", () => {
    // stale=warn but issues=risk → overall risky.
    const r = computeRepoInsight(
      repo({ updated_at: isoDaysAgo(STALE_WARN_DAYS + 5), open_issues: ISSUES_RISK_COUNT }),
      NOW,
    );
    expect(r.alerts).toHaveLength(2);
    expect(r.status).toBe("risky");
  });
});

describe("computeRepoInsight — opportunities", () => {
  it("flags popular-but-neglected when popular AND stale", () => {
    const r = computeRepoInsight(
      repo({ stars: POPULAR_STARS, updated_at: isoDaysAgo(STALE_WARN_DAYS + 1) }),
      NOW,
    );
    expect(r.opportunities).toContain("insights_opp_popular_neglected");
  });
  it("does not flag popular-but-neglected when popular but fresh", () => {
    const r = computeRepoInsight(repo({ stars: POPULAR_STARS, updated_at: isoDaysAgo(1) }), NOW);
    expect(r.opportunities).toHaveLength(0);
  });
  it("does not flag popular-but-neglected when stale but not popular", () => {
    const r = computeRepoInsight(
      repo({ stars: POPULAR_STARS - 1, updated_at: isoDaysAgo(STALE_WARN_DAYS + 1) }),
      NOW,
    );
    expect(r.opportunities).toHaveLength(0);
  });
});

describe("computeInsights — summary + sorting", () => {
  const repos: Repo[] = [
    repo({ id: "strong", full_name: "acme/strong", stars: 5, updated_at: isoDaysAgo(2) }),
    repo({
      id: "risky",
      full_name: "acme/risky",
      stars: 1,
      updated_at: isoDaysAgo(STALE_RISK_DAYS + 1),
    }),
    repo({
      id: "watch",
      full_name: "acme/watch",
      stars: 99,
      open_issues: ISSUES_WARN_COUNT,
      updated_at: isoDaysAgo(2),
    }),
  ];

  it("counts each status and the total", () => {
    const { summary } = computeInsights(repos, NOW);
    expect(summary).toEqual({ strong: 1, watch: 1, risky: 1, total: 3 });
  });

  it("sorts risky → watch → strong", () => {
    const { items } = computeInsights(repos, NOW);
    expect(items.map((i) => i.status)).toEqual(["risky", "watch", "strong"]);
  });

  it("within a status, sorts by stars descending", () => {
    const sameStatus: Repo[] = [
      repo({ id: "a", full_name: "acme/a", stars: 10, updated_at: isoDaysAgo(1) }),
      repo({ id: "b", full_name: "acme/b", stars: 200, updated_at: isoDaysAgo(1) }),
      repo({ id: "c", full_name: "acme/c", stars: 50, updated_at: isoDaysAgo(1) }),
    ];
    const { items } = computeInsights(sameStatus, NOW);
    expect(items.map((i) => i.repo.stars)).toEqual([200, 50, 10]);
  });

  it("handles an empty list", () => {
    const { items, summary } = computeInsights([], NOW);
    expect(items).toHaveLength(0);
    expect(summary).toEqual({ strong: 0, watch: 0, risky: 0, total: 0 });
  });

  it("tolerates a non-array input without throwing", () => {
    const { summary } = computeInsights(undefined as unknown as Repo[], NOW);
    expect(summary.total).toBe(0);
  });
});
