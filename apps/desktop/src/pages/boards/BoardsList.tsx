import { useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { LayoutGrid, Plus, Pencil, Trash2, AlertCircle } from "lucide-react";
import { api, type Board } from "../../lib/api";
import { queryKeys, useBoards } from "../../lib/query/queries";
import { formatRelativeTime } from "../../lib/format";
import { Button } from "../../components/ui/Button";
import { Skeleton } from "../../components/ui/Skeleton";

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

function BoardsSkeleton() {
  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {Array.from({ length: 4 }, (_, i) => (
        <div
          key={i}
          className="flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4"
        >
          <Skeleton className="h-4 w-1/2" />
          <Skeleton className="h-3 w-1/3" />
        </div>
      ))}
    </div>
  );
}

export default function BoardsList() {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const qc = useQueryClient();
  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);

  const query = useBoards();
  const boards: Board[] = Array.isArray(query.data) ? query.data : [];
  const loading = query.isPending && query.fetchStatus !== "idle";
  const error =
    actionError ??
    (query.isError
      ? query.error instanceof Error
        ? query.error.message
        : String(query.error)
      : null);
  const setError = setActionError;
  const reload = () =>
    qc.invalidateQueries({ queryKey: queryKeys.boards() });

  async function createBoard() {
    const name = newName.trim();
    if (!name || busy) return;
    setBusy(true);
    try {
      const board = await api.createBoard(name);
      setNewName("");
      setCreating(false);
      navigate(`/boards/${board.id}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setBusy(false);
    }
  }

  async function renameBoard(board: Board) {
    const next = window.prompt(t("board_rename"), board.name);
    if (next === null) return;
    const name = next.trim();
    if (!name || name === board.name) return;
    try {
      await api.renameBoard(board.id, name);
      await reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  async function deleteBoard(board: Board) {
    if (!window.confirm(t("board_delete_confirm"))) return;
    try {
      await api.deleteBoard(board.id);
      await reload();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }

  return (
    <div className="mx-auto max-w-6xl px-8 py-10">
      <header className="mb-5 flex items-start justify-between gap-4">
        <div>
          <h1 className="text-[22px] font-semibold tracking-tight text-text">
            {t("boards_title")}
          </h1>
          <p className="mt-1 text-[13px] text-text-muted">
            {t("boards_subtitle")}
          </p>
        </div>
        {!creating && (
          <Button variant="primary" onClick={() => setCreating(true)}>
            <Plus size={15} strokeWidth={1.75} aria-hidden />
            {t("board_new")}
          </Button>
        )}
      </header>

      {creating && (
        <div className="mb-4 flex items-center gap-2">
          <input
            autoFocus
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void createBoard();
              if (e.key === "Escape") {
                setCreating(false);
                setNewName("");
              }
            }}
            placeholder={t("board_name_placeholder")}
            className={
              "min-w-[14rem] flex-1 rounded-[--radius] border border-border bg-surface-2 px-3 py-2 text-[13px] text-text " +
              "placeholder:text-text-faint focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
              focusRing
            }
          />
          <Button variant="primary" disabled={busy} onClick={() => void createBoard()}>
            {t("board_create")}
          </Button>
          <Button
            variant="ghost"
            onClick={() => {
              setCreating(false);
              setNewName("");
            }}
          >
            {t("board_done")}
          </Button>
        </div>
      )}

      {loading ? (
        <BoardsSkeleton />
      ) : error ? (
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <AlertCircle size={22} strokeWidth={1.75} aria-hidden className="text-closed" />
          <p className="text-sm font-medium text-text">{t("section_error")}</p>
          <p className="max-w-md break-words text-xs text-text-faint">{error}</p>
        </div>
      ) : boards.length === 0 ? (
        <div className="flex flex-col items-start gap-3 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <LayoutGrid size={22} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <p className="max-w-md text-sm text-text-muted">{t("boards_empty")}</p>
        </div>
      ) : (
        <ul className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
          {boards.map((board) => (
            <li key={board.id}>
              <div
                className={
                  "group relative flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-4 " +
                  "transition-[transform,color,background-color,border-color] duration-200 ease-out " +
                  "hover:-translate-y-px hover:border-border-strong hover:bg-surface-2 motion-reduce:transform-none"
                }
              >
                <button
                  type="button"
                  onClick={() => navigate(`/boards/${board.id}`)}
                  className={"flex flex-col gap-1 text-left " + focusRing}
                >
                  <span className="text-[14px] font-medium text-text">{board.name}</span>
                  <span className="text-[12px] text-text-faint">
                    {formatRelativeTime(board.updated_at, Date.now(), i18n.language)}
                  </span>
                </button>
                <div className="mt-1 flex items-center gap-1 opacity-0 transition-opacity duration-150 group-hover:opacity-100 group-focus-within:opacity-100">
                  <button
                    type="button"
                    aria-label={t("board_rename")}
                    title={t("board_rename")}
                    onClick={() => void renameBoard(board)}
                    className={
                      "rounded-[--radius] p-1.5 text-text-muted hover:bg-surface-3 hover:text-text " +
                      focusRing
                    }
                  >
                    <Pencil size={14} strokeWidth={1.75} aria-hidden />
                  </button>
                  <button
                    type="button"
                    aria-label={t("board_delete")}
                    title={t("board_delete")}
                    onClick={() => void deleteBoard(board)}
                    className={
                      "rounded-[--radius] p-1.5 text-text-muted hover:bg-surface-3 hover:text-closed " +
                      focusRing
                    }
                  >
                    <Trash2 size={14} strokeWidth={1.75} aria-hidden />
                  </button>
                </div>
              </div>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
