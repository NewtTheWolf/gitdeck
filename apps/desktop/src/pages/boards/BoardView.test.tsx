import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, screen, within } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter, Routes, Route } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";
import type { Board, Issue, PullRequest, TaskDto } from "../../lib/api";

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const { getBoard, listTasks, listIssues, listPullRequests } = vi.hoisted(() => ({
  getBoard: vi.fn(),
  listTasks: vi.fn(),
  listIssues: vi.fn(),
  listPullRequests: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    getBoard: (...a: unknown[]) => getBoard(...a),
    listTasks: (...a: unknown[]) => listTasks(...a),
    listIssues: (...a: unknown[]) => listIssues(...a),
    listPullRequests: (...a: unknown[]) => listPullRequests(...a),
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
import BoardView from "./BoardView";

const issue = (
  n: number,
  title: string,
  repo: string,
  label: string,
): Issue => ({
  number: n,
  title,
  html_url: `https://github.com/${repo}/issues/${n}`,
  state: "open",
  author_login: "octo",
  author_avatar_url: null,
  repo_name_with_owner: repo,
  created_at: "2026-06-01T00:00:00Z",
  updated_at: "2026-06-10T00:00:00Z",
  comments_count: 0,
  labels: [{ name: label, color: "d73a4a" }],
  assignees: [],
});

const board: Board = {
  id: "b-1",
  name: "My Board",
  position: 0,
  created_at: "x",
  updated_at: "x",
  columns: [
    {
      id: "col-bugs",
      name: "Bugs",
      position: 0,
      created_at: "x",
      filter: { labels: ["bug"] },
      cards: [],
    },
    {
      id: "col-acme",
      name: "Acme repo",
      position: 1,
      created_at: "x",
      filter: { repos: ["acme/other"] },
      cards: [],
    },
  ],
};

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("BoardView", () => {
  beforeEach(() => {
    getBoard.mockReset().mockResolvedValue(board);
    listTasks.mockReset().mockResolvedValue([] as TaskDto[]);
    listPullRequests.mockReset().mockResolvedValue([] as PullRequest[]);
    listIssues.mockReset().mockResolvedValue([
      issue(1, "Bug in widget", "acme/widget", "bug"), // matches Bugs (label bug)
      issue(2, "Other repo issue", "acme/other", "chore"), // matches Acme repo (repo)
    ]);
  });

  it("resolves items into the right columns by filter", async () => {
    renderWithClient(
      <MemoryRouter initialEntries={["/boards/b-1"]}>
        <Routes>
          <Route path="/boards/:boardId" element={<BoardView />} />
        </Routes>
      </MemoryRouter>,
    );

    // Title renders once board loads.
    expect(await screen.findByText("My Board")).toBeInTheDocument();
    await screen.findByText("Bug in widget");

    // Issue #1 (label bug) lands in the Bugs column; issue #2 (repo) does not.
    const bugsCol = screen.getByTestId("column-col-bugs");
    expect(within(bugsCol).getByText("Bug in widget")).toBeInTheDocument();
    expect(within(bugsCol).queryByText("Other repo issue")).toBeNull();

    // Issue #2 (acme/other) lands in the Acme repo column.
    const acmeCol = screen.getByTestId("column-col-acme");
    expect(within(acmeCol).getByText("Other repo issue")).toBeInTheDocument();
    expect(within(acmeCol).queryByText("Bug in widget")).toBeNull();
  });

  it("shows board-not-found when the board is missing", async () => {
    getBoard.mockResolvedValue(null);
    renderWithClient(
      <MemoryRouter initialEntries={["/boards/missing"]}>
        <Routes>
          <Route path="/boards/:boardId" element={<BoardView />} />
        </Routes>
      </MemoryRouter>,
    );
    expect(await screen.findByText(/Board not found/i)).toBeInTheDocument();
  });
});
