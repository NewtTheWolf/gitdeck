import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter, Routes, Route } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const {
  getIssue,
  getPullRequest,
  listIssueComments,
  listPullReviews,
  setIssueState,
  addIssueLabels,
  removeIssueLabel,
  addIssueAssignees,
  createIssueComment,
} = vi.hoisted(() => ({
  getIssue: vi.fn(),
  getPullRequest: vi.fn(),
  listIssueComments: vi.fn(),
  listPullReviews: vi.fn(),
  setIssueState: vi.fn(),
  addIssueLabels: vi.fn(),
  removeIssueLabel: vi.fn(),
  addIssueAssignees: vi.fn(),
  createIssueComment: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    getIssue: (...a: unknown[]) => getIssue(...a),
    getPullRequest: (...a: unknown[]) => getPullRequest(...a),
    listIssueComments: (...a: unknown[]) => listIssueComments(...a),
    listPullReviews: (...a: unknown[]) => listPullReviews(...a),
    setIssueState: (...a: unknown[]) => setIssueState(...a),
    addIssueLabels: (...a: unknown[]) => addIssueLabels(...a),
    removeIssueLabel: (...a: unknown[]) => removeIssueLabel(...a),
    addIssueAssignees: (...a: unknown[]) => addIssueAssignees(...a),
    createIssueComment: (...a: unknown[]) => createIssueComment(...a),
  },
}));

vi.mock("../../contexts/AccountContext", () => ({
  useAccounts: () => ({
    accounts: [{ id: "acc-1", display_name: "octocat" }],
    activeAccountId: "acc-1",
    setActiveAccount: vi.fn(),
    refresh: vi.fn(),
  }),
}));

import "../../lib/i18n";
import { IssueDetailView, PullRequestDetailView } from "./DetailView";

const ISSUE = {
  number: 42,
  title: "The widget is broken",
  body: "Steps to **reproduce**:\n\n- open it",
  html_url: "https://github.com/acme/widget/issues/42",
  state: "open",
  author_login: "alice",
  author_avatar_url: null,
  repo_name_with_owner: "acme/widget",
  created_at: "2026-06-10T00:00:00Z",
  updated_at: "2026-06-15T00:00:00Z",
  closed_at: null,
  comments_count: 1,
  labels: [{ name: "bug", color: "d73a4a" }],
  assignees: [{ login: "bob", avatar_url: null }],
  milestone_title: null,
};

const PR = {
  ...ISSUE,
  number: 7,
  title: "Add a fix",
  html_url: "https://github.com/acme/widget/pull/7",
  is_draft: false,
  merged: false,
  mergeable_state: "clean",
  base_ref: "main",
  head_ref: "fix-branch",
  additions: 10,
  deletions: 2,
  changed_files: 3,
  commits: 1,
  // The active account (octocat) is among the requested reviewers.
  requested_reviewers: [{ login: "octocat", avatar_url: null }],
  requested_teams: [],
};

function renderIssue() {
  return renderWithClient(
    <MemoryRouter initialEntries={["/repos/acme/widget/issues/42"]}>
      <Routes>
        <Route
          path="/repos/:owner/:repo/issues/:number"
          element={<IssueDetailView />}
        />
      </Routes>
    </MemoryRouter>,
  );
}

function renderPr() {
  return renderWithClient(
    <MemoryRouter initialEntries={["/repos/acme/widget/pull/7"]}>
      <Routes>
        <Route
          path="/repos/:owner/:repo/pull/:number"
          element={<PullRequestDetailView />}
        />
      </Routes>
    </MemoryRouter>,
  );
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("IssueDetailView", () => {
  beforeEach(() => {
    getIssue.mockReset().mockResolvedValue(ISSUE);
    listIssueComments.mockReset().mockResolvedValue([
      {
        id: 1,
        author_login: "carol",
        author_avatar_url: null,
        body: "I can confirm this bug.",
        created_at: "2026-06-12T00:00:00Z",
        updated_at: "2026-06-12T00:00:00Z",
        html_url: "https://github.com/acme/widget/issues/42#c1",
      },
    ]);
  });

  it("renders title, an assignee, a label and the comment body", async () => {
    renderIssue();
    expect(await screen.findByText("The widget is broken")).toBeInTheDocument();
    // Assignee login appears in the aside.
    expect(await screen.findByText("bob")).toBeInTheDocument();
    // Label chip.
    expect(screen.getByText("bug")).toBeInTheDocument();
    // Comment body (markdown-rendered).
    expect(
      await screen.findByText("I can confirm this bug."),
    ).toBeInTheDocument();
  });
});

describe("PullRequestDetailView", () => {
  beforeEach(() => {
    getPullRequest.mockReset().mockResolvedValue(PR);
    listIssueComments.mockReset().mockResolvedValue([]);
    listPullReviews.mockReset().mockResolvedValue([
      {
        id: 1,
        reviewer_login: "dave",
        reviewer_avatar_url: null,
        state: "APPROVED",
        body: null,
        submitted_at: "2026-06-13T00:00:00Z",
        html_url: "https://github.com/acme/widget/pull/7#r1",
      },
    ]);
  });

  it("renders 'review requested from you', an approval, and an Approved summary", async () => {
    renderPr();
    expect(await screen.findByText("Add a fix")).toBeInTheDocument();
    // The active account (octocat) is a requested reviewer → prominent badge.
    expect(
      await screen.findByText("Review requested from you"),
    ).toBeInTheDocument();
    // dave's latest review is APPROVED → reviewer decision + summary chip.
    expect(await screen.findByText("dave")).toBeInTheDocument();
    // "Approved" appears for both the reviewer decision and the summary chip.
    expect(screen.getAllByText("Approved").length).toBeGreaterThanOrEqual(1);
  });
});
