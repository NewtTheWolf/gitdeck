import { useCallback, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { Trash2, AlertCircle, CircleUser } from "lucide-react";
import { api, type Account } from "../lib/api";
import {
  getServerConfig,
  setServerConfig,
  getGrpcClient,
  type ServerConfig,
} from "../lib/transport";
import { Button } from "../components/ui/Button";
import { SyncStatusBadge } from "../components/SyncStatusBadge";

const inputClass =
  "w-full rounded-[--radius] border border-border bg-surface-2 px-3 py-2 text-sm text-text " +
  "outline-none transition-[color,border-color] duration-150 ease-out placeholder:text-text-faint " +
  "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

const labelClass = "mb-1.5 block text-xs font-medium text-text-muted";

type TestState =
  | { kind: "idle" }
  | { kind: "testing" }
  | { kind: "ok" }
  | { kind: "error"; message: string };

export default function Settings() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [connecting, setConnecting] = useState(false);

  // --- server / backend ---
  const [serverCfg, setServerCfg] = useState<ServerConfig>(getServerConfig);
  const [serverUrl, setServerUrl] = useState(serverCfg.url);
  const [serverApiKey, setServerApiKey] = useState(serverCfg.apiKey ?? "");
  const [testState, setTestState] = useState<TestState>({ kind: "idle" });
  const [savedFlash, setSavedFlash] = useState(false);

  const draftConfig = useMemo<ServerConfig>(
    () => ({
      mode: serverCfg.mode,
      url: serverUrl.trim(),
      apiKey: serverApiKey.trim() || undefined,
      sessionToken: serverCfg.sessionToken,
    }),
    [serverCfg.mode, serverUrl, serverApiKey, serverCfg.sessionToken],
  );

  // --- server auth (multi-user) ---
  const [authUsername, setAuthUsername] = useState("");
  const [authPassword, setAuthPassword] = useState("");
  const [authBusy, setAuthBusy] = useState(false);
  const [authError, setAuthError] = useState<string | null>(null);
  const [me, setMe] = useState<{
    username: string;
    has_github_token: boolean;
  } | null>(null);
  const [githubToken, setGithubTokenField] = useState("");
  const [tokenSaved, setTokenSaved] = useState(false);

  const isRemoteMode = serverCfg.mode === "remote";
  const hasSession = Boolean(serverCfg.sessionToken);

  const refreshMe = useCallback(async () => {
    if (!serverCfg.sessionToken) {
      setMe(null);
      return;
    }
    try {
      const info = await api.serverGetMe();
      setMe({ username: info.username, has_github_token: info.has_github_token });
    } catch {
      // Token may be stale/invalid; clear the signed-in view but keep the token
      // so the user can decide to log out explicitly.
      setMe(null);
    }
  }, [serverCfg.sessionToken]);

  useEffect(() => {
    void refreshMe();
  }, [refreshMe]);

  async function login() {
    setAuthBusy(true);
    setAuthError(null);
    try {
      const token = await api.serverLogin(
        authUsername.trim(),
        authPassword,
      );
      const next: ServerConfig = { ...serverCfg, sessionToken: token };
      setServerConfig(next);
      setServerCfg(next);
      setAuthPassword("");
      queryClient.invalidateQueries();
    } catch (e) {
      setAuthError(String(e));
    } finally {
      setAuthBusy(false);
    }
  }

  async function register() {
    setAuthBusy(true);
    setAuthError(null);
    try {
      await api.serverRegister(authUsername.trim(), authPassword);
      const token = await api.serverLogin(authUsername.trim(), authPassword);
      const next: ServerConfig = { ...serverCfg, sessionToken: token };
      setServerConfig(next);
      setServerCfg(next);
      setAuthPassword("");
      queryClient.invalidateQueries();
    } catch (e) {
      setAuthError(String(e));
    } finally {
      setAuthBusy(false);
    }
  }

  function logout() {
    const next: ServerConfig = { ...serverCfg, sessionToken: undefined };
    setServerConfig(next);
    setServerCfg(next);
    setMe(null);
    setAuthError(null);
    queryClient.invalidateQueries();
  }

  async function saveGithubToken() {
    setAuthBusy(true);
    setAuthError(null);
    try {
      await api.serverSetGithubToken(githubToken.trim());
      setGithubTokenField("");
      setTokenSaved(true);
      setTimeout(() => setTokenSaved(false), 1500);
      await refreshMe();
    } catch (e) {
      setAuthError(String(e));
    } finally {
      setAuthBusy(false);
    }
  }

  async function testConnection() {
    setTestState({ kind: "testing" });
    try {
      await getGrpcClient(draftConfig).listAccounts({});
      setTestState({ kind: "ok" });
    } catch (e) {
      setTestState({ kind: "error", message: String(e) });
    }
  }

  function saveServer() {
    if (draftConfig.mode === "remote" && draftConfig.url === "") {
      setTestState({ kind: "error", message: t("server_url_required") });
      return;
    }
    setServerConfig(draftConfig);
    setServerCfg(draftConfig);
    setSavedFlash(true);
    setTimeout(() => setSavedFlash(false), 1500);
    // Refetch all queries so the new transport takes effect immediately.
    queryClient.invalidateQueries();
  }

  const canConnect = useMemo(
    () => clientId.trim() !== "" && clientSecret.trim() !== "" && !connecting,
    [clientId, clientSecret, connecting],
  );

  async function load() {
    try {
      setAccounts(await api.listAccounts());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function connect() {
    if (!canConnect) return;
    setConnecting(true);
    setError(null);
    try {
      await api.startGithubLogin(clientId.trim(), clientSecret.trim());
      setClientSecret("");
      await load();
    } catch (e) {
      setError(String(e));
    } finally {
      setConnecting(false);
    }
  }

  async function remove(account: Account) {
    setError(null);
    try {
      await api.deleteAccount(account.id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="mx-auto max-w-2xl px-8 py-10">
      <header className="mb-6">
        <h1 className="text-[22px] font-semibold tracking-tight text-text">
          {t("settings_title")}
        </h1>
      </header>

      {error && (
        <div className="mb-5 flex items-start gap-2 rounded-[--radius] border border-border bg-surface-2 px-3 py-2 text-sm text-text">
          <AlertCircle
            size={15}
            strokeWidth={1.75}
            aria-hidden
            className="mt-0.5 shrink-0 text-closed"
          />
          <span className="break-words">{error}</span>
        </div>
      )}

      <section className="mb-9">
        <h2 className="mb-3 text-[11px] font-medium uppercase tracking-wider text-text-faint">
          {t("server_heading")}
        </h2>
        <div className="space-y-3">
          <div
            role="radiogroup"
            aria-label={t("server_heading")}
            className="flex gap-2"
          >
            <label className="flex items-center gap-2 text-sm text-text">
              <input
                type="radio"
                name="server-mode"
                value="local"
                checked={serverCfg.mode === "local"}
                onChange={() =>
                  setServerCfg((c) => ({ ...c, mode: "local" }))
                }
              />
              {t("server_mode_local")}
            </label>
            <label className="flex items-center gap-2 text-sm text-text">
              <input
                type="radio"
                name="server-mode"
                value="remote"
                checked={serverCfg.mode === "remote"}
                onChange={() =>
                  setServerCfg((c) => ({ ...c, mode: "remote" }))
                }
              />
              {t("server_mode_remote")}
            </label>
          </div>
          <p className="text-xs leading-relaxed text-text-faint">
            {t("server_mode_hint")}
          </p>
          <label className="block">
            <span className={labelClass}>{t("server_url")}</span>
            <input
              className={inputClass}
              value={serverUrl}
              onChange={(e) => setServerUrl(e.target.value)}
              placeholder={t("server_url_placeholder")}
              autoComplete="off"
            />
          </label>
          <label className="block">
            <span className={labelClass}>{t("server_api_key")}</span>
            <input
              type="password"
              className={inputClass}
              value={serverApiKey}
              onChange={(e) => setServerApiKey(e.target.value)}
              placeholder={t("server_api_key_placeholder")}
              autoComplete="off"
            />
          </label>
          <div className="flex items-center gap-2">
            <Button
              variant="secondary"
              onClick={testConnection}
              disabled={testState.kind === "testing"}
            >
              {testState.kind === "testing"
                ? t("server_testing")
                : t("server_test")}
            </Button>
            <Button variant="primary" onClick={saveServer}>
              {savedFlash ? t("server_saved") : t("server_save")}
            </Button>
            {testState.kind === "ok" && (
              <span className="text-xs text-open">{t("server_test_ok")}</span>
            )}
            {testState.kind === "error" && (
              <span className="break-words text-xs text-closed">
                {t("server_test_fail")}: {testState.message}
              </span>
            )}
          </div>

          {isRemoteMode && (
            <div className="space-y-3 rounded-[--radius-lg] border border-border bg-surface px-4 py-4">
              <div>
                <h3 className="text-xs font-medium text-text">
                  {t("server_auth_heading")}
                </h3>
                <p className="mt-1 text-xs leading-relaxed text-text-faint">
                  {t("server_auth_hint")}
                </p>
              </div>

              {hasSession ? (
                <div className="space-y-3">
                  <div className="flex items-center justify-between gap-3">
                    <span className="text-sm text-text">
                      {me
                        ? t("server_signed_in_as", { username: me.username })
                        : t("server_signed_in")}
                    </span>
                    <Button variant="secondary" onClick={logout}>
                      {t("server_logout")}
                    </Button>
                  </div>

                  <label className="block">
                    <span className={labelClass}>{t("server_github_token")}</span>
                    <input
                      type="password"
                      className={inputClass}
                      value={githubToken}
                      onChange={(e) => setGithubTokenField(e.target.value)}
                      placeholder="ghp_…"
                      autoComplete="off"
                    />
                  </label>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="secondary"
                      onClick={saveGithubToken}
                      disabled={authBusy || githubToken.trim() === ""}
                    >
                      {tokenSaved ? t("server_saved") : t("server_save_token")}
                    </Button>
                    {me && (
                      <span
                        className={
                          "text-xs " +
                          (me.has_github_token ? "text-open" : "text-text-faint")
                        }
                      >
                        {me.has_github_token
                          ? `✓ ${t("server_github_token_set")}`
                          : t("server_github_token_unset")}
                      </span>
                    )}
                  </div>
                </div>
              ) : (
                <form
                  className="space-y-3"
                  onSubmit={(e) => {
                    e.preventDefault();
                    login();
                  }}
                >
                  <label className="block">
                    <span className={labelClass}>{t("server_username")}</span>
                    <input
                      className={inputClass}
                      value={authUsername}
                      onChange={(e) => setAuthUsername(e.target.value)}
                      autoComplete="username"
                    />
                  </label>
                  <label className="block">
                    <span className={labelClass}>{t("server_password")}</span>
                    <input
                      type="password"
                      className={inputClass}
                      value={authPassword}
                      onChange={(e) => setAuthPassword(e.target.value)}
                      autoComplete="current-password"
                    />
                  </label>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="primary"
                      type="submit"
                      disabled={
                        authBusy ||
                        authUsername.trim() === "" ||
                        authPassword === ""
                      }
                    >
                      {t("server_login")}
                    </Button>
                    <Button
                      variant="secondary"
                      type="button"
                      onClick={register}
                      disabled={
                        authBusy ||
                        authUsername.trim() === "" ||
                        authPassword === ""
                      }
                    >
                      {t("server_register")}
                    </Button>
                  </div>
                </form>
              )}

              {authError && (
                <p className="break-words text-xs text-closed">{authError}</p>
              )}
            </div>
          )}

          <div className="flex items-center gap-2 pt-1">
            <span className={labelClass + " mb-0"}>{t("sync_status_label")}</span>
            <SyncStatusBadge remote={isRemoteMode} />
          </div>
        </div>
      </section>

      <section className="mb-9">
        <h2 className="mb-3 text-[11px] font-medium uppercase tracking-wider text-text-faint">
          {t("connect_github_heading")}
        </h2>
        <form
          className="space-y-3"
          onSubmit={(e) => {
            e.preventDefault();
            connect();
          }}
        >
          <label className="block">
            <span className={labelClass}>{t("gh_client_id")}</span>
            <input
              className={inputClass}
              value={clientId}
              onChange={(e) => setClientId(e.target.value)}
              autoComplete="off"
            />
          </label>
          <label className="block">
            <span className={labelClass}>{t("gh_client_secret")}</span>
            <input
              type="password"
              className={inputClass}
              value={clientSecret}
              onChange={(e) => setClientSecret(e.target.value)}
              autoComplete="off"
            />
          </label>
          <p className="text-xs leading-relaxed text-text-faint">
            {t("gh_callback_hint")}
          </p>
          <Button variant="primary" type="submit" disabled={!canConnect}>
            {connecting ? t("connecting") : t("connect_github")}
          </Button>
        </form>
      </section>

      <section>
        <h2 className="mb-3 text-[11px] font-medium uppercase tracking-wider text-text-faint">
          {t("accounts_heading")}
        </h2>
        {accounts.length === 0 ? (
          <p className="py-6 text-sm text-text-muted">{t("no_accounts")}</p>
        ) : (
          <ul className="space-y-2">
            {accounts.map((account) => (
              <li
                key={account.id}
                className="rounded-[--radius-lg] border border-border bg-surface px-3.5 py-3"
              >
                <div className="flex items-center gap-3">
                  <CircleUser
                    size={15}
                    strokeWidth={1.75}
                    aria-hidden
                    className="text-text-muted"
                  />
                  <span className="flex-1 truncate text-sm text-text">
                    {account.display_name}
                  </span>
                  <button
                    type="button"
                    aria-label={t("delete_account")}
                    title={t("delete_account")}
                    className={
                      "flex size-8 items-center justify-center rounded-[--radius-sm] text-text-faint " +
                      "transition-colors duration-150 ease-out hover:text-closed " +
                      "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
                      "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none"
                    }
                    onClick={() => remove(account)}
                  >
                    <Trash2 size={15} strokeWidth={1.75} aria-hidden />
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
