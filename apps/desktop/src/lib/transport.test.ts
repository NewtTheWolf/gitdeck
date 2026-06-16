import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// Capture the interceptors handed to createGrpcWebTransport so we can run the
// bearer interceptor against a fake request and assert which token it sends.
const captured: { interceptors: unknown[] } = { interceptors: [] };

vi.mock("@connectrpc/connect-web", () => ({
  createGrpcWebTransport: (opts: { interceptors: unknown[] }) => {
    captured.interceptors = opts.interceptors;
    return { __fake: true };
  },
}));

vi.mock("@connectrpc/connect", () => ({
  createClient: () => ({ __client: true }),
}));

import { getGrpcClient, setServerConfig, type ServerConfig } from "./transport";

type Interceptor = (
  next: (req: { header: Headers }) => Promise<unknown>,
) => (req: { header: Headers }) => Promise<unknown>;

/** Run the (single) captured interceptor and return the Authorization header. */
async function bearerHeaderFor(config: ServerConfig): Promise<string | null> {
  getGrpcClient(config);
  const interceptor = captured.interceptors[0] as Interceptor;
  let seen: string | null = null;
  const handler = interceptor(async (req) => {
    seen = req.header.get("Authorization");
    return {};
  });
  await handler({ header: new Headers() });
  return seen;
}

beforeEach(() => {
  localStorage.clear();
  captured.interceptors = [];
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("transport bearer token", () => {
  it("uses the session token over the api key", async () => {
    const header = await bearerHeaderFor({
      mode: "remote",
      url: "http://localhost:50061",
      apiKey: "static-key",
      sessionToken: "jwt-session",
    });
    expect(header).toBe("Bearer jwt-session");
  });

  it("falls back to the api key when there is no session token", async () => {
    const header = await bearerHeaderFor({
      mode: "remote",
      url: "http://localhost:50061",
      apiKey: "static-key",
    });
    expect(header).toBe("Bearer static-key");
  });

  it("sends no Authorization header when neither token is set", async () => {
    const header = await bearerHeaderFor({
      mode: "remote",
      url: "http://localhost:50061",
    });
    expect(header).toBeNull();
  });

  it("rebuilds the client when the session token changes", () => {
    setServerConfig({ mode: "remote", url: "http://localhost:50061" });
    const a = getGrpcClient();
    const b = getGrpcClient();
    expect(b).toBe(a); // memoized while config is unchanged

    setServerConfig({
      mode: "remote",
      url: "http://localhost:50061",
      sessionToken: "jwt-1",
    });
    const c = getGrpcClient();
    expect(c).not.toBe(a); // session token change invalidates the cached client
  });

  it("persists and round-trips the session token", () => {
    setServerConfig({
      mode: "remote",
      url: "http://localhost:50061",
      sessionToken: "jwt-persist",
    });
    const raw = localStorage.getItem("gitdeck.server");
    expect(raw).toContain("jwt-persist");
  });
});
