import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";
import { renderWithClient } from "../../lib/test/renderWithClient";

// NotificationRow navigates to the in-app detail, so the view needs a Router.
function renderInbox() {
  return renderWithClient(
    <MemoryRouter>
      <InboxView />
    </MemoryRouter>,
  );
}

vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const { listNotifications, markNotificationRead } = vi.hoisted(() => ({
  listNotifications: vi.fn(),
  markNotificationRead: vi.fn(),
}));
vi.mock("../../lib/api", () => ({
  api: {
    listNotifications: (...a: unknown[]) => listNotifications(...a),
    markNotificationRead: (...a: unknown[]) => markNotificationRead(...a),
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
import InboxView, { notificationWebUrl } from "./InboxView";

const notifications = [
  {
    id: "n-1",
    repo_name_with_owner: "acme/widget",
    subject_title: "Unread issue here",
    subject_type: "Issue",
    subject_url: "https://api.github.com/repos/acme/widget/issues/12",
    reason: "mention",
    unread: true,
    updated_at: "2026-06-15T10:00:00Z",
  },
  {
    id: "n-2",
    repo_name_with_owner: "acme/widget",
    subject_title: "Read pull request",
    subject_type: "PullRequest",
    subject_url: "https://api.github.com/repos/acme/widget/pulls/8",
    reason: "review_requested",
    unread: false,
    updated_at: "2026-06-14T10:00:00Z",
  },
];

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("notificationWebUrl", () => {
  it("converts an issue api url to a web url", () => {
    expect(
      notificationWebUrl("https://api.github.com/repos/acme/widget/issues/12"),
    ).toBe("https://github.com/acme/widget/issues/12");
  });
  it("converts a pulls api url to a /pull/ web url", () => {
    expect(
      notificationWebUrl("https://api.github.com/repos/acme/widget/pulls/8"),
    ).toBe("https://github.com/acme/widget/pull/8");
  });
  it("returns null for unconvertible urls", () => {
    expect(notificationWebUrl(null)).toBeNull();
    expect(notificationWebUrl("https://example.com/x")).toBeNull();
  });
});

describe("InboxView", () => {
  beforeEach(() => {
    listNotifications.mockReset();
    markNotificationRead.mockReset().mockResolvedValue(undefined);
  });

  it("renders both unread and read notifications", async () => {
    listNotifications.mockResolvedValue(notifications);
    renderInbox();
    expect(await screen.findByText("Unread issue here")).toBeInTheDocument();
    expect(screen.getByText("Read pull request")).toBeInTheDocument();
  });

  it("marks a notification read via the per-item action", async () => {
    listNotifications.mockResolvedValue(notifications);
    renderInbox();
    await screen.findByText("Unread issue here");

    // Only the unread item exposes a mark-read button.
    const markButtons = screen.getAllByLabelText("Mark read");
    expect(markButtons).toHaveLength(1);
    fireEvent.click(markButtons[0]);

    await waitFor(() =>
      expect(markNotificationRead).toHaveBeenCalledWith("acc-1", "n-1"),
    );
  });

  it("shows the empty state when there are no notifications", async () => {
    listNotifications.mockResolvedValue([]);
    renderInbox();
    expect(await screen.findByText(/Inbox zero/i)).toBeInTheDocument();
  });
});
