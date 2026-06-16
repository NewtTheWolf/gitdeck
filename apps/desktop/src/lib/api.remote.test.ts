import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
const { getGrpcClient, isRemote } = vi.hoisted(() => ({
  getGrpcClient: vi.fn(),
  isRemote: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));
vi.mock("./transport", () => ({
  getGrpcClient: () => getGrpcClient(),
  isRemote: () => isRemote(),
}));

import { api } from "./api";

describe("api dispatch (remote vs local)", () => {
  beforeEach(() => {
    invoke.mockReset().mockResolvedValue(undefined);
    getGrpcClient.mockReset();
    isRemote.mockReset();
  });

  it("listTasks calls invoke when mode is local", async () => {
    isRemote.mockReturnValue(false);
    invoke.mockResolvedValue([]);
    await api.listTasks({ status: "open" });
    expect(invoke).toHaveBeenCalledWith("list_tasks", {
      params: { status: "open" },
    });
    expect(getGrpcClient).not.toHaveBeenCalled();
  });

  it("listTasks routes to invoke even when remote (local-first)", async () => {
    isRemote.mockReturnValue(true);
    invoke.mockResolvedValue([]);
    await api.listTasks({ status: "open" });
    expect(invoke).toHaveBeenCalledWith("list_tasks", {
      params: { status: "open" },
    });
    expect(getGrpcClient).not.toHaveBeenCalled();
  });

  it("listAccounts parses config_json into config (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = {
      listAccounts: vi.fn().mockResolvedValue({
        accounts: [
          {
            id: "a1",
            provider: "github",
            displayName: "Octo",
            configJson: '{"token_present":true}',
            createdAt: "2026-01-01",
          },
        ],
      }),
    };
    getGrpcClient.mockReturnValue(client);

    const res = await api.listAccounts();

    expect(res).toEqual([
      {
        id: "a1",
        provider: "github",
        display_name: "Octo",
        created_at: "2026-01-01",
        config: { token_present: true },
      },
    ]);
  });

  // Boards/columns/cards are now a SYNCED entity: ALWAYS local-first (the
  // embedded store), even in remote mode. The K3 sync engine reconciles them
  // with the server in the background, so the CRUD methods never hit the client.
  it("getBoard routes to invoke even when remote (local-first)", async () => {
    isRemote.mockReturnValue(true);
    invoke.mockResolvedValue(null);
    await api.getBoard("b1");
    expect(invoke).toHaveBeenCalledWith("get_board", { id: "b1" });
    expect(getGrpcClient).not.toHaveBeenCalled();
  });

  it("createColumn routes to invoke even when remote (local-first)", async () => {
    isRemote.mockReturnValue(true);
    invoke.mockResolvedValue({});
    await api.createColumn("b1", "New", { state: "closed" });
    expect(invoke).toHaveBeenCalledWith("create_column", {
      params: { board_id: "b1", name: "New", filter: { state: "closed" } },
    });
    expect(getGrpcClient).not.toHaveBeenCalled();
  });

  it("createTask routes to invoke even when remote (local-first)", async () => {
    isRemote.mockReturnValue(true);
    invoke.mockResolvedValue({});
    await api.createTask({ title: "x" });
    expect(invoke).toHaveBeenCalledWith("create_task", {
      params: { title: "x" },
    });
    expect(getGrpcClient).not.toHaveBeenCalled();
  });

  it("getIssue normalizes nested labels/assignees and widens number (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = {
      getIssue: vi.fn().mockResolvedValue({
        $typeName: "gitdeck.v1.GetIssueResponse",
        issue: {
          $typeName: "gitdeck.v1.IssueDetailMessage",
          number: 42n,
          title: "Bug",
          htmlUrl: "https://x/42",
          state: "open",
          authorLogin: "octo",
          repoNameWithOwner: "o/r",
          createdAt: "2026-01-01",
          updatedAt: "2026-01-02",
          commentsCount: 3n,
          labels: [{ $typeName: "gitdeck.v1.LabelMessage", name: "bug", color: "f00" }],
          assignees: [{ $typeName: "gitdeck.v1.UserMessage", login: "octo", avatarUrl: "https://a" }],
        },
      }),
    };
    getGrpcClient.mockReturnValue(client);

    const issue = await api.getIssue({ id: "a1", owner: "o", repo: "r", number: 42 });

    expect(client.getIssue).toHaveBeenCalledWith({
      id: "a1",
      owner: "o",
      repo: "r",
      number: 42n,
    });
    expect(invoke).not.toHaveBeenCalled();
    expect(issue).toMatchObject({
      title: "Bug",
      html_url: "https://x/42",
      repo_name_with_owner: "o/r",
      labels: [{ name: "bug", color: "f00" }],
      assignees: [{ login: "octo", avatar_url: "https://a" }],
    });
  });

  it("getIssue returns null when the issue is absent (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = { getIssue: vi.fn().mockResolvedValue({ issue: undefined }) };
    getGrpcClient.mockReturnValue(client);

    const issue = await api.getIssue({ id: "a1", owner: "o", repo: "r", number: 7 });

    expect(issue).toBeNull();
  });

  it("setIssueState calls the client and returns void (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = { setIssueState: vi.fn().mockResolvedValue({}) };
    getGrpcClient.mockReturnValue(client);

    const res = await api.setIssueState({
      id: "a1",
      owner: "o",
      repo: "r",
      number: 9,
      state: "closed",
    });

    expect(client.setIssueState).toHaveBeenCalledWith({
      id: "a1",
      owner: "o",
      repo: "r",
      number: 9n,
      state: "closed",
    });
    expect(invoke).not.toHaveBeenCalled();
    expect(res).toBeUndefined();
  });

  it("listWorkflowRuns normalizes runs to snake_case (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = {
      listWorkflowRuns: vi.fn().mockResolvedValue({
        runs: [
          {
            $typeName: "gitdeck.v1.WorkflowRunMessage",
            id: 100n,
            name: "CI",
            headBranch: "main",
            status: "completed",
            conclusion: "success",
            htmlUrl: "https://x/run",
            createdAt: "2026-01-01",
            updatedAt: "2026-01-02",
          },
        ],
      }),
    };
    getGrpcClient.mockReturnValue(client);

    const runs = await api.listWorkflowRuns({
      id: "a1",
      owner: "o",
      repo: "r",
      per_page: 10,
    });

    expect(client.listWorkflowRuns).toHaveBeenCalledWith({
      id: "a1",
      owner: "o",
      repo: "r",
      perPage: 10,
    });
    expect(runs).toEqual([
      {
        id: 100,
        name: "CI",
        head_branch: "main",
        status: "completed",
        conclusion: "success",
        html_url: "https://x/run",
        created_at: "2026-01-01",
        updated_at: "2026-01-02",
      },
    ]);
  });

  it("getTrafficViews maps an unset traffic message to null (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = { getTrafficViews: vi.fn().mockResolvedValue({ traffic: undefined }) };
    getGrpcClient.mockReturnValue(client);

    const traffic = await api.getTrafficViews({ id: "a1", owner: "o", repo: "r" });

    expect(client.getTrafficViews).toHaveBeenCalledWith({
      id: "a1",
      owner: "o",
      repo: "r",
      perPage: undefined,
    });
    expect(traffic).toBeNull();
  });

  it("createIssueComment returns the created comment url (remote)", async () => {
    isRemote.mockReturnValue(true);
    const client = {
      createIssueComment: vi.fn().mockResolvedValue({ htmlUrl: "https://x/c1" }),
    };
    getGrpcClient.mockReturnValue(client);

    const url = await api.createIssueComment({
      id: "a1",
      owner: "o",
      repo: "r",
      number: 5,
      body: "hi",
    });

    expect(client.createIssueComment).toHaveBeenCalledWith({
      id: "a1",
      owner: "o",
      repo: "r",
      number: 5n,
      body: "hi",
    });
    expect(url).toBe("https://x/c1");
  });

  it("getIssue routes to invoke when local", async () => {
    isRemote.mockReturnValue(false);
    invoke.mockResolvedValue(null);
    await api.getIssue({ id: "a1", owner: "o", repo: "r", number: 1 });
    expect(invoke).toHaveBeenCalledWith("get_issue", {
      params: { id: "a1", owner: "o", repo: "r", number: 1 },
    });
    expect(getGrpcClient).not.toHaveBeenCalled();
  });
});
