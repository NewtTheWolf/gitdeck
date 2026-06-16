import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import "@testing-library/jest-dom/vitest";
import { renderWithClient } from "../lib/test/renderWithClient";

const {
  listAccounts,
  deleteAccount,
  startGithubLogin,
  serverLogin,
  serverRegister,
  serverGetMe,
  serverSetGithubToken,
} = vi.hoisted(() => ({
  listAccounts: vi.fn(),
  deleteAccount: vi.fn(),
  startGithubLogin: vi.fn(),
  serverLogin: vi.fn(),
  serverRegister: vi.fn(),
  serverGetMe: vi.fn(),
  serverSetGithubToken: vi.fn(),
}));
vi.mock("../lib/api", () => ({
  api: {
    listAccounts: (...a: unknown[]) => listAccounts(...a),
    deleteAccount: (...a: unknown[]) => deleteAccount(...a),
    startGithubLogin: (...a: unknown[]) => startGithubLogin(...a),
    serverLogin: (...a: unknown[]) => serverLogin(...a),
    serverRegister: (...a: unknown[]) => serverRegister(...a),
    serverGetMe: (...a: unknown[]) => serverGetMe(...a),
    serverSetGithubToken: (...a: unknown[]) => serverSetGithubToken(...a),
  },
}));

const { getServerConfig, setServerConfig, getGrpcClient, isRemote } =
  vi.hoisted(() => ({
    getServerConfig: vi.fn(),
    setServerConfig: vi.fn(),
    getGrpcClient: vi.fn(),
    isRemote: vi.fn(),
  }));
vi.mock("../lib/transport", () => ({
  getServerConfig: () => getServerConfig(),
  setServerConfig: (...a: unknown[]) => setServerConfig(...a),
  getGrpcClient: (...a: unknown[]) => getGrpcClient(...a),
  isRemote: (...a: unknown[]) => isRemote(...a),
  subscribeServerConfig: () => () => {},
}));

import "../lib/i18n";
import Settings from "./Settings";

beforeEach(() => {
  listAccounts.mockResolvedValue([]);
  getServerConfig.mockReturnValue({ mode: "local", url: "" });
  isRemote.mockReturnValue(false);
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("Settings — Server / Backend section", () => {
  it("renders the server section with mode toggle and url input", async () => {
    renderWithClient(<Settings />);
    expect(await screen.findByText("Server / Backend")).toBeInTheDocument();
    expect(screen.getByText("Local (embedded)")).toBeInTheDocument();
    expect(screen.getByText("Remote (gitdeck-server)")).toBeInTheDocument();
    expect(
      screen.getByPlaceholderText("http://127.0.0.1:50061"),
    ).toBeInTheDocument();
  });

  it("Save persists the config via setServerConfig", async () => {
    renderWithClient(<Settings />);
    await screen.findByText("Server / Backend");

    // Select remote and fill in a URL.
    fireEvent.click(screen.getByDisplayValue("remote"));
    fireEvent.change(screen.getByPlaceholderText("http://127.0.0.1:50061"), {
      target: { value: "http://localhost:50061" },
    });
    fireEvent.click(screen.getByText("Save"));

    await waitFor(() => expect(setServerConfig).toHaveBeenCalledTimes(1));
    expect(setServerConfig).toHaveBeenCalledWith(
      expect.objectContaining({ mode: "remote", url: "http://localhost:50061" }),
    );
  });

  it("Test connection calls a cheap RPC via the remote client", async () => {
    const listAccountsRpc = vi.fn().mockResolvedValue({ accounts: [] });
    getGrpcClient.mockReturnValue({ listAccounts: listAccountsRpc });

    renderWithClient(<Settings />);
    await screen.findByText("Server / Backend");

    fireEvent.click(screen.getByText("Test connection"));

    await waitFor(() =>
      expect(screen.getByText("Connection successful")).toBeInTheDocument(),
    );
    expect(listAccountsRpc).toHaveBeenCalled();
  });
});

describe("Settings — server auth", () => {
  it("renders the login/register form when remote with no session token", async () => {
    getServerConfig.mockReturnValue({
      mode: "remote",
      url: "http://localhost:50061",
    });
    renderWithClient(<Settings />);
    await screen.findByText("Server / Backend");

    expect(
      screen.getByText("Account (multi-user servers)"),
    ).toBeInTheDocument();
    expect(screen.getByText("Log in")).toBeInTheDocument();
    expect(screen.getByText("Register")).toBeInTheDocument();
    // No session yet ⇒ no "Signed in as" / Logout.
    expect(screen.queryByText("Log out")).not.toBeInTheDocument();
  });

  it("logging in stores the returned session token via setServerConfig", async () => {
    getServerConfig.mockReturnValue({
      mode: "remote",
      url: "http://localhost:50061",
    });
    serverLogin.mockResolvedValue("jwt-token-123");

    renderWithClient(<Settings />);
    await screen.findByText("Server / Backend");

    fireEvent.change(screen.getByLabelText("Username"), {
      target: { value: "alice" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "secret" },
    });
    fireEvent.click(screen.getByText("Log in"));

    await waitFor(() => expect(serverLogin).toHaveBeenCalledWith("alice", "secret"));
    await waitFor(() =>
      expect(setServerConfig).toHaveBeenCalledWith(
        expect.objectContaining({ sessionToken: "jwt-token-123" }),
      ),
    );
  });

  it("shows 'Signed in as' and sync status when a session token exists", async () => {
    getServerConfig.mockReturnValue({
      mode: "remote",
      url: "http://localhost:50061",
      sessionToken: "jwt-existing",
    });
    isRemote.mockReturnValue(true);
    serverGetMe.mockResolvedValue({
      user_id: "u-1",
      username: "alice",
      has_github_token: true,
    });

    renderWithClient(<Settings />);
    await screen.findByText("Server / Backend");

    expect(await screen.findByText("Signed in as alice")).toBeInTheDocument();
    expect(screen.getByText("Log out")).toBeInTheDocument();
    expect(screen.getByText("✓ GitHub token set")).toBeInTheDocument();
    // The sync-status row label is present.
    expect(screen.getAllByText("Sync").length).toBeGreaterThan(0);
  });
});
