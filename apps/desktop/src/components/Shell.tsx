import type { ComponentType } from "react";
import { NavLink, Link } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  ListTodo,
  FolderGit2,
  CircleDot,
  GitPullRequest,
  ListChecks,
  LayoutGrid,
  Inbox,
  Activity,
  TrendingUp,
  Newspaper,
  Settings as SettingsIcon,
  Languages,
  Plus,
  type LucideProps,
} from "lucide-react";
import { useAccounts } from "../contexts/AccountContext";

type NavItem = { to: string; key: string; icon: ComponentType<LucideProps> };

const NAV: NavItem[] = [
  { to: "/", key: "nav_todos", icon: ListTodo },
  { to: "/boards", key: "nav_boards", icon: LayoutGrid },
  { to: "/repos", key: "nav_repos", icon: FolderGit2 },
  { to: "/issues", key: "nav_issues", icon: CircleDot },
  { to: "/pulls", key: "nav_pulls", icon: GitPullRequest },
  { to: "/triage", key: "nav_triage", icon: ListChecks },
  { to: "/inbox", key: "nav_inbox", icon: Inbox },
  { to: "/ci", key: "nav_ci", icon: Activity },
  { to: "/insights", key: "nav_insights", icon: TrendingUp },
  { to: "/digest", key: "nav_digest", icon: Newspaper },
  { to: "/settings", key: "nav_settings", icon: SettingsIcon },
];

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none";

function LogoMark() {
  return (
    <span
      aria-hidden
      className="flex size-7 shrink-0 items-center justify-center rounded-[--radius] bg-accent text-[13px] font-semibold text-accent-fg"
    >
      N
    </span>
  );
}

function AccountArea() {
  const { t } = useTranslation();
  const { accounts, activeAccountId, setActiveAccount } = useAccounts();

  return (
    <div className="mt-auto border-t border-border pt-3">
      <p className="mb-2 px-1 text-[11px] font-medium uppercase tracking-wider text-text-faint">
        {t("accounts_label")}
      </p>
      {accounts.length === 0 ? (
        <Link
          to="/settings"
          className={
            "flex items-center gap-2 rounded-[--radius] border border-dashed border-border " +
            "px-2.5 py-2 text-xs text-text-muted " +
            "transition-[opacity,transform,color,background-color,border-color] duration-150 ease-out " +
            "hover:border-border-strong hover:text-text " +
            focusRing
          }
        >
          <Plus size={14} strokeWidth={1.75} aria-hidden className="shrink-0" />
          {t("connect_account_hint")}
        </Link>
      ) : (
        <ul className="space-y-0.5">
          {accounts.map((account) => {
            const label = account.display_name || account.id;
            const initial = (label.trim()[0] ?? "?").toUpperCase();
            const isActive = account.id === activeAccountId;
            return (
              <li key={account.id}>
                <button
                  type="button"
                  onClick={() => setActiveAccount(account.id)}
                  aria-pressed={isActive}
                  className={
                    "flex w-full items-center gap-2.5 rounded-[--radius] px-2 py-1.5 text-left " +
                    "transition-[opacity,transform,color,background-color,border-color] duration-150 ease-out " +
                    focusRing +
                    " " +
                    (isActive
                      ? "bg-surface-2"
                      : "hover:bg-surface-2")
                  }
                >
                  <span
                    aria-hidden
                    className={
                      "flex size-6 shrink-0 items-center justify-center rounded-full border " +
                      "border-border bg-surface-3 text-[11px] font-semibold " +
                      (isActive ? "text-accent" : "text-text-muted")
                    }
                  >
                    {initial}
                  </span>
                  <span
                    className={
                      "min-w-0 flex-1 truncate text-[13px] " +
                      (isActive ? "text-text" : "text-text-muted")
                    }
                  >
                    {label}
                  </span>
                  {isActive && (
                    <span
                      aria-hidden
                      className="size-1.5 shrink-0 rounded-full bg-accent"
                    />
                  )}
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}

export default function Shell({ children }: { children: React.ReactNode }) {
  const { t, i18n } = useTranslation();

  function toggleLocale() {
    const next = i18n.language === "en" ? "de" : "en";
    i18n.changeLanguage(next);
    try {
      localStorage.setItem("locale", next);
    } catch {
      // ignore storage failures
    }
  }

  return (
    <div className="flex min-h-screen text-text">
      <aside className="sticky top-0 flex h-screen w-60 shrink-0 flex-col gap-5 border-r border-border bg-surface px-3 py-4">
        <div className="flex items-center gap-2.5 px-1">
          <LogoMark />
          <span className="text-[15px] font-semibold tracking-tight text-text">
            {t("app_title")}
          </span>
        </div>

        <nav className="flex flex-col gap-0.5">
          {NAV.map((item) => {
            const Icon = item.icon;
            return (
              <NavLink
                key={item.to}
                to={item.to}
                end={item.to === "/"}
                className={({ isActive }) =>
                  "group relative flex items-center gap-2.5 rounded-[--radius] py-2 pl-3 pr-2 text-[13px] " +
                  "transition-[opacity,transform,color,background-color,border-color] duration-150 ease-out " +
                  focusRing +
                  " " +
                  (isActive
                    ? "bg-surface-2 font-medium text-text"
                    : "text-text-muted hover:bg-surface-2 hover:text-text")
                }
              >
                {({ isActive }) => (
                  <>
                    <span
                      aria-hidden
                      className={
                        "absolute left-0 top-1/2 h-4 w-0.5 -translate-y-1/2 rounded-full bg-accent " +
                        "transition-opacity duration-150 ease-out motion-reduce:transition-none " +
                        (isActive ? "opacity-100" : "opacity-0")
                      }
                    />
                    <Icon
                      size={16}
                      strokeWidth={1.75}
                      aria-hidden
                      className={isActive ? "text-accent" : "text-text-muted"}
                    />
                    {t(item.key)}
                  </>
                )}
              </NavLink>
            );
          })}
        </nav>

        <AccountArea />

        <button
          type="button"
          onClick={toggleLocale}
          aria-label={t("toggle_language")}
          title={t("toggle_language")}
          className={
            "flex items-center gap-2 rounded-[--radius] border border-border px-2.5 py-2 text-xs text-text-muted " +
            "transition-[opacity,transform,color,background-color,border-color] duration-150 ease-out " +
            "hover:border-border-strong hover:text-text " +
            focusRing
          }
        >
          <Languages size={14} strokeWidth={1.75} aria-hidden />
          <span className="uppercase tracking-wide tnum">{i18n.language}</span>
        </button>
      </aside>

      <main className="min-w-0 flex-1 overflow-y-auto">{children}</main>
    </div>
  );
}
