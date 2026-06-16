import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

const { captureSnapshots, dailyDigest } = vi.hoisted(() => ({
  captureSnapshots: vi.fn(),
  dailyDigest: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    captureSnapshots: (...a: unknown[]) => captureSnapshots(...a),
    dailyDigest: (...a: unknown[]) => dailyDigest(...a),
  },
}));

vi.mock("../../contexts/AccountContext", () => ({
  useAccounts: () => ({
    accounts: [{ id: "acc-1", display_name: "octo" }],
    activeAccountId: "acc-1",
    setActiveAccount: vi.fn(),
    refresh: vi.fn(),
  }),
}));

import "../../lib/i18n";
import DailyDigestView, { digestMarkdown, summarize } from "./DailyDigestView";

function entry(over: Record<string, unknown> = {}) {
  return {
    repo_full_name: "acme/x",
    stars_delta: 0,
    forks_delta: 0,
    open_issues_delta: 0,
    stars: 10,
    forks: 0,
    open_issues: 2,
    ...over,
  };
}

function renderView() {
  return renderWithClient(
    <MemoryRouter>
      <DailyDigestView />
    </MemoryRouter>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("digest helpers", () => {
  it("summarize totals positive star deltas and finds the biggest mover", () => {
    const s = summarize([
      entry({ repo_full_name: "a/one", stars_delta: 3 }),
      entry({ repo_full_name: "a/two", stars_delta: -1, open_issues_delta: 5 }),
      entry({ repo_full_name: "a/three" }),
    ]);
    expect(s.starsGained).toBe(3);
    expect(s.changedCount).toBe(2);
    expect(s.biggest?.repo_full_name).toBe("a/two");
  });

  it("digestMarkdown lists changed repos with signed deltas", () => {
    const md = digestMarkdown({
      day: "2026-06-16",
      previous_day: "2026-06-15",
      entries: [
        entry({ repo_full_name: "a/one", stars_delta: 4, open_issues_delta: 2 }),
        entry({ repo_full_name: "a/two" }),
      ],
    });
    expect(md).toContain("# Daily Digest — 2026-06-16 vs 2026-06-15");
    expect(md).toContain("- a/one: +4 stars, +2 open issues");
    expect(md).not.toContain("a/two");
  });
});

describe("DailyDigestView", () => {
  beforeEach(() => {
    captureSnapshots.mockReset().mockResolvedValue(1);
    dailyDigest.mockReset();
  });

  it("captures a snapshot on mount and renders the first-run info when there is no previous day", async () => {
    dailyDigest.mockResolvedValue({
      day: "2026-06-16",
      previous_day: null,
      entries: [entry()],
    });

    renderView();

    await waitFor(() =>
      expect(captureSnapshots).toHaveBeenCalledWith("acc-1"),
    );
    expect(await screen.findByText("Snapshot captured")).toBeInTheDocument();
    expect(
      screen.getByText(/compares against the previous day/i),
    ).toBeInTheDocument();
  });

  it("renders the summary and a changed-repo delta row for a two-day digest", async () => {
    dailyDigest.mockResolvedValue({
      day: "2026-06-16",
      previous_day: "2026-06-15",
      entries: [
        entry({
          repo_full_name: "acme/mover",
          stars_delta: 5,
          open_issues_delta: 2,
          stars: 42,
          open_issues: 7,
        }),
        entry({ repo_full_name: "acme/quiet" }),
      ],
    });

    renderView();

    // The mover appears both as the summary "biggest mover" and as a row.
    expect((await screen.findAllByText("acme/mover")).length).toBeGreaterThan(0);
    // Summary: +5 stars gained.
    expect(screen.getByText("+5")).toBeInTheDocument();
    // Unchanged repo is hidden by default.
    expect(screen.queryByText("acme/quiet")).not.toBeInTheDocument();
  });
});
