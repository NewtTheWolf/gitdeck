import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ComponentType,
} from "react";
import { createPortal } from "react-dom";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  ListTodo,
  FolderGit2,
  CircleDot,
  GitPullRequest,
  LayoutGrid,
  Inbox,
  Activity,
  Settings as SettingsIcon,
  Languages,
  Plus,
  CornerDownLeft,
  ArrowUp,
  ArrowDown,
  Search,
  type LucideProps,
} from "lucide-react";
import { useAccounts } from "../../contexts/AccountContext";
import { useCommandPalette } from "../../contexts/CommandPaletteContext";
import { api, type Issue, type PullRequest, type Repo } from "../../lib/api";
import { repoDetailPath } from "../../lib/repoRoute";
import { issueDetailPath, pullDetailPath } from "../../lib/itemRoute";

type Icon = ComponentType<LucideProps>;

interface CommandItem {
  id: string;
  /** Primary label used for filtering + display. */
  label: string;
  /** Optional mono secondary text (repo full_name, #number, etc.). */
  hint?: string;
  /** Render the primary label in the mono font (used for repo full_names). */
  mono?: boolean;
  icon: Icon;
  /** Extra haystack text for filtering (e.g. repo for an issue). */
  searchText?: string;
  run: () => void;
}

interface CommandGroup {
  id: string;
  title: string;
  items: CommandItem[];
}

const NAV_ITEMS: { to: string; key: string; icon: Icon }[] = [
  { to: "/", key: "nav_todos", icon: ListTodo },
  { to: "/boards", key: "nav_boards", icon: LayoutGrid },
  { to: "/repos", key: "nav_repos", icon: FolderGit2 },
  { to: "/issues", key: "nav_issues", icon: CircleDot },
  { to: "/pulls", key: "nav_pulls", icon: GitPullRequest },
  { to: "/inbox", key: "nav_inbox", icon: Inbox },
  { to: "/ci", key: "nav_ci", icon: Activity },
  { to: "/settings", key: "nav_settings", icon: SettingsIcon },
];

function matches(item: CommandItem, q: string): boolean {
  if (!q) return true;
  const hay = `${item.label} ${item.hint ?? ""} ${item.searchText ?? ""}`.toLowerCase();
  return hay.includes(q);
}

/** Session-lived cache of the dynamic groups, fetched on first open. */
interface DynamicData {
  repos: Repo[];
  issues: Issue[];
  pulls: PullRequest[];
}

