import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { MemoryRouter } from "react-router-dom";

// The card (via listRow's openItem) imports the Tauri opener plugin, which has
// no implementation under jsdom — stub it so the module loads.
vi.mock("@tauri-apps/plugin-opener", () => ({
  openUrl: vi.fn(() => Promise.resolve()),
}));

const navigate = vi.fn();
vi.mock("react-router-dom", async (orig) => {
  const actual = await orig<typeof import("react-router-dom")>();
  return { ...actual, useNavigate: () => navigate };
});

import "../../lib/i18n";
import type { Issue, PullRequest } from "../../lib/api";
import { IssueCard } from "./IssueCard";

const baseIssue: Issue = {
  number: 42,
  title: "Fix the broken thing",
  html_url: "https://github.com/acme/widget/issues/42",
  state: "open",
  author_login: "octocat",
  author_avatar_url: null,
  repo_name_with_owner: "acme/widget",
  created_at: "2026-06-10T00:00:00Z",
  updated_at: "2026-06-15T00:00:00Z",
  comments_count: 3,
  labels: [{ name: "bug", color: "d73a4a" }],
  assignees: [],
};

function renderCard(ui: React.ReactElement) {
  return render(<MemoryRouter>{ui}</MemoryRouter>);
}

afterEach(() => {
  cleanup();
  navigate.mockReset();
});

describe("IssueCard", () => {
  it("renders title, repo, number and label", () => {
    renderCard(
      <IssueCard
        item={baseIssue}
        kindKey="issue"
        kind={<span data-testid="kind" />}
      />,
    );
    expect(screen.getByText("Fix the broken thing")).toBeInTheDocument();
    expect(screen.getByText("acme/widget")).toBeInTheDocument();
    expect(screen.getByText("#42")).toBeInTheDocument();
    expect(screen.getByText("bug")).toBeInTheDocument();
    expect(screen.getByTestId("kind")).toBeInTheDocument();
  });

  it("links to the in-app issue detail path", () => {
    renderCard(<IssueCard item={baseIssue} kindKey="issue" kind={null} />);
    // The card itself is the first link; its href is the in-app detail route.
    const card = screen.getByRole("link", { name: /Fix the broken thing/i });
    expect(card).toHaveAttribute("href", "/repos/acme/widget/issues/42");
  });

  it("links a PR to the in-app pull detail path", () => {
    const pr: PullRequest = { ...baseIssue, is_draft: false };
    renderCard(<IssueCard item={pr} kindKey="pr" kind={null} />);
    const card = screen.getByRole("link", { name: /Fix the broken thing/i });
    expect(card).toHaveAttribute("href", "/repos/acme/widget/pull/42");
  });

  it("navigates to the issue detail on click instead of opening a url", async () => {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    renderCard(<IssueCard item={baseIssue} kindKey="issue" kind={null} />);
    const card = screen.getByRole("link", { name: /Fix the broken thing/i });
    card.click();
    expect(navigate).toHaveBeenCalledWith("/repos/acme/widget/issues/42");
    expect(openUrl).not.toHaveBeenCalled();
  });

  it("caps labels and shows a +N overflow", () => {
    const many: Issue = {
      ...baseIssue,
      labels: Array.from({ length: 7 }, (_, i) => ({
        name: `label-${i}`,
        color: "ededed",
      })),
    };
    renderCard(<IssueCard item={many} kindKey="issue" kind={null} />);
    // MAX_LABELS = 4, so 3 overflow.
    expect(screen.getByText("+3")).toBeInTheDocument();
  });

  it("renders the Draft badge for a draft PR via extraMeta", () => {
    const pr: PullRequest = { ...baseIssue, is_draft: true };
    renderCard(
      <IssueCard
        item={pr}
        kindKey="pr"
        kind={null}
        extraMeta={<span>Draft</span>}
      />,
    );
    expect(screen.getByText("Draft")).toBeInTheDocument();
  });
});
