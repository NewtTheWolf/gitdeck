import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const { listRepos } = vi.hoisted(() => ({ listRepos: vi.fn() }));
vi.mock("../../lib/api", () => ({
  api: { listRepos: (...a: unknown[]) => listRepos(...a) },
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
import InsightsView from "./InsightsView";

const MS_PER_DAY = 86_400_000;
function isoDaysAgo(days: number): string {
  return new Date(Date.now() - days * MS_PER_DAY).toISOString();
}

function repo(over: Record<string, unknown>) {
  return {
    id: "r",
    full_name: "acme/x",
    description: null,
    html_url: "https://github.com/acme/x",
    stars: 0,
    open_issues: 0,
    language: null,
    is_private: false,
    is_fork: false,
    updated_at: isoDaysAgo(1),
    ...over,
  };
}

function renderView() {
  return renderWithClient(
    <MemoryRouter>
      <InsightsView />
    </MemoryRouter>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("InsightsView", () => {
  beforeEach(() => listRepos.mockReset());

  it("renders a strong and a risky repo with summary counts and an alert badge", async () => {
    listRepos.mockResolvedValue([
      repo({ id: "strong", full_name: "acme/strong", updated_at: isoDaysAgo(2) }),
      repo({
        id: "risky",
        full_name: "acme/risky",
        updated_at: isoDaysAgo(300),
        open_issues: 60,
      }),
    ]);

    const { container } = renderView();

    // Both repo cards render (risky sorts first).
    expect(await screen.findByText("acme/risky")).toBeInTheDocument();
    expect(screen.getByText("acme/strong")).toBeInTheDocument();

    // Summary: 1 strong, 1 risky. Status labels appear (summary pill + card).
    await waitFor(() => {
      expect(screen.getAllByText("Strong").length).toBeGreaterThan(0);
      expect(screen.getAllByText("Risky").length).toBeGreaterThan(0);
    });

    // A status dot is rendered (semantic color class on a presentational span).
    expect(container.querySelector(".bg-closed")).toBeTruthy();

    // The risky repo's issues alert badge is translated from its messageKey.
    expect(await screen.findByText("60 issues need attention")).toBeInTheDocument();
    // And the stale alert badge.
    expect(screen.getByText("No push for 300 days")).toBeInTheDocument();
  });

  it("shows the empty state when there are no repos", async () => {
    listRepos.mockResolvedValue([]);
    renderView();
    expect(
      await screen.findByText("No repositories found for this account."),
    ).toBeInTheDocument();
  });
});
