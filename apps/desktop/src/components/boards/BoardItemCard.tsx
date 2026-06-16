import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CircleDot,
  GitPullRequest,
  ListTodo,
  MoreHorizontal,
  ExternalLink,
  X,
} from "lucide-react";
import type { BoardItem } from "../../lib/boards/resolve";
import type { BoardColumn } from "../../lib/api";
import { formatRelativeTime, labelChipStyle } from "../../lib/format";
import { issueDetailPath, pullDetailPath } from "../../lib/itemRoute";
import { openItem, MAX_LABELS } from "../dashboard/listRow";

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none";

function KindGlyph({ item }: { item: BoardItem }) {
  if (item.kind === "pr") {
    return (
      <span className="flex items-center gap-1.5">
        <span
          aria-hidden
          className={
            "size-2 shrink-0 rounded-full " +
            (item.state === "open" ? "bg-open" : "bg-closed")
          }
        />
        <GitPullRequest
          size={13}
          strokeWidth={1.75}
          aria-hidden
          className={item.state === "open" ? "text-open" : "text-closed"}
        />
      </span>
    );
  }
  if (item.kind === "issue") {
    return (
      <span className="flex items-center gap-1.5">
        <span
          aria-hidden
          className={
            "size-2 shrink-0 rounded-full " +
            (item.state === "open" ? "bg-open" : "bg-closed")
          }
        />
        <CircleDot
          size={13}
          strokeWidth={1.75}
          aria-hidden
          className={item.state === "open" ? "text-open" : "text-closed"}
        />
      </span>
    );
  }
  return (
    <span className="flex items-center gap-1.5">
      <span
        aria-hidden
        className={
          "size-2 shrink-0 rounded-full " +
          (item.state === "open" ? "bg-warn" : "bg-text-faint")
        }
      />
      <ListTodo size={13} strokeWidth={1.75} aria-hidden className="text-text-muted" />
    </span>
  );
}

export interface BoardItemCardProps {
  item: BoardItem;
  /** Columns the card can be moved into (excludes its current column). */
  columns: BoardColumn[];
  onMove: (columnId: string) => void;
  onRemove: () => void;
  onDragStart: () => void;
  onDragEnd: () => void;
}

export function BoardItemCard({
  item,
  columns,
  onMove,
  onRemove,
  onDragStart,
  onDragEnd,
}: BoardItemCardProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const [menuOpen, setMenuOpen] = useState(false);
  const labels = item.labels || [];
  const clickable = Boolean(item.html_url);
  // GitHub issues/PRs open the in-app detail; todos (and anything without a
  // number) fall back to opening their source url in the browser.
  const itemNumber = (item.raw as { number?: number }).number;
  const detailPath =
    item.repo && itemNumber != null
      ? item.kind === "pr"
        ? pullDetailPath(item.repo, itemNumber)
        : item.kind === "issue"
          ? issueDetailPath(item.repo, itemNumber)
          : null
      : null;

  function openItemTarget() {
    if (detailPath) navigate(detailPath);
    else if (item.html_url) openItem(item.html_url);
  }

  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData("text/plain", item.key);
        e.dataTransfer.effectAllowed = "move";
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      className={
        "group/card relative flex cursor-grab flex-col gap-2 rounded-[--radius] border border-border bg-surface-2 p-3 " +
        "transition-[transform,color,background-color,border-color] duration-150 ease-out " +
        "hover:border-border-strong active:cursor-grabbing motion-reduce:transition-none"
      }
    >
      <div className="flex items-center gap-2 text-[11px]">
        <KindGlyph item={item} />
        {item.repo && (
          <span className="min-w-0 truncate font-mono text-text-muted" title={item.repo}>
            {item.repo}
          </span>
        )}
        {item.kind !== "todo" && (
          <span className="tnum shrink-0 font-mono text-text-faint">
            #{(item.raw as { number?: number }).number}
          </span>
        )}
        <span className="ml-auto shrink-0 whitespace-nowrap text-text-faint">
          {formatRelativeTime(item.updated_at, Date.now(), i18n.language)}
        </span>
      </div>

      {clickable ? (
        <button
          type="button"
          onClick={openItemTarget}
          className={"text-left " + focusRing}
        >
          <span className="line-clamp-3 text-[13px] font-medium leading-snug text-text">
            {item.title}
          </span>
        </button>
      ) : (
        <span className="line-clamp-3 text-[13px] font-medium leading-snug text-text">
          {item.title}
        </span>
      )}

      {labels.length > 0 && (
        <div className="flex flex-wrap items-center gap-1.5">
          {labels.slice(0, MAX_LABELS).map((name) => {
            const style = labelChipStyle("ededed");
            return (
              <span
                key={name}
                className="inline-flex items-center gap-1 rounded-full border px-1.5 py-0.5 text-[10px] text-text-muted"
                style={{ background: style.background, borderColor: style.borderColor }}
              >
                <span
                  aria-hidden
                  className="size-1.5 shrink-0 rounded-full"
                  style={{ background: style.dot }}
                />
                {name}
              </span>
            );
          })}
          {labels.length > MAX_LABELS && (
            <span className="tnum text-[10px] text-text-faint">
              +{labels.length - MAX_LABELS}
            </span>
          )}
        </div>
      )}

      {/* Card menu */}
      <div className="absolute right-2 top-2">
        <button
          type="button"
          aria-label={t("move_to_column")}
          onClick={() => setMenuOpen((o) => !o)}
          className={
            "rounded-[--radius-sm] p-1 text-text-faint opacity-0 transition-opacity duration-150 " +
            "hover:bg-surface-3 hover:text-text group-hover/card:opacity-100 " +
            (menuOpen ? "opacity-100 " : "") +
            focusRing
          }
        >
          <MoreHorizontal size={14} strokeWidth={1.75} aria-hidden />
        </button>
        {menuOpen && (
          <>
            <div
              className="fixed inset-0 z-10"
              aria-hidden
              onClick={() => setMenuOpen(false)}
            />
            <div className="absolute right-0 top-7 z-20 w-48 overflow-hidden rounded-[--radius] border border-border bg-surface shadow-lg">
              {clickable && (
                <button
                  type="button"
                  onClick={() => {
                    openItem(item.html_url as string);
                    setMenuOpen(false);
                  }}
                  className="flex w-full items-center gap-2 px-3 py-2 text-left text-[12px] text-text-muted hover:bg-surface-2 hover:text-text"
                >
                  <ExternalLink size={13} strokeWidth={1.75} aria-hidden />
                  {t("card_open_link")}
                </button>
              )}
              {columns.length > 0 && (
                <div className="border-t border-border px-3 pb-1 pt-2 text-[10px] font-medium uppercase tracking-wide text-text-faint">
                  {t("move_to_column")}
                </div>
              )}
              {columns.map((col) => (
                <button
                  key={col.id}
                  type="button"
                  onClick={() => {
                    onMove(col.id);
                    setMenuOpen(false);
                  }}
                  className="flex w-full items-center px-3 py-1.5 text-left text-[12px] text-text-muted hover:bg-surface-2 hover:text-text"
                >
                  <span className="truncate">{col.name}</span>
                </button>
              ))}
              <button
                type="button"
                onClick={() => {
                  onRemove();
                  setMenuOpen(false);
                }}
                className="flex w-full items-center gap-2 border-t border-border px-3 py-2 text-left text-[12px] text-text-muted hover:bg-surface-2 hover:text-closed"
              >
                <X size={13} strokeWidth={1.75} aria-hidden />
                {t("remove_from_board")}
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export default BoardItemCard;
