import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import type {
  DashboardFilters,
  SortKey,
  StateFilter,
} from "../../lib/dashboard";

const STATES: StateFilter[] = ["all", "open", "closed"];
const SORTS: SortKey[] = ["updated", "created"];

const selectClass =
  "rounded-[--radius] border border-border bg-surface-2 px-2.5 py-1.5 text-[13px] text-text " +
  "outline-none transition-[color,border-color] duration-150 ease-out " +
  "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

interface FilterBarProps {
  filters: DashboardFilters;
  onChange: (next: DashboardFilters) => void;
  /** Distinct `owner/repo` paths to populate the repo dropdown. */
  repos: string[];
}

export function FilterBar({ filters, onChange, repos }: FilterBarProps) {
  const { t } = useTranslation();

  function set<K extends keyof DashboardFilters>(
    key: K,
    value: DashboardFilters[K],
  ) {
    onChange({ ...filters, [key]: value });
  }

  function stateLabel(s: StateFilter) {
    return s === "all"
      ? t("filter_all")
      : s === "open"
        ? t("filter_open")
        : t("filter_closed");
  }

  return (
    <div className="mb-4 flex flex-wrap items-center gap-2">
      {/* State segmented control */}
      <div
        role="group"
        aria-label={t("filter_state")}
        className="flex gap-0.5 rounded-[--radius] border border-border bg-surface p-0.5"
      >
        {STATES.map((s) => (
          <button
            key={s}
            type="button"
            aria-pressed={filters.state === s}
            onClick={() => set("state", s)}
            className={
              "rounded-[--radius-sm] px-2.5 py-1 text-[13px] " +
              "transition-[color,background-color] duration-150 ease-out " +
              focusRing +
              " " +
              (filters.state === s
                ? "bg-surface-2 font-medium text-text"
                : "text-text-muted hover:text-text")
            }
          >
            {stateLabel(s)}
          </button>
        ))}
      </div>

      {/* Free-text search */}
      <div className="relative min-w-[12rem] flex-1">
        <Search
          size={14}
          strokeWidth={1.75}
          aria-hidden
          className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-text-faint"
        />
        <input
          type="search"
          value={filters.search}
          onChange={(e) => set("search", e.target.value)}
          placeholder={t("search_placeholder")}
          className={
            "w-full rounded-[--radius] border border-border bg-surface-2 py-1.5 pl-8 pr-2.5 text-[13px] text-text " +
            "outline-none transition-[color,border-color] duration-150 ease-out placeholder:text-text-faint " +
            "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
            "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none"
          }
        />
      </div>

      {/* Repo filter */}
      <select
        aria-label={t("all_repos")}
        value={filters.repo}
        onChange={(e) => set("repo", e.target.value)}
        className={selectClass + " max-w-[14rem] font-mono"}
      >
        <option value="">{t("all_repos")}</option>
        {repos.map((repo) => (
          <option key={repo} value={repo}>
            {repo}
          </option>
        ))}
      </select>

      {/* Sort */}
      <select
        aria-label={t("sort_label")}
        value={filters.sort}
        onChange={(e) => set("sort", e.target.value as SortKey)}
        className={selectClass}
      >
        {SORTS.map((s) => (
          <option key={s} value={s}>
            {s === "updated" ? t("sort_updated") : t("sort_created")}
          </option>
        ))}
      </select>
    </div>
  );
}

export default FilterBar;
