import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const { client } = vi.hoisted(() => ({
  client: {
    register: vi.fn(),
    login: vi.fn(),
    setGithubToken: vi.fn(),
    getMe: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./transport", () => ({
  getGrpcClient: () => client,
  isRemote: () => true,
}));

import { api } from "./api";

beforeEach(() => {
  vi.clearAllMocks();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("server auth wrappers", () => {
  it("serverLogin returns the token", async () => {
    client.login.mockResolvedValue({ token: "jwt-abc" });
    const token = await api.serverLogin("alice", "pw");
    expect(client.login).toHaveBeenCalledWith({
      username: "alice",
      password: "pw",
    });
    expect(token).toBe("jwt-abc");
  });

  it("serverRegister returns the user id (snake_case)", async () => {
    client.register.mockResolvedValue({ userId: "u-1" });
    const res = await api.serverRegister("alice", "pw");
    expect(res).toEqual({ user_id: "u-1" });
  });

  it("serverGetMe normalizes camelCase to snake_case", async () => {
    client.getMe.mockResolvedValue({
      userId: "u-1",
      username: "alice",
      hasGithubToken: true,
    });
    const me = await api.serverGetMe();
    expect(me).toEqual({
      user_id: "u-1",
      username: "alice",
      has_github_token: true,
    });
  });

  it("serverSetGithubToken passes the camelCase field", async () => {
    client.setGithubToken.mockResolvedValue({});
    await api.serverSetGithubToken("ghp_xyz");
    expect(client.setGithubToken).toHaveBeenCalledWith({
      githubToken: "ghp_xyz",
    });
  });
});
