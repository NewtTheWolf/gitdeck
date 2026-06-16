import { useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import {
  ChevronLeft,
  Plus,
  MoreHorizontal,
  SlidersHorizontal,
  Pencil,
  Trash2,
  AlertCircle,
} from "lucide-react";
import {
  api,
  type BoardColumn,
  type SmartFilter,
  type TaskDto,
} from "../../lib/api";
import { useAccounts } from "../../contexts/AccountContext";
import {
  queryKeys,
  useBoard,
  useIssues,
  usePulls,
  useTasks,
} from "../../lib/query/queries";
import {
  buildPool,
  resolveBoard,
  itemKey,
  type BoardItem,
} from "../../lib/boards/resolve";
import { Skeleton } from "../../components/ui/Skeleton";
import { Button } from "../../components/ui/Button";
import { BoardItemCard } from "../../components/boards/BoardItemCard";
import { ColumnFilterEditor } from "../../components/boards/ColumnFilterEditor";

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none";

function distinct(values: (string | undefined)[]): string[] {
  return Array.from(new Set(values.filter((v): v is string => !!v))).sort();
}

interface ColumnViewProps {
  column: BoardColumn;
  items: BoardItem[];
  allColumns: BoardColumn[];
  repos: string[];
  labels: string[];
  onUpdateColumn: (id: string, name: string, filter: SmartFilter) => void;
  onDeleteColumn: (id: string) => void;
  onRename: (id: string, name: string) => void;
  onMoveCard: (itemKey: string, columnId: string) => void;
  onRemoveCard: (itemKey: string) => void;
  onDropToColumn: (columnId: string) => void;
  onDragStart: (key: string) => void;
  onDragEnd: () => void;
}

function ColumnView({
  column,
  items,
  allColumns,
  repos,
  labels,
  onUpdateColumn,
  onDeleteColumn,
  onRename,
  onMoveCard,
  onRemoveCard,
  onDropToColumn,
  onDragStart,
  onDragEnd,
}: ColumnViewProps) {
  const { t } = useTranslation();
  const [menuOpen, setMenuOpen] = useState(false);
  const [filterOpen, setFilterOpen] = useState(false);
  const [editingName, setEditingName] = useState(false);
  const [nameDraft, setNameDraft] = useState(column.name);
  const [dragOver, setDragOver] = useState(false);

  const otherColumns = allColumns.filter((c) => c.id !== column.id);

  function commitName() {
    const next = nameDraft.trim();
    setEditingName(false);
    if (next && next !== column.name) onRename(column.id, next);
    else setNameDraft(column.name);
  }

  return (
    <div
      data-testid={`column-${column.id}`}
      onDragOver={(e) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        if (!dragOver) setDragOver(true);
      }}
      onDragLeave={() => setDragOver(false)}
      onDrop={(e) => {
        e.preventDefault();
        setDragOver(false);
        onDropToColumn(column.id);
      }}
      className={
        "flex w-72 shrink-0 flex-col rounded-[--radius-lg] border bg-surface " +
        "transition-colors duration-150 " +
        (dragOver ? "border-accent" : "border-border")
      }
    >
      {/* Header */}
      <div className="relative flex items-center gap-2 border-b border-border px-3 py-2.5">
        {editingName ? (
          <input
            autoFocus
            value={nameDraft}
            onChange={(e) => setNameDraft(e.target.value)}
            onBlur={commitName}
            onKeyDown={(e) => {
              if (e.key === "Enter") commitName();
              if (e.key === "Escape") {
                setNameDraft(column.name);
                setEditingName(false);
              }
            }}
            className={
              "min-w-0 flex-1 rounded-[--radius-sm] border border-border bg-surface-2 px-1.5 py-0.5 text-[13px] font-medium text-text " +
              focusRing
            }
          />
        ) : (
          <button
            type="button"
            onClick={() => {
              setNameDraft(column.name);
              setEditingName(true);
            }}
            className={"min-w-0 flex-1 truncate text-left text-[13px] font-medium text-text " + focusRing}
            title={t("column_rename")}
          >
            {column.name}
          </button>
        )}
        <span className="tnum shrink-0 rounded-full bg-surface-2 px-1.5 py-0.5 text-[11px] text-text-faint">
          {items.length}
        </span>
        <button
          type="button"
          aria-label={t("column_edit_filter")}
          title={t("column_edit_filter")}
          onClick={() => {
            setFilterOpen((o) => !o);
            setMenuOpen(false);
          }}
          className={
            "rounded-[--radius-sm] p-1 text-text-faint hover:bg-surface-2 hover:text-text " +
            (filterOpen ? "bg-surface-2 text-text " : "") +
            focusRing
          }
        >
          <SlidersHorizontal size={14} strokeWidth={1.75} aria-hidden />
        </button>
        <button
          type="button"
          aria-label={t("column_rename")}
          onClick={() => setMenuOpen((o) => !o)}
          className={
            "rounded-[--radius-sm] p-1 text-text-faint hover:bg-surface-2 hover:text-text " + focusRing
          }
        >
          <MoreHorizontal size={14} strokeWidth={1.75} aria-hidden />
        </button>

        {menuOpen && (
          <>
            <div className="fixed inset-0 z-10" aria-hidden onClick={() => setMenuOpen(false)} />
            <div className="absolute right-2 top-10 z-20 w-44 overflow-hidden rounded-[--radius] border border-border bg-surface shadow-lg">
              <button
                type="button"
                onClick={() => {
                  setNameDraft(column.name);
                  setEditingName(true);
                  setMenuOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-2 text-left text-[12px] text-text-muted hover:bg-surface-2 hover:text-text"
              >
                <Pencil size={13} strokeWidth={1.75} aria-hidden />
                {t("column_rename")}
              </button>
              <button
                type="button"
                onClick={() => {
                  setFilterOpen(true);
                  setMenuOpen(false);
                }}
                className="flex w-full items-center gap-2 px-3 py-2 text-left text-[12px] text-text-muted hover:bg-surface-2 hover:text-text"
              >
                <SlidersHorizontal size={13} strokeWidth={1.75} aria-hidden />
                {t("column_edit_filter")}
              </button>
              <button
                type="button"
                onClick={() => {
                  setMenuOpen(false);
                  onDeleteColumn(column.id);
                }}
                className="flex w-full items-center gap-2 border-t border-border px-3 py-2 text-left text-[12px] text-text-muted hover:bg-surface-2 hover:text-closed"
              >
                <Trash2 size={13} strokeWidth={1.75} aria-hidden />
                {t("column_delete")}
              </button>
            </div>
          </>
        )}

        {filterOpen && (
          <>
            <div className="fixed inset-0 z-10" aria-hidden onClick={() => setFilterOpen(false)} />
            <div className="absolute right-0 top-11 z-20">
              <ColumnFilterEditor
                filter={column.filter}
                repos={repos}
                labels={labels}
                onClose={() => setFilterOpen(false)}
                onApply={(filter) => {
                  onUpdateColumn(column.id, column.name, filter);
                  setFilterOpen(false);
                }}
              />
            </div>
          </>
        )}
      </div>

      {/* Cards */}
      <div className="flex min-h-[6rem] flex-1 flex-col gap-2 overflow-y-auto p-2">
        {items.length === 0 ? (
          <p className="px-2 py-6 text-center text-[12px] text-text-faint">
            {t("board_no_match")}
          </p>
        ) : (
          items.map((item) => (
            <BoardItemCard
              key={item.key}
              item={item}
              columns={otherColumns}
              onMove={(columnId) => onMoveCard(item.key, columnId)}
              onRemove={() => onRemoveCard(item.key)}
              onDragStart={() => onDragStart(item.key)}
              onDragEnd={onDragEnd}
            />
          ))
        )}
      </div>
    </div>
  );
}

export default function BoardView() {
  const { t } = useTranslation();
  const { boardId } = useParams<{ boardId: string }>();
  const { accounts, activeAccountId } = useAccounts();
  const qc = useQueryClient();

  const [actionError, setActionError] = useState<string | null>(null);
  const [addingColumn, setAddingColumn] = useState(false);
  const [newColName, setNewColName] = useState("");
  const draggingRef = useRef<string | null>(null);

  // Board + the issues/pulls/tasks pool are independent cached queries that run
  // in parallel and dedupe against the other views (e.g. Issues/Pulls/Tasks).
  const boardQuery = useBoard(boardId);
  const tasksQuery = useTasks();
  const issuesQuery = useIssues(activeAccountId);
  const pullsQuery = usePulls(activeAccountId);

  const board = boardQuery.data ?? null;
  const notFound = boardQuery.isSuccess && boardQuery.data == null;
  const loading =
    (boardQuery.isPending && boardQuery.fetchStatus !== "idle") ||
    (tasksQuery.isPending && tasksQuery.fetchStatus !== "idle") ||
    (issuesQuery.isPending && issuesQuery.fetchStatus !== "idle") ||
    (pullsQuery.isPending && pullsQuery.fetchStatus !== "idle");

  const queryError =
    boardQuery.error ?? issuesQuery.error ?? pullsQuery.error ?? tasksQuery.error;
  const error =
    actionError ??
    (queryError
      ? queryError instanceof Error
        ? queryError.message
        : String(queryError)
      : null);
  const setError = setActionError;

  const reloadBoard = async () => {
    await qc.invalidateQueries({ queryKey: queryKeys.board(boardId) });
  };

  const pool = useMemo<BoardItem[]>(
    () =>
      buildPool({
        accountId: activeAccountId ?? "",
        issues: issuesQuery.data ?? [],
        pulls: pullsQuery.data ?? [],
        todos: (tasksQuery.data ?? []) as TaskDto[],
      }),
    [activeAccountId, issuesQuery.data, pullsQuery.data, tasksQuery.data],
  );

  const resolved = useMemo(
    () => (board ? resolveBoard(board, pool) : []),
    [board, pool],
  );

  const repos = useMemo(() => distinct(pool.map((i) => i.repo)), [pool]);
  const labels = useMemo(
    () => distinct(pool.flatMap((i) => i.labels)),
    [pool],
  );

  const accountName =
    accounts.find((a) => a.id === activeAccountId)?.display_name ?? null;

  async function handleUpdateColumn(
    id: string,
    name: string,
    filter: SmartFilter,
  ) {
    try {
      await api.updateColumn(id, name, filter);
      await reloadBoard();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleDeleteColumn(id: string) {
    if (!window.confirm(t("column_delete_confirm"))) return;
    try {
      await api.deleteColumn(id);
      await reloadBoard();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleRenameColumn(id: string, name: string) {
    const col = board?.columns.find((c) => c.id === id);
    if (!col) return;
    try {
      await api.updateColumn(id, name, col.filter);
      await reloadBoard();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleAddColumn() {
    const name = newColName.trim();
    if (!name || !boardId) return;
    try {
      await api.createColumn(boardId, name, { state: "open" });
      setNewColName("");
      setAddingColumn(false);
      await reloadBoard();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleMoveCard(key: string, columnId: string) {
    if (!boardId) return;
    try {
      await api.placeCard(boardId, columnId, key);
      await reloadBoard();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleRemoveCard(key: string) {
    if (!boardId) return;
    try {
      await api.removeCard(boardId, key);
      await reloadBoard();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  function handleDropToColumn(columnId: string) {
    const key = draggingRef.current;
    draggingRef.current = null;
    if (!key) return;
    // Find which column the item currently resolves into; only place if moving.
    const current = resolved.find((rc) => rc.items.some((it) => itemKey(it) === key));
    if (current?.column.id === columnId) return;
    void handleMoveCard(key, columnId);
  }

  if (loading) {
    return (
      <div className="px-8 py-10">
        <Skeleton className="mb-6 h-6 w-40" />
        <div className="flex gap-3">
          {Array.from({ length: 3 }, (_, i) => (
            <div
              key={i}
              className="w-72 shrink-0 rounded-[--radius-lg] border border-border bg-surface p-3"
            >
              <Skeleton className="mb-3 h-4 w-1/2" />
              <div className="space-y-2">
                <Skeleton className="h-16 w-full" />
                <Skeleton className="h-16 w-full" />
              </div>
            </div>
          ))}
        </div>
      </div>
    );
  }

  if (notFound) {
    return (
      <div className="mx-auto max-w-6xl px-8 py-10">
        <Link
          to="/boards"
          className={
            "mb-5 inline-flex items-center gap-1 text-[13px] text-text-muted hover:text-text " + focusRing
          }
        >
          <ChevronLeft size={15} strokeWidth={1.75} aria-hidden />
          {t("board_back")}
        </Link>
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <AlertCircle size={22} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <p className="text-sm text-text-muted">{t("board_not_found")}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen flex-col">
      <header className="shrink-0 border-b border-border px-8 py-5">
        <Link
          to="/boards"
          className={
            "mb-2 inline-flex items-center gap-1 text-[12px] text-text-muted hover:text-text " + focusRing
          }
        >
          <ChevronLeft size={14} strokeWidth={1.75} aria-hidden />
          {t("board_back")}
        </Link>
        <div className="flex items-center justify-between gap-4">
          <h1 className="text-[20px] font-semibold tracking-tight text-text">
            {board?.name}
          </h1>
          {accountName && (
            <span className="font-mono text-[12px] text-text-muted">{accountName}</span>
          )}
        </div>
        {!activeAccountId && (
          <p className="mt-2 text-[12px] text-text-faint">{t("board_no_account")}</p>
        )}
        {error && (
          <p className="mt-2 break-words text-[12px] text-closed">{error}</p>
        )}
      </header>

      {/* Horizontal columns */}
      <div className="flex flex-1 items-start gap-3 overflow-x-auto px-8 py-5">
        {resolved.map((rc) => (
          <ColumnView
            key={rc.column.id}
            column={rc.column}
            items={rc.items}
            allColumns={board?.columns ?? []}
            repos={repos}
            labels={labels}
            onUpdateColumn={handleUpdateColumn}
            onDeleteColumn={handleDeleteColumn}
            onRename={handleRenameColumn}
            onMoveCard={handleMoveCard}
            onRemoveCard={handleRemoveCard}
            onDropToColumn={handleDropToColumn}
            onDragStart={(key) => {
              draggingRef.current = key;
            }}
            onDragEnd={() => {
              draggingRef.current = null;
            }}
          />
        ))}

        {/* Add column */}
        <div className="w-72 shrink-0">
          {addingColumn ? (
            <div className="flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-3">
              <input
                autoFocus
                value={newColName}
                onChange={(e) => setNewColName(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") void handleAddColumn();
                  if (e.key === "Escape") {
                    setAddingColumn(false);
                    setNewColName("");
                  }
                }}
                placeholder={t("column_name_placeholder")}
                className={
                  "rounded-[--radius] border border-border bg-surface-2 px-2.5 py-1.5 text-[13px] text-text " +
                  "placeholder:text-text-faint focus-visible:border-accent " +
                  focusRing
                }
              />
              <div className="flex items-center gap-2">
                <Button variant="primary" onClick={() => void handleAddColumn()}>
                  {t("board_create")}
                </Button>
                <Button
                  variant="ghost"
                  onClick={() => {
                    setAddingColumn(false);
                    setNewColName("");
                  }}
                >
                  {t("board_done")}
                </Button>
              </div>
            </div>
          ) : (
            <button
              type="button"
              onClick={() => setAddingColumn(true)}
              className={
                "flex w-full items-center justify-center gap-2 rounded-[--radius-lg] border border-dashed border-border " +
                "px-3 py-3 text-[13px] text-text-muted transition-colors duration-150 " +
                "hover:border-border-strong hover:text-text " +
                focusRing
              }
            >
              <Plus size={15} strokeWidth={1.75} aria-hidden />
              {t("column_new")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
