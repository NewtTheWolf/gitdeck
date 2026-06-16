import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter, Routes, Route } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const {
  getRepoDetail,
  getLanguages,
  listReleases,
  listForks,
  listContributors,
  listWorkflowRuns,
  getTrafficViews,
  getTrafficClones,
  listReferrers,
  listPaths,
  searchCode,
  searchIssues,
} = vi.hoisted(() => ({
  getRepoDetail: vi.fn(),
  getLanguages: vi.fn(),
  listReleases: vi.fn(),
  listForks: vi.fn(),
  listContributors: vi.fn(),
  listWorkflowRuns: vi.fn(),
  getTrafficViews: vi.fn(),
  getTrafficClones: vi.fn(),
  listReferrers: vi.fn(),
  listPaths: vi.fn(),
  searchCode: vi.fn(),
  searchIssues: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    getRepoDetail: (...a: unknown[]) => getRepoDetail(...a),
    getLanguages: (...a: unknown[]) => getLanguages(...a),
    listReleases: (...a: unknown[]) => listReleases(...a),
    listForks: (...a: unknown[]) => listForks(...a),
    listContributors: (...a: unknown[]) => listContributors(...a),
    listWorkflowRuns: (...a: unknown[]) => listWorkflowRuns(...a),
    getTrafficViews: (...a: unknown[]) => getTrafficViews(...a),
    getTrafficClones: (...a: unknown[]) => getTrafficClones(...a),
    listReferrers: (...a: unknown[]) => listReferrers(...a),
    listPaths: (...a: unknown[]) => listPaths(...a),
    searchCode: (...a: unknown[]) => searchCode(...a),
    searchIssues: (...a: unknown[]) => searchIssues(...a),
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
import RepositoryDetail from "./RepositoryDetail";

const DETAIL = {
  full_name: "acme/widget",
  description: "A widget",
  html_url: "https://github.com/acme/widget",
  homepage: null,
  language: "TypeScript",
  stars: 1234,
  forks: 56,
  open_issues: 7,
  watchers: 89,
  default_branch: "main",
  license: "MIT",
  topics: ["cli", "tools"],
  owner_login: "acme",
  owner_avatar_url: null,
  is_private: false,
  is_fork: false,
  is_archived: false,
  size: 1000,
  pushed_at: "2026-06-15T00:00:00Z",
  created_at: "2025-06-15T00:00:00Z",
  updated_at: "2026-06-15T00:00:00Z",
};

function renderDetail() {
  return renderWithClient(
    <MemoryRouter initialEntries={["/repos/acme/widget"]}>
      <Routes>
        <Route path="/repos/:owner/:repo" element={<RepositoryDetail />} />
      </Routes>
    </MemoryRouter>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("RepositoryDetail", () => {
  beforeEach(() => {
    getRepoDetail.mockReset().mockResolvedValue(DETAIL);
    getLanguages
      .mockReset()
      .mockResolvedValue([
        { name: "TypeScript", bytes: 8000 },
        { name: "CSS", bytes: 2000 },
      ]);
    listReleases.mockReset().mockResolvedValue([]);
    listForks.mockReset().mockResolvedValue([]);
    listContributors.mockReset().mockResolvedValue([]);
    listWorkflowRuns.mockReset().mockResolvedValue([]);
    getTrafficViews.mockReset().mockResolvedValue(null);
    getTrafficClones.mockReset().mockResolvedValue(null);
    listReferrers.mockReset().mockResolvedValue([]);
    listPaths.mockReset().mockResolvedValue([]);
    searchCode.mockReset().mockResolvedValue([]);
    searchIssues.mockReset().mockResolvedValue([]);
  });

  it("renders header stats and a language segment", async () => {
    renderDetail();

    // Header repo name + a stat value (stars formatted as 1.2k). The name shows
    // immediately (route coords); the stats appear once the detail query lands.
    expect(await screen.findByText("acme/widget")).toBeInTheDocument();
    expect(await screen.findByText("1.2k")).toBeInTheDocument();

    // Language breakdown: legend shows TypeScript with its share.
    await waitFor(() => {
      expect(screen.getByText("80.0%")).toBeInTheDocument();
    });
    // Releases should NOT have been fetched yet (lazy).
    expect(listReleases).not.toHaveBeenCalled();
  });

  it("lazy-loads releases when the Releases tab is activated", async () => {
    listReleases.mockResolvedValue([
      {
        id: 1,
        tag_name: "v1.0.0",
        name: "First release",
        html_url: "https://github.com/acme/widget/releases/v1.0.0",
        body: null,
        draft: false,
        prerelease: false,
        published_at: "2026-06-10T00:00:00Z",
        author_login: "octo",
      },
    ]);

    renderDetail();
    await screen.findByText("acme/widget");

    fireEvent.click(screen.getByRole("tab", { name: "Releases" }));

    expect(await screen.findByText("First release")).toBeInTheDocument();
    expect(listReleases).toHaveBeenCalledTimes(1);
  });

  it("shows the no-access info when traffic views and clones are both null", async () => {
    renderDetail();
    await screen.findByText("acme/widget");

    fireEvent.click(screen.getByRole("tab", { name: "Traffic" }));

    expect(
      await screen.findByText(
        "Traffic data requires push access to this repository.",
      ),
    ).toBeInTheDocument();
    expect(getTrafficViews).toHaveBeenCalledTimes(1);
    expect(getTrafficClones).toHaveBeenCalledTimes(1);
  });

  it("loads code + issue mentions when the Mentions tab is activated", async () => {
    searchCode.mockResolvedValue([
      {
        repo_name_with_owner: "other/project",
        path: "docs/links.md",
        html_url: "https://github.com/other/project/blob/main/docs/links.md",
        name: "links.md",
      },
    ]);
    searchIssues.mockResolvedValue([
      {
        number: 42,
        title: "Depends on acme/widget",
        html_url: "https://github.com/other/project/issues/42",
        state: "open",
        author_login: "octo",
        author_avatar_url: null,
        repo_name_with_owner: "other/project",
        created_at: "2026-06-10T00:00:00Z",
        updated_at: "2026-06-12T00:00:00Z",
        comments_count: 2,
        labels: [],
        assignees: [],
      },
    ]);

    renderDetail();
    await screen.findByText("acme/widget");

    fireEvent.click(screen.getByRole("tab", { name: "Mentions" }));

    expect(await screen.findByText("docs/links.md")).toBeInTheDocument();
    expect(screen.getByText("Depends on acme/widget")).toBeInTheDocument();
    expect(searchCode).toHaveBeenCalledTimes(1);
    expect(searchIssues).toHaveBeenCalledTimes(1);
    // The query quotes the repo full name as an exact phrase.
    expect(searchCode).toHaveBeenCalledWith(
      expect.objectContaining({ query: '"acme/widget"' }),
    );
  });
});
