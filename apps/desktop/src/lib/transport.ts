import { createClient, type Client, type Interceptor } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { Gitdeck } from "./gen/gitdeck/v1/gitdeck_pb";

/**
 * Backend selection. The desktop ALWAYS has the embedded Tauri service, so
 * "local" routes every call through `invoke`. "remote" opts into talking to a
 * standalone `gitdeck-server` over gRPC-Web; un-wired methods still fall back to
 * the embedded service even in remote mode.
 */
export interface ServerConfig {
  mode: "local" | "remote";
  url: string;
  apiKey?: string;
  /**
   * A user session token (JWT) obtained via serverLogin against a multi-user
   * server. When set it takes precedence over apiKey for the bearer header.
   */
  sessionToken?: string;
}

const STORAGE_KEY = "gitdeck.server";

const DEFAULT_CONFIG: ServerConfig = { mode: "local", url: "" };

/** Read the persisted server config, falling back to the local default. */
export function getServerConfig(): ServerConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_CONFIG };
    const parsed = JSON.parse(raw) as Partial<ServerConfig>;
    return {
      mode: parsed.mode === "remote" ? "remote" : "local",
      url: typeof parsed.url === "string" ? parsed.url : "",
      apiKey:
        typeof parsed.apiKey === "string" && parsed.apiKey.trim() !== ""
          ? parsed.apiKey
          : undefined,
      sessionToken:
        typeof parsed.sessionToken === "string" &&
        parsed.sessionToken.trim() !== ""
          ? parsed.sessionToken
          : undefined,
    };
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

const configListeners = new Set<() => void>();

/**
 * Subscribe to server-config changes (fired by setServerConfig). Returns an
 * unsubscribe fn. Used by the sync engine to restart on settings save.
 */
export function subscribeServerConfig(listener: () => void): () => void {
  configListeners.add(listener);
  return () => configListeners.delete(listener);
}

/** Persist the server config and reset the memoized client. */
export function setServerConfig(config: ServerConfig): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(config));
  // Drop the cached client so the next getGrpcClient() picks up the new config.
  cached = null;
  for (const l of configListeners) l();
}

/** True when we should route wired methods to the remote server. */
export function isRemote(): boolean {
  const cfg = getServerConfig();
  return cfg.mode === "remote" && cfg.url.trim() !== "";
}

/**
 * Bearer interceptor. The user session token (set after logging in to a
 * multi-user server) wins over the static API key; a single-user server needs
 * neither, in which case no Authorization header is sent.
 */
function bearerInterceptor(token: string | undefined): Interceptor {
  return (next) => async (req) => {
    if (token) req.header.set("Authorization", `Bearer ${token}`);
    return next(req);
  };
}

type GitdeckClient = Client<typeof Gitdeck>;

let cached: {
  url: string;
  token: string | undefined;
  client: GitdeckClient;
} | null = null;

/**
 * Build (and memoize) a connect-web client for the configured server. Memoized
 * per (url, bearer); changing the url, api key, or session token via
 * setServerConfig() invalidates it so the new bearer takes effect.
 */
export function getGrpcClient(config?: ServerConfig): GitdeckClient {
  const cfg = config ?? getServerConfig();
  const token = cfg.sessionToken ?? cfg.apiKey;
  if (cached && cached.url === cfg.url && cached.token === token) {
    return cached.client;
  }
  const transport = createGrpcWebTransport({
    baseUrl: cfg.url,
    interceptors: [bearerInterceptor(token)],
  });
  const client = createClient(Gitdeck, transport);
  cached = { url: cfg.url, token, client };
  return client;
}