export default function CommandPalette() {
  const { open, setOpen } = useCommandPalette();
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const { activeAccountId } = useAccounts();

  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [data, setData] = useState<DynamicData | null>(null);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  /** Avoids re-fetching the dynamic groups on every open within a session. */
  const fetchedFor = useRef<string | null>(null);

  const close = useCallback(() => setOpen(false), [setOpen]);

  function activate(item: CommandItem) {
    close();
    item.run();
  }

  function toggleLanguage() {
    const next = i18n.language === "en" ? "de" : "en";
    i18n.changeLanguage(next);
    try {
      localStorage.setItem("locale", next);
    } catch {
      // ignore storage failures
    }
  }

  // Reset transient state and focus the input each time the palette opens.
  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(0);
    const id = requestAnimationFrame(() => inputRef.current?.focus());
    return () => cancelAnimationFrame(id);
  }, [open]);

  // Lazy-load repos/issues/pulls on FIRST open per account; cache for session.
  useEffect(() => {
    if (!open || !activeAccountId) return;
    if (fetchedFor.current === activeAccountId) return;
    fetchedFor.current = activeAccountId;
    let cancelled = false;
    setLoading(true);
    Promise.all([
      api.listRepos(activeAccountId).catch(() => [] as Repo[]),
      api.listIssues(activeAccountId).catch(() => [] as Issue[]),
      api.listPullRequests(activeAccountId).catch(() => [] as PullRequest[]),
    ])
      .then(([repos, issues, pulls]) => {
        if (!cancelled) setData({ repos, issues, pulls });
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [open, activeAccountId]);

  const q = query.trim().toLowerCase();

  const groups = useMemo<CommandGroup[]>(() => {
    const navItems: CommandItem[] = NAV_ITEMS.map((n) => ({
      id: `nav:${n.to}`,
      label: t(n.key),
      icon: n.icon,
      run: () => navigate(n.to),
    }));

    const actionItems: CommandItem[] = [
      {
        id: "action:new-todo",
        label: t("cmdk_action_new_todo"),
        icon: Plus,
        run: () => navigate("/"),
      },
      {
        id: "action:new-board",
        label: t("cmdk_action_new_board"),
        icon: Plus,
        run: () => navigate("/boards"),
      },
      {
        id: "action:toggle-language",
        label: t("cmdk_action_toggle_language"),
        icon: Languages,
        run: toggleLanguage,
      },
      {
        id: "action:settings",
        label: t("cmdk_action_settings"),
        icon: SettingsIcon,
        run: () => navigate("/settings"),
      },
    ];

    const repoItems: CommandItem[] = (data?.repos ?? []).map((r) => ({
      id: `repo:${r.id}`,
      label: r.full_name,
      mono: true,
      icon: FolderGit2,
      run: () => navigate(repoDetailPath(r.full_name)),
    }));

    const issueItems: CommandItem[] = (data?.issues ?? []).map((it) => ({
      id: `issue:${it.repo_name_with_owner}#${it.number}`,
      label: it.title,
      hint: `${it.repo_name_with_owner} #${it.number}`,
      searchText: it.repo_name_with_owner,
      icon: CircleDot,
      run: () =>
        navigate(issueDetailPath(it.repo_name_with_owner, it.number)),
    }));

    const pullItems: CommandItem[] = (data?.pulls ?? []).map((pr) => ({
      id: `pull:${pr.repo_name_with_owner}#${pr.number}`,
      label: pr.title,
      hint: `${pr.repo_name_with_owner} #${pr.number}`,
      searchText: pr.repo_name_with_owner,
      icon: GitPullRequest,
      run: () =>
        navigate(pullDetailPath(pr.repo_name_with_owner, pr.number)),
    }));

    const built: CommandGroup[] = [
      { id: "navigation", title: t("cmdk_group_navigation"), items: navItems },
      { id: "actions", title: t("cmdk_group_actions"), items: actionItems },
      { id: "repos", title: t("cmdk_group_repos"), items: repoItems },
      { id: "issues", title: t("cmdk_group_issues"), items: issueItems },
      { id: "pulls", title: t("cmdk_group_pulls"), items: pullItems },
    ];

    return built
      .map((g) => ({ ...g, items: g.items.filter((it) => matches(it, q)) }))
      .filter((g) => g.items.length > 0);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [t, navigate, i18n.language, data, q]);

  // Flat, keyboard-navigable index across all (visible) groups.
  const flat = useMemo(() => groups.flatMap((g) => g.items), [groups]);

  // Keep the selection valid + snapped to the first result as results change.
  useEffect(() => {
    setActive((prev) => (prev >= flat.length ? 0 : prev));
  }, [flat.length]);

  useEffect(() => {
    setActive(0);
  }, [q]);

  // Scroll the active row into view as selection moves.
  useEffect(() => {
    const el = listRef.current?.querySelector<HTMLElement>(
      `[data-index="${active}"]`,
    );
    el?.scrollIntoView({ block: "nearest" });
  }, [active]);

  function onInputKeyDown(e: React.KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setActive((i) => (flat.length ? (i + 1) % flat.length : 0));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setActive((i) => (flat.length ? (i - 1 + flat.length) % flat.length : 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = flat[active];
      if (item) activate(item);
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  if (!open) return null;

  let runningIndex = -1;

  return createPortal(
    <div
      className="fixed inset-0 z-50 flex items-start justify-center px-4 pt-[12vh]"
      role="dialog"
      aria-modal="true"
      aria-label={t("cmdk_placeholder")}
    >
      {/* Backdrop — click to close. */}
      <button
        type="button"
        aria-hidden
        tabIndex={-1}
        onClick={close}
        className="fixed inset-0 cursor-default bg-canvas/70"
      />

      <div
        className="relative z-10 w-full max-w-lg overflow-hidden rounded-[--radius-lg] border border-border-strong bg-surface shadow-[--shadow-overlay]"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center gap-2.5 border-b border-border px-3.5">
          <Search
            size={16}
            strokeWidth={1.75}
            aria-hidden
            className="shrink-0 text-text-faint"
          />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={onInputKeyDown}
            placeholder={t("cmdk_placeholder")}
            aria-label={t("cmdk_placeholder")}
            spellCheck={false}
            autoComplete="off"
            className="w-full bg-transparent py-3 text-[14px] text-text outline-none placeholder:text-text-faint"
          />
        </div>

        <div
          ref={listRef}
          className="max-h-[min(60vh,24rem)] overflow-y-auto py-1.5"
        >
          {loading && groups.length === 0 && (
            <p className="px-3.5 py-3 text-[13px] text-text-faint">
              {t("cmdk_loading")}
            </p>
          )}

          {!loading && flat.length === 0 && (
            <p className="px-3.5 py-3 text-[13px] text-text-faint">
              {t("cmdk_empty")}
            </p>
          )}

          {groups.map((group) => (
            <div key={group.id} className="px-1.5 pb-1">
              <p className="px-2 pb-1 pt-2 text-[11px] font-medium uppercase tracking-wider text-text-faint">
                {group.title}
              </p>
              <ul>
                {group.items.map((item) => {
                  runningIndex += 1;
                  const index = runningIndex;
                  const isActive = index === active;
                  const ItemIcon = item.icon;
                  return (
                    <li key={item.id}>
                      <button
                        type="button"
                        data-index={index}
                        onMouseMove={() => setActive(index)}
                        onClick={() => activate(item)}
                        aria-selected={isActive}
                        className={
                          "flex w-full items-center gap-2.5 rounded-[--radius] px-2 py-1.5 text-left " +
                          "outline-none motion-reduce:transition-none " +
                          (isActive ? "bg-surface-2" : "")
                        }
                      >
                        <ItemIcon
                          size={15}
                          strokeWidth={1.75}
                          aria-hidden
                          className={
                            "shrink-0 " +
                            (isActive ? "text-accent" : "text-text-muted")
                          }
                        />
                        <span
                          className={
                            "min-w-0 flex-1 truncate text-[13px] " +
                            (item.mono ? "font-mono " : "") +
                            (isActive ? "text-text" : "text-text-muted")
                          }
                        >
                          {item.label}
                        </span>
                        {item.hint && (
                          <span className="shrink-0 truncate font-mono text-[11px] tnum text-text-faint">
                            {item.hint}
                          </span>
                        )}
                      </button>
                    </li>
                  );
                })}
              </ul>
            </div>
          ))}
        </div>

        <div className="flex items-center gap-3 border-t border-border px-3.5 py-2 text-[11px] text-text-faint">
          <span className="flex items-center gap-1">
            <ArrowUp size={11} strokeWidth={2} aria-hidden />
            <ArrowDown size={11} strokeWidth={2} aria-hidden />
            {t("cmdk_hint_navigate")}
          </span>
          <span className="flex items-center gap-1">
            <CornerDownLeft size={11} strokeWidth={2} aria-hidden />
            {t("cmdk_hint_open")}
          </span>
          <span className="ml-auto flex items-center gap-1">
            <kbd className="rounded-sm border border-border px-1 font-sans text-[10px] uppercase">
              esc
            </kbd>
            {t("cmdk_hint_close")}
          </span>
        </div>
      </div>
    </div>,
    document.body,
  );
}
