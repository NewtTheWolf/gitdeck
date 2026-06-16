import { beforeEach, describe, expect, it, vi } from "vitest";

// vi.mock is hoisted above imports, so the mock fn must be created via
// vi.hoisted to be available inside the factory under vitest 4.
const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { api } from "./api";

describe("api", () => {
  beforeEach(() => invoke.mockReset().mockResolvedValue(undefined));

  it("listTasks passes params", async () => {
    await api.listTasks({ status: "open" });
    expect(invoke).toHaveBeenCalledWith("list_tasks", { params: { status: "open" } });
  });
  it("createTask passes params", async () => {
    await api.createTask({ title: "x" });
    expect(invoke).toHaveBeenCalledWith("create_task", { params: { title: "x" } });
  });
  it("updateTask passes params", async () => {
    await api.updateTask({ id: "id-1", status: "open" });
    expect(invoke).toHaveBeenCalledWith("update_task", { params: { id: "id-1", status: "open" } });
  });
  it("completeTask passes id", async () => {
    await api.completeTask("id-1");
    expect(invoke).toHaveBeenCalledWith("complete_task", { id: "id-1" });
  });
  it("deleteTask passes id", async () => {
    await api.deleteTask("id-2");
    expect(invoke).toHaveBeenCalledWith("delete_task", { id: "id-2" });
  });

  it("listAccounts invokes list_accounts", async () => {
    await api.listAccounts();
    expect(invoke).toHaveBeenCalledWith("list_accounts");
  });
  it("listRepos passes id", async () => {
    await api.listRepos("acc-1");
    expect(invoke).toHaveBeenCalledWith("list_repos", { id: "acc-1" });
  });
  it("listIssues passes id", async () => {
    await api.listIssues("acc-1");
    expect(invoke).toHaveBeenCalledWith("list_issues", { id: "acc-1" });
  });
  it("listPullRequests passes id", async () => {
    await api.listPullRequests("acc-1");
    expect(invoke).toHaveBeenCalledWith("list_pull_requests", { id: "acc-1" });
  });
  it("deleteAccount passes id", async () => {
    await api.deleteAccount("acc-1");
    expect(invoke).toHaveBeenCalledWith("delete_account", { id: "acc-1" });
  });
  it("syncAccount passes id", async () => {
    await api.syncAccount("acc-2");
    expect(invoke).toHaveBeenCalledWith("sync_account", { id: "acc-2" });
  });
  it("captureSnapshots passes id", async () => {
    await api.captureSnapshots("acc-1");
    expect(invoke).toHaveBeenCalledWith("capture_snapshots", { id: "acc-1" });
  });
  it("dailyDigest passes id", async () => {
    await api.dailyDigest("acc-1");
    expect(invoke).toHaveBeenCalledWith("daily_digest", { id: "acc-1" });
  });
  it("listSnapshots wraps params", async () => {
    await api.listSnapshots({ id: "acc-1", since_day: "2026-06-15" });
    expect(invoke).toHaveBeenCalledWith("list_snapshots", {
      params: { id: "acc-1", since_day: "2026-06-15" },
    });
  });
  it("listNotifications passes id", async () => {
    await api.listNotifications("acc-1");
    expect(invoke).toHaveBeenCalledWith("list_notifications", { id: "acc-1" });
  });
  it("markNotificationRead wraps id+thread_id in params", async () => {
    await api.markNotificationRead("acc-1", "thread-9");
    expect(invoke).toHaveBeenCalledWith("mark_notification_read", {
      params: { id: "acc-1", thread_id: "thread-9" },
    });
  });
  it("listWorkflowRuns passes params", async () => {
    await api.listWorkflowRuns({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      per_page: 1,
    });
    expect(invoke).toHaveBeenCalledWith("list_workflow_runs", {
      params: { id: "acc-1", owner: "acme", repo: "widget", per_page: 1 },
    });
  });
  it("getRepoDetail passes params", async () => {
    await api.getRepoDetail({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("get_repo_detail", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("listReleases passes params", async () => {
    await api.listReleases({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("list_releases", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("listForks passes params", async () => {
    await api.listForks({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("list_forks", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("listContributors passes params", async () => {
    await api.listContributors({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("list_contributors", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("getLanguages passes params", async () => {
    await api.getLanguages({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("get_languages", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("getTrafficViews passes params", async () => {
    await api.getTrafficViews({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("get_traffic_views", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("getTrafficClones passes params", async () => {
    await api.getTrafficClones({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("get_traffic_clones", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("listReferrers passes params", async () => {
    await api.listReferrers({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("list_referrers", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("listPaths passes params", async () => {
    await api.listPaths({ id: "acc-1", owner: "acme", repo: "widget" });
    expect(invoke).toHaveBeenCalledWith("list_paths", {
      params: { id: "acc-1", owner: "acme", repo: "widget" },
    });
  });
  it("searchCode passes params", async () => {
    await api.searchCode({ id: "acc-1", query: '"acme/widget"', per_page: 20 });
    expect(invoke).toHaveBeenCalledWith("search_code", {
      params: { id: "acc-1", query: '"acme/widget"', per_page: 20 },
    });
  });
  it("searchIssues passes params", async () => {
    await api.searchIssues({ id: "acc-1", query: '"acme/widget"', per_page: 20 });
    expect(invoke).toHaveBeenCalledWith("search_issues", {
      params: { id: "acc-1", query: '"acme/widget"', per_page: 20 },
    });
  });
  it("startGithubLogin passes named args", async () => {
    await api.startGithubLogin("cid", "secret");
    expect(invoke).toHaveBeenCalledWith("start_github_login", {
      clientId: "cid",
      clientSecret: "secret",
    });
  });

  // --- triage write actions -------------------------------------------------
  it("setIssueState wraps params", async () => {
    await api.setIssueState({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      number: 42,
      state: "closed",
    });
    expect(invoke).toHaveBeenCalledWith("set_issue_state", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42, state: "closed" },
    });
  });
  it("addIssueLabels wraps params", async () => {
    await api.addIssueLabels({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      number: 42,
      labels: ["bug"],
    });
    expect(invoke).toHaveBeenCalledWith("add_issue_labels", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42, labels: ["bug"] },
    });
  });
  it("removeIssueLabel wraps params", async () => {
    await api.removeIssueLabel({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      number: 42,
      label: "bug",
    });
    expect(invoke).toHaveBeenCalledWith("remove_issue_label", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42, label: "bug" },
    });
  });
  it("addIssueAssignees wraps params", async () => {
    await api.addIssueAssignees({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      number: 42,
      assignees: ["octocat"],
    });
    expect(invoke).toHaveBeenCalledWith("add_issue_assignees", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42, assignees: ["octocat"] },
    });
  });
  it("createIssueComment wraps params", async () => {
    await api.createIssueComment({
      id: "acc-1",
      owner: "acme",
      repo: "widget",
      number: 42,
      body: "looking into this",
    });
    expect(invoke).toHaveBeenCalledWith("create_issue_comment", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42, body: "looking into this" },
    });
  });

  // --- in-app issue / PR detail ---------------------------------------------
  it("getIssue wraps params", async () => {
    await api.getIssue({ id: "acc-1", owner: "acme", repo: "widget", number: 42 });
    expect(invoke).toHaveBeenCalledWith("get_issue", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42 },
    });
  });
  it("getPullRequest wraps params", async () => {
    await api.getPullRequest({ id: "acc-1", owner: "acme", repo: "widget", number: 7 });
    expect(invoke).toHaveBeenCalledWith("get_pull_request", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 7 },
    });
  });
  it("listIssueComments wraps params", async () => {
    await api.listIssueComments({ id: "acc-1", owner: "acme", repo: "widget", number: 42 });
    expect(invoke).toHaveBeenCalledWith("list_issue_comments", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 42 },
    });
  });
  it("listPullReviews wraps params", async () => {
    await api.listPullReviews({ id: "acc-1", owner: "acme", repo: "widget", number: 7 });
    expect(invoke).toHaveBeenCalledWith("list_pull_reviews", {
      params: { id: "acc-1", owner: "acme", repo: "widget", number: 7 },
    });
  });

  // --- boards ---------------------------------------------------------------
  it("listBoards invokes list_boards", async () => {
    await api.listBoards();
    expect(invoke).toHaveBeenCalledWith("list_boards");
  });
  it("getBoard passes id", async () => {
    await api.getBoard("b-1");
    expect(invoke).toHaveBeenCalledWith("get_board", { id: "b-1" });
  });
  it("createBoard wraps name in params", async () => {
    await api.createBoard("Sprint");
    expect(invoke).toHaveBeenCalledWith("create_board", {
      params: { name: "Sprint" },
    });
  });
  it("renameBoard wraps id+name in params", async () => {
    await api.renameBoard("b-1", "Renamed");
    expect(invoke).toHaveBeenCalledWith("rename_board", {
      params: { id: "b-1", name: "Renamed" },
    });
  });
  it("deleteBoard passes id", async () => {
    await api.deleteBoard("b-1");
    expect(invoke).toHaveBeenCalledWith("delete_board", { id: "b-1" });
  });
  it("reorderBoards wraps ordered_ids in params", async () => {
    await api.reorderBoards(["b-2", "b-1"]);
    expect(invoke).toHaveBeenCalledWith("reorder_boards", {
      params: { ordered_ids: ["b-2", "b-1"] },
    });
  });
  it("createColumn wraps board_id+name+filter in params", async () => {
    await api.createColumn("b-1", "Todo", { state: "open" });
    expect(invoke).toHaveBeenCalledWith("create_column", {
      params: { board_id: "b-1", name: "Todo", filter: { state: "open" } },
    });
  });
  it("createColumn passes undefined filter when omitted", async () => {
    await api.createColumn("b-1", "Manual");
    expect(invoke).toHaveBeenCalledWith("create_column", {
      params: { board_id: "b-1", name: "Manual", filter: undefined },
    });
  });
  it("updateColumn wraps id+name+filter in params", async () => {
    await api.updateColumn("c-1", "Doing", { state: "all" });
    expect(invoke).toHaveBeenCalledWith("update_column", {
      params: { id: "c-1", name: "Doing", filter: { state: "all" } },
    });
  });
  it("deleteColumn passes id", async () => {
    await api.deleteColumn("c-1");
    expect(invoke).toHaveBeenCalledWith("delete_column", { id: "c-1" });
  });
  it("reorderColumns wraps board_id+ordered_ids in params", async () => {
    await api.reorderColumns("b-1", ["c-2", "c-1"]);
    expect(invoke).toHaveBeenCalledWith("reorder_columns", {
      params: { board_id: "b-1", ordered_ids: ["c-2", "c-1"] },
    });
  });
  it("placeCard wraps board_id+column_id+item_key in params", async () => {
    await api.placeCard("b-1", "c-1", "todo:42");
    expect(invoke).toHaveBeenCalledWith("place_card", {
      params: { board_id: "b-1", column_id: "c-1", item_key: "todo:42" },
    });
  });
  it("removeCard wraps board_id+item_key in params", async () => {
    await api.removeCard("b-1", "todo:42");
    expect(invoke).toHaveBeenCalledWith("remove_card", {
      params: { board_id: "b-1", item_key: "todo:42" },
    });
  });
});
