import { Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import type { UseQueryResult } from "@tanstack/react-query";
import { AlertCircle, FolderGit2 } from "lucide-react";
import { useAccounts } from "../../contexts/AccountContext";

interface SectionState<T> {
  data: T[];
  loading: boolean;
  error: string | null;
  /** The display name of the account the data belongs to. */
  account: string | null;
}

/** A query hook bound to the active account id — e.g. `useRepos`. */
type AccountQueryHook<T> = (
  accountId: string | null,
) => UseQueryResult<T[] | undefined>;

/**
 * Adapt an account-scoped react-query hook into the section's loading/error/data
 * shape. Caching, deduping, and stale-while-revalidate are handled by the query
 * client; this just maps `isPending`/`isError`/`data` onto the existing UX and
 * resolves the active account's display name. Never throws.
 */
function useSectionData<T>(
  useHook: AccountQueryHook<T>,
): SectionState<T> & { hasAccounts: boolean } {
  const { accounts, activeAccountId } = useAccounts();
  const account =
    accounts.find((a) => a.id === activeAccountId)?.display_name ??
    activeAccountId ??
    null;

  const query = useHook(activeAccountId);
  // A disabled (no-account) query is `pending` but not `fetching`; only show the
  // loading state when a fetch is actually in flight.
  const loading = query.isPending && query.fetchStatus !== "idle";
  const error = query.isError
    ? query.error instanceof Error
      ? query.error.message
      : String(query.error)
    : null;

  return {
    data: Array.isArray(query.data) ? query.data : [],
    loading,
    error,
    account,
    hasAccounts: accounts.length > 0,
  };
}

export { useSectionData };

export function SectionHeader({
  title,
  account,
}: {
  title: string;
  account: string | null;
}) {
  return (
    <header className="mb-5">
      <h1 className="text-[22px] font-semibold tracking-tight text-text">
        {title}
      </h1>
      {account && (
        <p className="mt-1 font-mono text-[13px] text-text-muted">{account}</p>
      )}
    </header>
  );
}

export function NoAccount() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-start gap-3 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
      <FolderGit2 size={22} strokeWidth={1.75} aria-hidden className="text-text-faint" />
      <p className="max-w-md text-sm text-text-muted">
        {t("dashboard_connect_hint")}
      </p>
      <Link
        to="/settings"
        className={
          "rounded-[--radius] bg-accent px-3.5 py-2 text-sm font-medium text-accent-fg " +
          "transition-[opacity,transform,color,background-color] duration-150 ease-out hover:bg-accent-hover " +
          "outline-none focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 " +
          "focus-visible:ring-offset-canvas motion-reduce:transition-none"
        }
      >
        {t("nav_settings")}
      </Link>
    </div>
  );
}

export function SectionError({ message }: { message: string }) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
      <AlertCircle size={22} strokeWidth={1.75} aria-hidden className="text-closed" />
      <p className="text-sm font-medium text-text">{t("section_error")}</p>
      <p className="max-w-md break-words text-xs text-text-faint">{message}</p>
    </div>
  );
}

export function SectionShell({ children }: { children: React.ReactNode }) {
  return <div className="mx-auto max-w-6xl px-8 py-10">{children}</div>;
}
