import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Trash2, AlertCircle, CircleUser } from "lucide-react";
import { api, type Account } from "../lib/api";
import { Button } from "../components/ui/Button";

const inputClass =
  "w-full rounded-[--radius] border border-border bg-surface-2 px-3 py-2 text-sm text-text " +
  "outline-none transition-[color,border-color] duration-150 ease-out placeholder:text-text-faint " +
  "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

const labelClass = "mb-1.5 block text-xs font-medium text-text-muted";

export default function Settings() {
  const { t } = useTranslation();
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [error, setError] = useState<string | null>(null);

  const [clientId, setClientId] = useState("");
  const [clientSecret, setClientSecret] = useState("");
  const [connecting, setConnecting] = useState(false);

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
