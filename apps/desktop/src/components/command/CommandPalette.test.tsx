import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";

const navigate = vi.fn();
vi.mock("react-router-dom", async (orig) => {
  const actual = await orig<typeof import("react-router-dom")>();
  return { ...actual, useNavigate: () => navigate };
});

const { openUrl } = vi.hoisted(() => ({ openUrl: vi.fn(() => Promise.resolve()) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl }));

const { listRepos, listIssues, listPullRequests } = vi.hoisted(() => ({
  listRepos: vi.fn(),
  listIssues: vi.fn(),
  listPullRequests: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    listRepos: (...a: unknown[]) => listRepos(...a),
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
import { CommandPaletteProvider } from "../../contexts/CommandPaletteContext";
import CommandPalette from "./CommandPalette";

function renderPalette() {
  return render(
    <MemoryRouter>
      <CommandPaletteProvider>
        <CommandPalette />
      </CommandPaletteProvider>
    </MemoryRouter>,
  );
}

function openWithCtrlK() {
  fireEvent.keyDown(window, { key: "k", ctrlKey: true });
}

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

beforeEach(() => {
  navigate.mockReset();
  openUrl.mockReset().mockResolvedValue(undefined);
  listRepos.mockReset().mockResolvedValue([]);
  listIssues.mockReset().mockResolvedValue([]);
  listPullRequests.mockReset().mockResolvedValue([]);
});

describe("CommandPalette", () => {
  it("is closed by default and opens on Ctrl+K", async () => {
    renderPalette();
    expect(
      screen.queryByPlaceholderText(/Search for commands/i),
    ).not.toBeInTheDocument();

    openWithCtrlK();

    expect(
      await screen.findByPlaceholderText(/Search for commands/i),
    ).toBeInTheDocument();
  });

  it("closes on Escape", async () => {
    renderPalette();
    openWithCtrlK();
    const input = await screen.findByPlaceholderText(/Search for commands/i);

    fireEvent.keyDown(input, { key: "Escape" });

    await waitFor(() =>
      expect(
        screen.queryByPlaceholderText(/Search for commands/i),
      ).not.toBeInTheDocument(),
    );
  });

  it("filters results by query", async () => {
    renderPalette();
    openWithCtrlK();
    const input = await screen.findByPlaceholderText(/Search for commands/i);

    // "Todos" nav entry is present by default.
    expect(screen.getByText("Todos")).toBeInTheDocument();

    fireEvent.change(input, { target: { value: "settings" } });

    expect(screen.queryByText("Todos")).not.toBeInTheDocument();
    expect(screen.getByText("Open Settings")).toBeInTheDocument();
  });

  it("navigates when a navigation result is selected", async () => {
    renderPalette();
    openWithCtrlK();
    await screen.findByPlaceholderText(/Search for commands/i);

    fireEvent.click(screen.getByText("Boards"));

    expect(navigate).toHaveBeenCalledWith("/boards");
  });

  it("navigates to a repo detail path when a repo result is selected", async () => {
    listRepos.mockResolvedValue([
      {
        id: "r-1",
        full_name: "acme/widget",
        description: null,
        html_url: "https://github.com/acme/widget",
        stars: 0,
        open_issues: 0,
        language: null,
        is_private: false,
        is_fork: false,
        updated_at: "2026-06-15T00:00:00Z",
      },
    ]);
    renderPalette();
    openWithCtrlK();
    await screen.findByPlaceholderText(/Search for commands/i);

    const repoRow = await screen.findByText("acme/widget");
    fireEvent.click(repoRow);

    expect(navigate).toHaveBeenCalledWith("/repos/acme/widget");
  });

  it("navigates to the in-app issue detail when an issue result is selected", async () => {
    listIssues.mockResolvedValue([
      {
        number: 12,
        title: "Fix the flux capacitor",
        html_url: "https://github.com/acme/widget/issues/12",
        state: "open",
        author_login: "marty",
        author_avatar_url: null,
        repo_name_with_owner: "acme/widget",
        created_at: "2026-06-10T00:00:00Z",
        updated_at: "2026-06-15T00:00:00Z",
        comments_count: 0,
        labels: [],
        assignees: [],
      },
    ]);
    renderPalette();
    openWithCtrlK();
    await screen.findByPlaceholderText(/Search for commands/i);

    const issueRow = await screen.findByText("Fix the flux capacitor");
    fireEvent.click(issueRow);

    expect(navigate).toHaveBeenCalledWith("/repos/acme/widget/issues/12");
    expect(openUrl).not.toHaveBeenCalled();
  });
});
