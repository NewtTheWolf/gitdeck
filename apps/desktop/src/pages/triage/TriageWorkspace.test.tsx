import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  screen,
  waitFor,
} from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

// TriageRow's title navigates to the in-app detail, so the view needs a Router.
function renderTriage() {
  return renderWithClient(
    <MemoryRouter>
      <TriageWorkspace />
    </MemoryRouter>,
  );
}

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const {
  listIssues,
  setIssueState,
  addIssueLabels,
  removeIssueLabel,
  addIssueAssignees,
  createIssueComment,
} = vi.hoisted(() => ({
  listIssues: vi.fn(),
  setIssueState: vi.fn(),
  addIssueLabels: vi.fn(),
  removeIssueLabel: vi.fn(),
  addIssueAssignees: vi.fn(),
  createIssueComment: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    listIssues: (...a: unknown[]) => listIssues(...a),
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
import TriageWorkspace from "./TriageWorkspace";

const issues = [
  {
    number: 42,
    title: "Fix the broken thing",
    html_url: "https://github.com/acme/widget/issues/42",
    state: "open",
    author_login: "alice",
    author_avatar_url: null,
    repo_name_with_owner: "acme/widget",
    created_at: "2026-06-10T00:00:00Z",
    updated_at: "2026-06-15T00:00:00Z",
    comments_count: 3,
    labels: [{ name: "bug", color: "d73a4a" }],
    assignees: [],
  },
];

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("TriageWorkspace", () => {
  beforeEach(() => {
    listIssues.mockReset().mockResolvedValue(issues);
    setIssueState.mockReset().mockResolvedValue(undefined);
    addIssueLabels.mockReset().mockResolvedValue(undefined);
    removeIssueLabel.mockReset().mockResolvedValue(undefined);
    addIssueAssignees.mockReset().mockResolvedValue(undefined);
    createIssueComment
      .mockReset()
      .mockResolvedValue("https://github.com/acme/widget/issues/42#issuecomment-1");
  });

  it("renders issues from listIssues", async () => {
    renderTriage();
    expect(await screen.findByText("Fix the broken thing")).toBeInTheDocument();
    expect(screen.getByText("alice")).toBeInTheDocument();
    expect(screen.getByText("#42")).toBeInTheDocument();
  });

  it("Close shows an inline confirm then calls setIssueState with the right coords", async () => {
    renderTriage();
    await screen.findByText("Fix the broken thing");

    fireEvent.click(screen.getByRole("button", { name: "Close" }));
    // Confirm prompt appears; the action is not called until confirmed.
    expect(screen.getByText("Close this issue?")).toBeInTheDocument();
    expect(setIssueState).not.toHaveBeenCalled();

    // The confirm button is the now-second "Close" (inside the confirm cluster).
    const confirmButtons = screen.getAllByRole("button", { name: "Close" });
    fireEvent.click(confirmButtons[confirmButtons.length - 1]);

    await waitFor(() =>
      expect(setIssueState).toHaveBeenCalledWith({
        id: "acc-1",
        owner: "acme",
        repo: "widget",
        number: 42,
        state: "closed",
      }),
    );
  });

  it("Assign to me calls addIssueAssignees with the account login", async () => {
    renderTriage();
    await screen.findByText("Fix the broken thing");

    fireEvent.click(screen.getByRole("button", { name: "Assign to me" }));
    await waitFor(() =>
      expect(addIssueAssignees).toHaveBeenCalledWith({
        id: "acc-1",
        owner: "acme",
        repo: "widget",
        number: 42,
        assignees: ["octocat"],
      }),
    );
  });

  it("submitting a comment calls createIssueComment and shows confirmation", async () => {
    renderTriage();
    await screen.findByText("Fix the broken thing");

    // The toggle is the only "Comment" button before the composer is open.
    fireEvent.click(screen.getByRole("button", { name: "Comment" }));
    const box = screen.getByPlaceholderText("Leave a comment…");
    fireEvent.change(box, { target: { value: "looking into this" } });
    // Now there are two "Comment" buttons (toggle + composer send); the send
    // button is the one inside the composer form (the textarea's form).
    const sendButton = box.closest("form")!.querySelector("button[type=submit]")!;
    fireEvent.click(sendButton);

    await waitFor(() =>
      expect(createIssueComment).toHaveBeenCalledWith({
        id: "acc-1",
        owner: "acme",
        repo: "widget",
        number: 42,
        body: "looking into this",
      }),
    );
    expect(
      await screen.findByText("Comment added — view on GitHub"),
    ).toBeInTheDocument();
  });

  it("reverts an optimistic assign and shows an error when the write rejects", async () => {
    addIssueAssignees.mockRejectedValue(new Error("403 Forbidden"));
    renderTriage();
    await screen.findByText("Fix the broken thing");

    fireEvent.click(screen.getByRole("button", { name: "Assign to me" }));

    // Error surfaces inline; the button re-enables (optimistic assignee reverted).
    expect(await screen.findByText(/403 Forbidden/)).toBeInTheDocument();
    await waitFor(() =>
      expect(
        screen.getByRole("button", { name: "Assign to me" }),
      ).not.toBeDisabled(),
    );
  });
});
