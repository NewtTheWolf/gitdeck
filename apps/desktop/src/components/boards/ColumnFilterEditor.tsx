import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Search } from "lucide-react";
import type { SmartFilter } from "../../lib/api";

const STATES: NonNullable<SmartFilter["state"]>[] = ["open", "closed", "all"];
const TYPES: NonNullable<SmartFilter["item_types"]>[number][] = [
  "issue",
  "pr",
  "todo",
];

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none";

function toggleArray<T>(arr: T[] | undefined, value: T): T[] | undefined {
  const set = new Set(arr ?? []);
  if (set.has(value)) set.delete(value);
  else set.add(value);
  const next = Array.from(set);
  return next.length > 0 ? next : undefined;
}

export interface ColumnFilterEditorProps {
  filter: SmartFilter;
  repos: string[];
  labels: string[];
  onApply: (filter: SmartFilter) => void;
  onClose: () => void;
}

export function ColumnFilterEditor({
  filter,
  repos,
  labels,
  onApply,
  onClose,
}: ColumnFilterEditorProps) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<SmartFilter>(() => ({ ...filter }));

  const state = draft.state ?? "open";
  const types = draft.item_types ?? [];
  const selRepos = draft.repos ?? [];
  const selLabels = draft.labels ?? [];

  function set<K extends keyof SmartFilter>(key: K, value: SmartFilter[K]) {
    setDraft((d) => {
      const next = { ...d };
      if (value === undefined) delete next[key];
      else next[key] = value;
      return next;
    });
  }

  function typeLabel(k: "issue" | "pr" | "todo") {
    return k === "issue" ? t("type_issue") : k === "pr" ? t("type_pr") : t("type_todo");
  }

  function stateLabel(s: "open" | "closed" | "all") {
    return s === "open" ? t("filter_open") : s === "closed" ? t("filter_closed") : t("filter_all");
  }

  return (
    <div className="flex w-[18rem] flex-col gap-3 rounded-[--radius] border border-border bg-surface p-3 text-[12px] shadow-lg">
      {/* State */}
      <div className="flex flex-col gap-1.5">
        <span className="text-[10px] font-medium uppercase tracking-wide text-text-faint">
          {t("filter_state_label")}
        </span>
        <div
          role="group"
          className="flex gap-0.5 rounded-[--radius] border border-border bg-surface-2 p-0.5"
        >
          {STATES.map((s) => (
            <button
              key={s}
              type="button"
              aria-pressed={state === s}
              onClick={() => set("state", s === "open" ? undefined : s)}
              className={
                "flex-1 rounded-[--radius-sm] px-2 py-1 text-[12px] transition-colors duration-150 " +
                focusRing +
                " " +
                (state === s
                  ? "bg-surface-3 font-medium text-text"
                  : "text-text-muted hover:text-text")
              }
            >
              {stateLabel(s)}
            </button>
          ))}
        </div>
      </div>

      {/* Types */}
      <div className="flex flex-col gap-1.5">
        <span className="text-[10px] font-medium uppercase tracking-wide text-text-faint">
          {t("filter_types")}
        </span>
        <div className="flex gap-1.5">
          {TYPES.map((k) => {
            const active = types.includes(k);
            return (
              <button
                key={k}
                type="button"
                aria-pressed={active}
                onClick={() => set("item_types", toggleArray(draft.item_types, k))}
                className={
                  "flex-1 rounded-[--radius] border px-2 py-1 text-[12px] transition-colors duration-150 " +
                  focusRing +
                  " " +
                  (active
                    ? "border-accent bg-accent/15 text-text"
                    : "border-border bg-surface-2 text-text-muted hover:text-text")
                }
              >
                {typeLabel(k)}
              </button>
            );
          })}
        </div>
      </div>

      {/* Text */}
      <div className="flex flex-col gap-1.5">
        <span className="text-[10px] font-medium uppercase tracking-wide text-text-faint">
          {t("filter_text")}
        </span>
        <div className="relative">
          <Search
            size={13}
            strokeWidth={1.75}
            aria-hidden
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-text-faint"
          />
          <input
            type="search"
            value={draft.text ?? ""}
            onChange={(e) => set("text", e.target.value || undefined)}
            placeholder={t("board_filter_text_placeholder")}
            className={
              "w-full rounded-[--radius] border border-border bg-surface-2 py-1.5 pl-8 pr-2.5 text-[12px] text-text " +
              "placeholder:text-text-faint focus-visible:border-accent " +
              focusRing
            }
          />
        </div>
      </div>

      {/* Repos */}
      {repos.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <span className="text-[10px] font-medium uppercase tracking-wide text-text-faint">
            {t("filter_repos")}
          </span>
          <div className="flex max-h-28 flex-col gap-0.5 overflow-y-auto">
            {repos.map((repo) => {
              const active = selRepos.includes(repo);
              return (
                <label
                  key={repo}
                  className="flex cursor-pointer items-center gap-2 rounded-[--radius-sm] px-1.5 py-1 hover:bg-surface-2"
                >
                  <input
                    type="checkbox"
                    checked={active}
                    onChange={() => set("repos", toggleArray(draft.repos, repo))}
                    className="size-3.5 accent-accent"
                  />
                  <span className="min-w-0 truncate font-mono text-text-muted">{repo}</span>
                </label>
              );
            })}
          </div>
        </div>
      )}

      {/* Labels */}
      {labels.length > 0 && (
        <div className="flex flex-col gap-1.5">
          <span className="text-[10px] font-medium uppercase tracking-wide text-text-faint">
            {t("filter_labels")}
          </span>
          <div className="flex max-h-28 flex-col gap-0.5 overflow-y-auto">
            {labels.map((label) => {
              const active = selLabels.some(
                (l) => l.toLowerCase() === label.toLowerCase(),
              );
              return (
                <label
                  key={label}
                  className="flex cursor-pointer items-center gap-2 rounded-[--radius-sm] px-1.5 py-1 hover:bg-surface-2"
                >
                  <input
                    type="checkbox"
                    checked={active}
                    onChange={() => set("labels", toggleArray(draft.labels, label))}
                    className="size-3.5 accent-accent"
                  />
                  <span className="min-w-0 truncate text-text-muted">{label}</span>
                </label>
              );
            })}
          </div>
        </div>
      )}

      <div className="flex items-center justify-between gap-2 border-t border-border pt-2.5">
        <button
          type="button"
          onClick={() => setDraft({})}
          className={
            "rounded-[--radius] px-2 py-1 text-[12px] text-text-muted hover:text-text " + focusRing
          }
        >
          {t("filter_clear")}
        </button>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onClose}
            className={
              "rounded-[--radius] px-2 py-1 text-[12px] text-text-muted hover:text-text " + focusRing
            }
          >
            {t("board_done")}
          </button>
          <button
            type="button"
            onClick={() => onApply(draft)}
            className={
              "rounded-[--radius] bg-accent px-3 py-1 text-[12px] font-medium text-accent-fg hover:bg-accent-hover " +
              focusRing
            }
          >
            {t("filter_apply")}
          </button>
        </div>
      </div>
    </div>
  );
}

export default ColumnFilterEditor;
