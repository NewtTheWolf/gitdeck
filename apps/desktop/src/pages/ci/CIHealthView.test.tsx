import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { renderWithClient } from "../../lib/test/renderWithClient";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const { listRepos, listWorkflowRuns } = vi.hoisted(() => ({
  listRepos: vi.fn(),
  listWorkflowRuns: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    listRepos: (...a: unknown[]) => listRepos(...a),
    listWorkflowRuns: (...a: unknown[]) => listWorkflowRuns(...a),
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
import CIHealthView, { ciStatusOf } from "./CIHealthView";

function repo(id: string, fullName: string, updatedAt: string) {
  return {
    id,
    full_name: fullName,
    description: null,
    html_url: `https://github.com/${fullName}`,
    stars: 0,
    open_issues: 0,
    language: null,
    is_private: false,
    is_fork: false,
    updated_at: updatedAt,
  };
}

function run(conclusion: string | null, status = "completed") {
  return {
    id: 1,
    name: "CI",
    head_branch: "main",
    status,
    conclusion,
    html_url: "https://github.com/acme/x/actions/runs/1",
    created_at: "2026-06-15T00:00:00Z",
    updated_at: "2026-06-15T00:00:00Z",
    event: "push",
  };
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("ciStatusOf", () => {
  it("maps completed/success to passing", () => {
    expect(ciStatusOf(run("success"))).toBe("passing");
  });
  it("maps completed/failure to failing", () => {
    expect(ciStatusOf(run("failure"))).toBe("failing");
  });
  it("maps in_progress to running", () => {
    expect(ciStatusOf(run(null, "in_progress"))).toBe("running");
  });
  it("maps null run to none", () => {
    expect(ciStatusOf(null)).toBe("none");
  });
});

describe("CIHealthView", () => {
  beforeEach(() => {
    listRepos.mockReset();
    listWorkflowRuns.mockReset();
  });

  it("renders repo cards with a passing and a failing summary", async () => {
    listRepos.mockResolvedValue([
      repo("r-1", "acme/passing", "2026-06-15T00:00:00Z"),
      repo("r-2", "acme/failing", "2026-06-14T00:00:00Z"),
    ]);
    listWorkflowRuns.mockImplementation((params: { owner: string; repo: string }) => {
      if (params.repo === "passing") return Promise.resolve([run("success")]);
      return Promise.resolve([run("failure")]);
    });

    renderWithClient(<CIHealthView />);

    expect(await screen.findByText("acme/passing")).toBeInTheDocument();
    expect(screen.getByText("acme/failing")).toBeInTheDocument();

    // Summary counts: 1 passing, 1 failing. Both labels appear (summary + card).
    await waitFor(() => {
      expect(screen.getAllByText("Passing").length).toBeGreaterThan(0);
      expect(screen.getAllByText("Failing").length).toBeGreaterThan(0);
    });
  });

  it("renders a no-runs repo as resilient state and a failing call as unknown", async () => {
    listRepos.mockResolvedValue([
      repo("r-1", "acme/empty", "2026-06-15T00:00:00Z"),
      repo("r-2", "acme/broken", "2026-06-14T00:00:00Z"),
    ]);
    listWorkflowRuns.mockImplementation((params: { repo: string }) => {
      if (params.repo === "empty") return Promise.resolve([]);
      return Promise.reject(new Error("boom"));
    });

    renderWithClient(<CIHealthView />);
    expect(await screen.findByText("acme/empty")).toBeInTheDocument();
    expect(screen.getByText("acme/broken")).toBeInTheDocument();
    // The view did not crash; both cards rendered.
  });
});
