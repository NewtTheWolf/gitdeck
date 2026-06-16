import { useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import { useTranslation } from "react-i18next";
import {
  CircleDot,
  CircleCheck,
  Loader2,
  MessageSquare,
  Plus,
  Send,
  Tag,
  UserPlus,
  X,
  ExternalLink,
} from "lucide-react";
import type { Issue } from "../../lib/api";
import { formatRelativeTime, labelChipStyle } from "../../lib/format";
import { issueDetailPath } from "../../lib/itemRoute";
import { Avatar } from "../../components/ui/Avatar";
import { openItem, StateDot } from "../../components/dashboard/listRow";

/** The set of in-flight actions for a single row (drives spinners + disabled). */
export type BusyAction = "state" | "assign" | "label" | "comment" | null;

export interface TriageRowProps {
  item: Issue;
  /** The login to assign when "Assign to me" is clicked (account display_name). */
  meLogin: string | null;
  /** Label names seen across the issue's repo — offered as quick-add chips. */
  repoLabelPool: string[];
  busy: BusyAction;
  /** Per-row transient error (e.g. a rejected write). Cleared by the parent. */
  error: string | null;
  /** Transient success line: the html_url of a just-created comment, or null. */
  commentUrl: string | null;
  onSetState: (next: Issue["state"]) => void;
  onAssignMe: () => void;
  onAddLabel: (label: string) => void;
  onRemoveLabel: (label: string) => void;
  onComment: (body: string) => void;
  onDismissError: () => void;
}

const ACTION_BTN =
  "inline-flex items-center gap-1.5 rounded-[--radius] border border-border bg-surface-2 " +
  "px-2.5 py-1.5 text-[12px] text-text-muted " +
  "transition-[color,border-color,background-color] duration-150 ease-out " +
  "hover:border-border-strong hover:text-text " +
  "disabled:cursor-not-allowed disabled:opacity-50 " +
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none";

const FIELD =
  "w-full rounded-[--radius] border border-border bg-surface-2 px-2.5 py-1.5 text-[13px] text-text " +
  "outline-none transition-[color,border-color] duration-150 ease-out placeholder:text-text-faint " +
  "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none";

/** A removable label chip — the ✕ calls `onRemove`. */
function RemovableLabel({
  name,
  color,
  onRemove,
  disabled,
  removeTitle,
}: {
  name: string;
  color: string;
  onRemove: () => void;
  disabled: boolean;
  removeTitle: string;
}) {
  const style = labelChipStyle(color);
  return (
    <span
      className="inline-flex items-center gap-1 rounded-full border px-1.5 py-0.5 text-[11px] text-text-muted"
      style={{ background: style.background, borderColor: style.borderColor }}
    >
      <span
        aria-hidden
        className="size-1.5 shrink-0 rounded-full"
        style={{ background: style.dot }}
      />
      {name}
      <button
        type="button"
        onClick={onRemove}
        disabled={disabled}
        title={removeTitle}
        aria-label={`${removeTitle}: ${name}`}
        className={
          "ml-0.5 flex size-4 items-center justify-center rounded-full " +
          "text-text-faint transition-colors duration-150 ease-out " +
          "hover:bg-surface-3 hover:text-text disabled:cursor-not-allowed disabled:opacity-40 " +
          "outline-none focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
        }
      >
        <X size={11} strokeWidth={2} aria-hidden />
      </button>
    </span>
  );
}

/**
 * One actionable triage row. Visually rhymes with IssueCard (hairline border,
 * surface fill) but is denser and carries an actions cluster: close/reopen
 * (with an inline confirm before closing), assign-to-me, add/remove label, and
 * an inline comment composer. All write feedback (spinner / error / success) is
 * driven by props the parent owns, so optimistic state + reverts live in one
 * place (TriageWorkspace).
 */
export function TriageRow({
  item,
  meLogin,
  repoLabelPool,
  busy,
  error,
  commentUrl,
  onSetState,
  onAssignMe,
  onAddLabel,
  onRemoveLabel,
  onComment,
  onDismissError,
}: TriageRowProps) {
  const { t, i18n } = useTranslation();
  const navigate = useNavigate();
  const labels = item.labels || [];
  const assignees = item.assignees || [];
  const author = item.author_login || "unknown";
  const isOpen = item.state === "open";
  const detailPath = issueDetailPath(item.repo_name_with_owner, item.number);

  const [confirmingClose, setConfirmingClose] = useState(false);
  const [labelOpen, setLabelOpen] = useState(false);
  const [labelText, setLabelText] = useState("");
  const [commentOpen, setCommentOpen] = useState(false);
  const [commentText, setCommentText] = useState("");
  const labelInputRef = useRef<HTMLInputElement>(null);

  const alreadyAssigned = !!meLogin && assignees.includes(meLogin);
  const poolUnused = repoLabelPool.filter(
    (name) => !labels.some((l) => l.name === name),
  );

  function submitLabel(name: string) {
    const trimmed = name.trim();
    if (!trimmed) return;
    onAddLabel(trimmed);
    setLabelText("");
    setLabelOpen(false);
  }

  function submitComment() {
    const body = commentText.trim();
    if (!body) return;
    onComment(body);
  }

  return (
    <div
      className={
        "flex flex-col gap-3 rounded-[--radius-lg] border border-border bg-surface p-4 " +
        "transition-colors duration-150 ease-out"
      }
    >
      {/* Top row: state + repo + #number + relative time */}
      <div className="flex items-center gap-2 text-[12px]">
        <span className="flex items-center gap-1.5">
          <StateDot className={isOpen ? "bg-open" : "bg-closed"} />
          <CircleDot
            size={13}
            strokeWidth={1.75}
            aria-hidden
            className={isOpen ? "text-open" : "text-closed"}
          />
        </span>
        <span
          className="min-w-0 truncate font-mono text-text-muted"
          title={item.repo_name_with_owner}
        >
          {item.repo_name_with_owner}
        </span>
        <span className="tnum shrink-0 font-mono text-text-faint">
          #{item.number}
        </span>
        <span className="ml-auto shrink-0 whitespace-nowrap text-text-faint">
          {formatRelativeTime(item.updated_at, Date.now(), i18n.language)}
        </span>
      </div>

      {/* Title — navigates to the in-app issue detail */}
      <button
        type="button"
        onClick={() => navigate(detailPath)}
        className={
          "text-left text-[13px] font-medium leading-relaxed text-text " +
          "transition-colors duration-150 ease-out hover:text-accent " +
          "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
          "focus-visible:ring-offset-2 focus-visible:ring-offset-surface rounded-[--radius-sm] motion-reduce:transition-none"
        }
      >
        {item.title}
      </button>

      {/* Labels (removable) + author */}
      <div className="flex flex-wrap items-center gap-1.5">
        <Avatar login={author} src={item.author_avatar_url} size={18} />
        <span className="text-[12px] font-medium text-text-muted">{author}</span>
        {labels.map((label) => (
          <RemovableLabel
            key={label.name}
            name={label.name}
            color={label.color}
            disabled={busy === "label"}
            removeTitle={t("triage_remove_label")}
            onRemove={() => onRemoveLabel(label.name)}
          />
        ))}
        <span
          className="ml-auto flex items-center gap-1 text-[12px] text-text-faint"
          title={t("comments_count", { count: item.comments_count })}
        >
          <MessageSquare size={13} strokeWidth={1.75} aria-hidden />
          <span className="tnum">{item.comments_count}</span>
        </span>
        {assignees.length > 0 && (
          <span className="flex -space-x-1.5">
            {assignees.slice(0, 3).map((login) => (
              <Avatar
                key={login}
                login={login}
                size={18}
                className="ring-1 ring-surface"
              />
            ))}
          </span>
        )}
      </div>

      {/* Actions cluster */}
      <div className="flex flex-wrap items-center gap-2 border-t border-border pt-3">
        {/* Close / Reopen with inline confirm before closing */}
        {isOpen ? (
          confirmingClose ? (
            <span className="inline-flex items-center gap-1.5 rounded-[--radius] border border-border bg-surface-2 p-0.5">
              <span className="px-2 text-[12px] text-text-muted">
                {t("triage_close_confirm")}
              </span>
              <button
                type="button"
                disabled={busy === "state"}
                onClick={() => {
                  setConfirmingClose(false);
                  onSetState("closed");
                }}
                className={
                  "inline-flex items-center gap-1.5 rounded-[--radius-sm] bg-accent px-2.5 py-1 " +
                  "text-[12px] font-medium text-accent-fg hover:bg-accent-hover " +
                  "disabled:cursor-not-allowed disabled:opacity-50 " +
                  "outline-none focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
                }
              >
                {busy === "state" && (
                  <Loader2 size={13} className="animate-spin motion-reduce:hidden" aria-hidden />
                )}
                {t("triage_close")}
              </button>
              <button
                type="button"
                onClick={() => setConfirmingClose(false)}
                className={
                  "rounded-[--radius-sm] px-2.5 py-1 text-[12px] text-text-muted hover:text-text " +
                  "outline-none focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
                }
              >
                {t("triage_cancel")}
              </button>
            </span>
          ) : (
            <button
              type="button"
              disabled={busy === "state"}
              onClick={() => setConfirmingClose(true)}
              className={ACTION_BTN}
            >
              <CircleCheck size={14} strokeWidth={1.75} aria-hidden />
              {t("triage_close")}
            </button>
          )
        ) : (
          <button
            type="button"
            disabled={busy === "state"}
            onClick={() => onSetState("open")}
            className={ACTION_BTN}
          >
            {busy === "state" ? (
              <Loader2 size={14} className="animate-spin motion-reduce:hidden" aria-hidden />
            ) : (
              <CircleDot size={14} strokeWidth={1.75} aria-hidden />
            )}
            {t("triage_reopen")}
          </button>
        )}

        {/* Assign to me */}
        <button
          type="button"
          disabled={busy === "assign" || alreadyAssigned || !meLogin}
          onClick={onAssignMe}
          title={meLogin ? `@${meLogin}` : undefined}
          className={ACTION_BTN}
        >
          {busy === "assign" ? (
            <Loader2 size={14} className="animate-spin motion-reduce:hidden" aria-hidden />
          ) : (
            <UserPlus size={14} strokeWidth={1.75} aria-hidden />
          )}
          {t("triage_assign_me")}
        </button>

        {/* Add label */}
        <div className="relative">
          <button
            type="button"
            disabled={busy === "label"}
            onClick={() => {
              setLabelOpen((v) => !v);
              setTimeout(() => labelInputRef.current?.focus(), 0);
            }}
            aria-expanded={labelOpen}
            className={ACTION_BTN}
          >
            {busy === "label" ? (
              <Loader2 size={14} className="animate-spin motion-reduce:hidden" aria-hidden />
            ) : (
              <Tag size={14} strokeWidth={1.75} aria-hidden />
            )}
            {t("triage_add_label")}
          </button>
          {labelOpen && (
            <div
              className="absolute left-0 top-full z-10 mt-1.5 w-64 rounded-[--radius-lg] border border-border bg-surface-2 p-2 shadow-lg"
              role="dialog"
            >
              <form
                onSubmit={(e) => {
                  e.preventDefault();
                  submitLabel(labelText);
                }}
                className="flex items-center gap-1.5"
              >
                <input
                  ref={labelInputRef}
                  type="text"
                  value={labelText}
                  onChange={(e) => setLabelText(e.target.value)}
                  placeholder={t("triage_label_placeholder")}
                  className={FIELD}
                />
                <button
                  type="submit"
                  disabled={!labelText.trim()}
                  aria-label={t("triage_add_label")}
                  className={
                    "flex size-8 shrink-0 items-center justify-center rounded-[--radius] " +
                    "border border-border bg-surface text-text-muted hover:text-text " +
                    "disabled:cursor-not-allowed disabled:opacity-40 " +
                    "outline-none focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
                  }
                >
                  <Plus size={15} strokeWidth={2} aria-hidden />
                </button>
              </form>
              {poolUnused.length > 0 && (
                <div className="mt-2 flex flex-wrap gap-1">
                  {poolUnused.slice(0, 12).map((name) => (
                    <button
                      key={name}
                      type="button"
                      onClick={() => submitLabel(name)}
                      className={
                        "rounded-full border border-border bg-surface px-2 py-0.5 text-[11px] text-text-muted " +
                        "hover:border-border-strong hover:text-text " +
                        "outline-none focus-visible:ring-2 focus-visible:ring-accent motion-reduce:transition-none"
                      }
                    >
                      {name}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Comment toggle */}
        <button
          type="button"
          onClick={() => setCommentOpen((v) => !v)}
          aria-expanded={commentOpen}
          className={ACTION_BTN}
        >
          <MessageSquare size={14} strokeWidth={1.75} aria-hidden />
          {t("triage_comment")}
        </button>

        {/* Open in browser */}
        <button
          type="button"
          onClick={() => openItem(item.html_url)}
          className={ACTION_BTN + " ml-auto"}
        >
          <ExternalLink size={14} strokeWidth={1.75} aria-hidden />
          {t("card_open_link")}
        </button>
      </div>

      {/* Inline comment composer */}
      {commentOpen && (
        <form
          onSubmit={(e) => {
            e.preventDefault();
            submitComment();
          }}
          className="flex flex-col gap-2"
        >
          <textarea
            value={commentText}
            onChange={(e) => setCommentText(e.target.value)}
            placeholder={t("triage_comment_placeholder")}
            rows={3}
            className={FIELD + " resize-y"}
          />
          <div className="flex items-center gap-2">
            <button
              type="submit"
              disabled={busy === "comment" || !commentText.trim()}
              className={
                "inline-flex items-center gap-1.5 rounded-[--radius] bg-accent px-3 py-1.5 " +
                "text-[12px] font-medium text-accent-fg hover:bg-accent-hover " +
                "disabled:cursor-not-allowed disabled:opacity-50 " +
                "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
                "focus-visible:ring-offset-2 focus-visible:ring-offset-surface motion-reduce:transition-none"
              }
            >
              {busy === "comment" ? (
                <Loader2 size={13} className="animate-spin motion-reduce:hidden" aria-hidden />
              ) : (
                <Send size={13} strokeWidth={1.75} aria-hidden />
              )}
              {t("triage_comment_send")}
            </button>
            {commentUrl && (
              <a
                href={commentUrl}
                onClick={(e) => {
                  e.preventDefault();
                  openItem(commentUrl);
                }}
                className="inline-flex items-center gap-1 text-[12px] text-open hover:underline"
              >
                <CircleCheck size={13} strokeWidth={1.75} aria-hidden />
                {t("triage_comment_added")}
              </a>
            )}
          </div>
        </form>
      )}

      {/* Inline error (a rejected write) */}
      {error && (
        <div
          role="alert"
          className="flex items-start gap-2 rounded-[--radius] border border-closed/40 bg-closed/10 px-3 py-2 text-[12px] text-text"
        >
          <span className="min-w-0 flex-1 break-words">
            {t("triage_action_failed")}: {error}
          </span>
          <button
            type="button"
            onClick={onDismissError}
            aria-label={t("triage_dismiss")}
            className="shrink-0 text-text-faint hover:text-text outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <X size={13} strokeWidth={2} aria-hidden />
          </button>
        </div>
      )}
    </div>
  );
}

export default TriageRow;
