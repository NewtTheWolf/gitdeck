import { useCallback, useMemo, useRef, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useTranslation } from "react-i18next";
import { openUrl } from "@tauri-apps/plugin-opener";
import {
  ArrowLeft,
  CircleDot,
  CircleCheck,
  ExternalLink,
  GitMerge,
  GitPullRequest,
  Loader2,
  MessageSquare,
  Plus,
  Send,
  Tag,
  Milestone,
  UserPlus,
  Users,
  X,
} from "lucide-react";
import type {
  IssueDetail,
  PullRequestDetail,
  Label,
  User,
} from "../../lib/api";
import {
  useIssueDetail,
  usePullRequestDetail,
  useIssueComments,
  usePullReviews,
  useSetIssueState,
  useAddIssueLabels,
  useRemoveIssueLabel,
  useAddIssueAssignees,
  useCreateIssueComment,
} from "../../lib/query/queries";
import { formatRelativeTime, labelChipStyle } from "../../lib/format";
import { repoDetailPath } from "../../lib/repoRoute";
import { useAccounts } from "../../contexts/AccountContext";
import { Avatar } from "../../components/ui/Avatar";
import { Skeleton } from "../../components/ui/Skeleton";
import { Markdown } from "../../components/ui/Markdown";
import { NoAccount, SectionShell } from "../dashboard/sectionScaffold";
import {
  latestReviewsByReviewer,
  reviewSummary,
  isReviewRequestedFromMe,
  type LatestReview,
} from "./reviews";

export type DetailKind = "issue" | "pr";

function openExternal(url: string) {
  void openUrl(url).catch(() => {});
}

const focusRing =
  "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

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

type BusyAction = "state" | "assign" | "label" | "comment" | null;

// --- small presentational pieces -------------------------------------------

function StateBadge({
  kind,
  detail,
}: {
  kind: DetailKind;
  detail: IssueDetail | PullRequestDetail;
}) {
  const { t } = useTranslation();
  let tone: string;
  let icon = <CircleDot size={13} strokeWidth={2} aria-hidden />;
  let label: string;

  if (kind === "pr") {
    const pr = detail as PullRequestDetail;
    icon = pr.merged ? (
      <GitMerge size={13} strokeWidth={2} aria-hidden />
    ) : (
      <GitPullRequest size={13} strokeWidth={2} aria-hidden />
    );
    if (pr.merged) {
      tone = "border-merged/40 bg-merged/10 text-merged";
      label = t("state_merged");
    } else if (pr.is_draft) {
      tone = "border-border bg-surface-2 text-text-faint";
      label = t("state_draft");
    } else if (pr.state === "open") {
      tone = "border-open/40 bg-open/10 text-open";
      label = t("state_open");
    } else {
      tone = "border-closed/40 bg-closed/10 text-closed";
      label = t("state_closed");
    }
  } else {
    if (detail.state === "open") {
      tone = "border-open/40 bg-open/10 text-open";
      label = t("state_open");
    } else {
      tone = "border-merged/40 bg-merged/10 text-merged";
      label = t("state_closed");
    }
  }

  return (
    <span
      className={
        "inline-flex items-center gap-1.5 rounded-full border px-2.5 py-1 text-[12px] font-medium " +
        tone
      }
    >
      {icon}
      {label}
    </span>
  );
}

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

function MetaBlock({
  icon,
  label,
  children,
}: {
  icon: React.ReactNode;
  label: string;
  children: React.ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2">
      <h2 className="flex items-center gap-1.5 text-[12px] font-medium uppercase tracking-wide text-text-faint">
        {icon}
        {label}
      </h2>
      {children}
    </section>
  );
}

// --- review status pieces ---------------------------------------------------

function ReviewerDecision({ review }: { review: LatestReview }) {
  const { t } = useTranslation();
  let dot: string;
  let label: string;
  if (review.state === "APPROVED") {
    dot = "text-open";
    label = t("review_approved");
  } else if (review.state === "CHANGES_REQUESTED") {
    dot = "text-closed";
    label = t("review_changes_requested");
  } else if (review.state === "DISMISSED") {
    dot = "text-text-faint";
    label = t("review_commented");
  } else {
    dot = "text-text-muted";
    label = t("review_commented");
  }
  return (
    <div className="flex items-center gap-2">
      <Avatar
        login={review.reviewer_login}
        src={review.reviewer_avatar_url}
        size={20}
      />
      <span className="text-[13px] text-text">{review.reviewer_login}</span>
      <span className={"ml-auto flex items-center gap-1 text-[12px] " + dot}>
        {review.state === "APPROVED" ? (
          <CircleCheck size={13} strokeWidth={2} aria-hidden />
        ) : review.state === "CHANGES_REQUESTED" ? (
          <X size={13} strokeWidth={2} aria-hidden />
        ) : (
          <MessageSquare size={13} strokeWidth={1.75} aria-hidden />
        )}
        {label}
      </span>
    </div>
  );
}

function SummaryChip({
  summary,
}: {
  summary: "changes_requested" | "approved" | "required" | "none";
}) {
  const { t } = useTranslation();
  const map = {
    changes_requested: {
      tone: "border-closed/40 bg-closed/10 text-closed",
      label: t("review_changes_requested"),
    },
    approved: {
      tone: "border-open/40 bg-open/10 text-open",
      label: t("review_approved"),
    },
    required: {
      tone: "border-warn/40 bg-warn/10 text-warn",
      label: t("review_required"),
    },
    none: {
      tone: "border-border bg-surface-2 text-text-faint",
      label: t("review_none"),
    },
  } as const;
  const cfg = map[summary];
  return (
    <span
      className={
        "inline-flex items-center rounded-full border px-2.5 py-1 text-[12px] font-medium " +
        cfg.tone
      }
    >
      {cfg.label}
    </span>
  );
}

// --- main view --------------------------------------------------------------

export default function DetailView({ kind }: { kind: DetailKind }) {
  const { t, i18n } = useTranslation();
  const params = useParams<{ owner: string; repo: string; number: string }>();
  const owner = params.owner ?? "";
  const repo = params.repo ?? "";
  const number = Number(params.number ?? "0");
  const { accounts, activeAccountId } = useAccounts();
  const activeAccount =
    accounts.find((a) => a.id === activeAccountId) ?? null;
  // The account's GitHub login IS its display_name (set at OAuth connect time).
  const meLogin = activeAccount?.display_name ?? null;

  const isPr = kind === "pr";

  const issueQuery = useIssueDetail(activeAccountId, owner, repo, number);
  const prQuery = usePullRequestDetail(activeAccountId, owner, repo, number);
  const detailQuery = isPr ? prQuery : issueQuery;
  const commentsQuery = useIssueComments(activeAccountId, owner, repo, number);
  const reviewsQuery = usePullReviews(activeAccountId, owner, repo, number, {
    enabled: isPr,
  });

  const detail = (detailQuery.data ?? null) as
    | IssueDetail
    | PullRequestDetail
    | null;

  // --- mutations -----------------------------------------------------------
  const setStateMutation = useSetIssueState(activeAccountId);
  const addLabelsMutation = useAddIssueLabels(activeAccountId);
  const removeLabelMutation = useRemoveIssueLabel(activeAccountId);
  const addAssigneesMutation = useAddIssueAssignees(activeAccountId);
  const createCommentMutation = useCreateIssueComment(activeAccountId);

  const [busy, setBusy] = useState<BusyAction>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [confirmingClose, setConfirmingClose] = useState(false);
  const [labelOpen, setLabelOpen] = useState(false);
  const [labelText, setLabelText] = useState("");
  const [commentText, setCommentText] = useState("");
  const labelInputRef = useRef<HTMLInputElement>(null);

  const coords = useMemo(
    () => ({ id: activeAccountId as string, owner, repo, number }),
    [activeAccountId, owner, repo, number],
  );

  function errMessage(err: unknown): string {
    return err instanceof Error ? err.message : String(err);
  }

  const run = useCallback(
    async (action: Exclude<BusyAction, null>, call: () => Promise<unknown>) => {
      if (!activeAccountId) return;
      setActionError(null);
      setBusy(action);
      try {
        await call();
      } catch (err) {
        setActionError(errMessage(err));
      } finally {
        setBusy(null);
      }
    },
    [activeAccountId],
  );

  const assignees: User[] = detail?.assignees ?? [];
  const alreadyAssigned =
    !!meLogin && assignees.some((a) => a.login === meLogin);
  const labels: Label[] = detail?.labels ?? [];

  function handleSetState(next: "open" | "closed") {
    void run("state", () =>
      setStateMutation.mutateAsync({ ...coords, state: next }),
    );
  }
  function handleAssignMe() {
    if (!meLogin || alreadyAssigned) return;
    void run("assign", () =>
      addAssigneesMutation.mutateAsync({ ...coords, assignees: [meLogin] }),
    );
  }
  function handleAddLabel(name: string) {
    const trimmed = name.trim();
    if (!trimmed) return;
    setLabelText("");
    setLabelOpen(false);
    void run("label", () =>
      addLabelsMutation.mutateAsync({ ...coords, labels: [trimmed] }),
    );
  }
  function handleRemoveLabel(name: string) {
    void run("label", () =>
      removeLabelMutation.mutateAsync({ ...coords, label: name }),
    );
  }
  function handleComment() {
    const body = commentText.trim();
    if (!body) return;
    void run("comment", async () => {
      await createCommentMutation.mutateAsync({ ...coords, body });
      setCommentText("");
    });
  }

  // --- render --------------------------------------------------------------
  const loading =
    detailQuery.isPending && detailQuery.fetchStatus !== "idle";

  const backLink = (
    <Link
      to={`/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}`}
      className={
        "mb-5 inline-flex items-center gap-1.5 text-[13px] text-text-muted " +
        "transition-colors duration-150 ease-out hover:text-text " +
        focusRing
      }
    >
      <ArrowLeft size={14} strokeWidth={1.75} aria-hidden />
      {`${owner}/${repo}`}
    </Link>
  );

  if (accounts.length === 0) {
    return (
      <SectionShell>
        {backLink}
        <NoAccount />
      </SectionShell>
    );
  }

  if (loading) {
    return (
      <SectionShell>
        {backLink}
        <div className="flex flex-col gap-4">
          <Skeleton className="h-6 w-3/4" />
          <Skeleton className="h-4 w-1/3" />
          <Skeleton className="h-24 w-full rounded-[--radius-lg]" />
        </div>
      </SectionShell>
    );
  }

  if (!detail) {
    return (
      <SectionShell>
        {backLink}
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <CircleDot
            size={20}
            strokeWidth={1.75}
            aria-hidden
            className="text-text-faint"
          />
          <p className="text-sm text-text-muted">{t("detail_not_found")}</p>
        </div>
      </SectionShell>
    );
  }

  const pr = isPr ? (detail as PullRequestDetail) : null;
  const latest = pr ? latestReviewsByReviewer(reviewsQuery.data ?? []) : [];
  const summary = pr
    ? reviewSummary(latest, pr.requested_reviewers, pr.requested_teams)
    : "none";
  const requestedFromMe = pr
    ? isReviewRequestedFromMe(pr.requested_reviewers, meLogin)
    : false;
  const isOpen = detail.state === "open";

  return (
    <SectionShell>
      {backLink}

      {/* Header */}
      <header className="mb-6 flex flex-col gap-3">
        <div className="flex flex-wrap items-center gap-2">
          <StateBadge kind={kind} detail={detail} />
          <Link
            to={repoDetailPath(detail.repo_name_with_owner)}
            className={
              "font-mono text-[13px] text-text-muted hover:text-text " + focusRing
            }
          >
            {detail.repo_name_with_owner}
          </Link>
          <a
            href={detail.html_url}
            onClick={(e) => {
              e.preventDefault();
              openExternal(detail.html_url);
            }}
            title={t("detail_open_on_github")}
            className={
              "ml-auto inline-flex items-center gap-1.5 rounded-[--radius] border border-border " +
              "bg-surface px-2.5 py-1 text-[13px] text-text-muted " +
              "transition-colors duration-150 ease-out hover:border-border-strong hover:text-text " +
              focusRing
            }
          >
            <ExternalLink size={13} strokeWidth={1.75} aria-hidden />
            {t("detail_open_on_github")}
          </a>
        </div>

        <h1 className="text-[22px] font-semibold leading-snug tracking-tight text-text">
          {detail.title}{" "}
          <span className="tnum font-mono font-normal text-text-faint">
            #{detail.number}
          </span>
        </h1>

        <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-[12px] text-text-muted">
          <span className="flex items-center gap-1.5">
            <Avatar
              login={detail.author_login}
              src={detail.author_avatar_url}
              size={20}
            />
            <span className="font-medium text-text">{detail.author_login}</span>
          </span>
          <span className="text-text-faint">
            {t("detail_opened")}{" "}
            {formatRelativeTime(detail.created_at, Date.now(), i18n.language)}
          </span>
          <span className="text-text-faint">
            ·{" "}
            {t("detail_updated")}{" "}
            {formatRelativeTime(detail.updated_at, Date.now(), i18n.language)}
          </span>
        </div>
      </header>

      <div className="grid grid-cols-1 gap-8 lg:grid-cols-[1fr_240px]">
        {/* Main column */}
        <div className="flex min-w-0 flex-col gap-6 lg:order-1">
          {/* PR branch + diff stats */}
          {pr && (
            <section className="flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-4">
              <div className="flex flex-wrap items-center gap-2 text-[12px]">
                <span className="font-medium uppercase tracking-wide text-text-faint">
                  {t("pr_branches")}
                </span>
                <span className="inline-flex items-center gap-1.5 font-mono text-[12px] text-text">
                  <span className="rounded-[--radius-sm] border border-border bg-surface-2 px-1.5 py-0.5">
                    {pr.base_ref}
                  </span>
                  <ArrowLeft size={12} strokeWidth={2} aria-hidden className="text-text-faint" />
                  <span className="rounded-[--radius-sm] border border-border bg-surface-2 px-1.5 py-0.5">
                    {pr.head_ref}
                  </span>
                </span>
              </div>
              {(pr.additions != null ||
                pr.deletions != null ||
                pr.changed_files != null) && (
                <div className="flex items-center gap-2 text-[12px]">
                  <span className="font-medium uppercase tracking-wide text-text-faint">
                    {t("pr_diff_stats")}
                  </span>
                  <span className="tnum font-mono text-open">
                    +{pr.additions ?? 0}
                  </span>
                  <span className="tnum font-mono text-closed">
                    −{pr.deletions ?? 0}
                  </span>
                  <span className="tnum font-mono text-text-faint">
                    · {pr.changed_files ?? 0} {t("pr_files")}
                  </span>
                </div>
              )}
            </section>
          )}

          {/* Body */}
          <section className="rounded-[--radius-lg] border border-border bg-surface p-4">
            {detail.body && detail.body.trim() ? (
              <Markdown>{detail.body}</Markdown>
            ) : (
              <p className="text-[13px] text-text-faint">{t("detail_no_body")}</p>
            )}
          </section>

          {/* PR reviews */}
          {pr && (
            <section className="flex flex-col gap-3">
              <div className="flex flex-wrap items-center gap-2">
                <h2 className="text-[14px] font-semibold text-text">
                  {t("pr_reviews")}
                </h2>
                <SummaryChip summary={summary} />
              </div>

              {requestedFromMe && (
                <div className="inline-flex w-fit items-center gap-1.5 rounded-[--radius] border border-accent bg-accent/10 px-2.5 py-1.5 text-[12px] font-medium text-accent">
                  <UserPlus size={13} strokeWidth={2} aria-hidden />
                  {t("pr_review_from_you")}
                </div>
              )}

              {/* Per-reviewer decisions */}
              {latest.length > 0 && (
                <div className="flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-4">
                  {latest.map((r) => (
                    <ReviewerDecision key={r.reviewer_login} review={r} />
                  ))}
                </div>
              )}

              {/* Requested reviewers */}
              {(pr.requested_reviewers.length > 0 ||
                pr.requested_teams.length > 0) && (
                <div className="flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-4">
                  <h3 className="text-[12px] font-medium uppercase tracking-wide text-text-faint">
                    {t("pr_requested_reviewers")}
                  </h3>
                  <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
                    {pr.requested_reviewers.map((u) => (
                      <span
                        key={u.login}
                        className="flex items-center gap-1.5 text-[13px]"
                      >
                        <Avatar login={u.login} src={u.avatar_url} size={20} />
                        <span className="text-text">{u.login}</span>
                        {u.login === meLogin && (
                          <span className="rounded-full border border-accent bg-accent/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-accent">
                            {t("pr_review_requested")}
                          </span>
                        )}
                      </span>
                    ))}
                    {pr.requested_teams.map((team) => (
                      <span
                        key={team}
                        className="flex items-center gap-1.5 text-[13px] text-text-muted"
                      >
                        <Users size={16} strokeWidth={1.75} aria-hidden />
                        {team}
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </section>
          )}

          {/* Comments */}
          <section className="flex flex-col gap-3">
            <h2 className="flex items-center gap-1.5 text-[14px] font-semibold text-text">
              <MessageSquare size={15} strokeWidth={1.75} aria-hidden />
              {t("detail_comments")}
              <span className="tnum text-[12px] font-normal text-text-faint">
                {detail.comments_count}
              </span>
            </h2>
            {commentsQuery.isPending &&
            commentsQuery.fetchStatus !== "idle" ? (
              <Skeleton className="h-20 w-full rounded-[--radius-lg]" />
            ) : (commentsQuery.data ?? []).length === 0 ? (
              <p className="rounded-[--radius-lg] border border-border bg-surface px-4 py-6 text-[13px] text-text-faint">
                {t("detail_no_comments")}
              </p>
            ) : (
              <div className="flex flex-col gap-3">
                {(commentsQuery.data ?? []).map((c) => (
                  <article
                    key={c.id}
                    className="flex flex-col gap-2 rounded-[--radius-lg] border border-border bg-surface p-4"
                  >
                    <div className="flex items-center gap-2 text-[12px]">
                      <Avatar
                        login={c.author_login}
                        src={c.author_avatar_url}
                        size={20}
                      />
                      <span className="font-medium text-text">
                        {c.author_login}
                      </span>
                      <span className="text-text-faint">
                        {formatRelativeTime(
                          c.created_at,
                          Date.now(),
                          i18n.language,
                        )}
                      </span>
                    </div>
                    {c.body.trim() ? (
                      <Markdown>{c.body}</Markdown>
                    ) : (
                      <p className="text-[13px] text-text-faint">
                        {t("detail_no_body")}
                      </p>
                    )}
                  </article>
                ))}
              </div>
            )}
          </section>

          {/* Comment composer */}
          <form
            onSubmit={(e) => {
              e.preventDefault();
              handleComment();
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
            <div>
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
            </div>
          </form>

          {actionError && (
            <div
              role="alert"
              className="flex items-start gap-2 rounded-[--radius] border border-closed/40 bg-closed/10 px-3 py-2 text-[12px] text-text"
            >
              <span className="min-w-0 flex-1 break-words">
                {t("triage_action_failed")}: {actionError}
              </span>
              <button
                type="button"
                onClick={() => setActionError(null)}
                aria-label={t("triage_dismiss")}
                className="shrink-0 text-text-faint hover:text-text outline-none focus-visible:ring-2 focus-visible:ring-accent"
              >
                <X size={13} strokeWidth={2} aria-hidden />
              </button>
            </div>
          )}
        </div>

        {/* Aside / meta */}
        <aside className="flex flex-col gap-6 lg:order-2">
          {/* Actions */}
          <div className="flex flex-col gap-2">
            {isOpen ? (
              confirmingClose ? (
                <div className="flex flex-col gap-1.5 rounded-[--radius] border border-border bg-surface-2 p-2">
                  <span className="px-1 text-[12px] text-text-muted">
                    {t("triage_close_confirm")}
                  </span>
                  <div className="flex items-center gap-1.5">
                    <button
                      type="button"
                      disabled={busy === "state"}
                      onClick={() => {
                        setConfirmingClose(false);
                        handleSetState("closed");
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
                      className="rounded-[--radius-sm] px-2.5 py-1 text-[12px] text-text-muted hover:text-text outline-none focus-visible:ring-2 focus-visible:ring-accent"
                    >
                      {t("triage_cancel")}
                    </button>
                  </div>
                </div>
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
                onClick={() => handleSetState("open")}
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

            <button
              type="button"
              disabled={busy === "assign" || alreadyAssigned || !meLogin}
              onClick={handleAssignMe}
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

            <div className="relative">
              <button
                type="button"
                disabled={busy === "label"}
                onClick={() => {
                  setLabelOpen((v) => !v);
                  setTimeout(() => labelInputRef.current?.focus(), 0);
                }}
                aria-expanded={labelOpen}
                className={ACTION_BTN + " w-full justify-center"}
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
                  className="absolute left-0 right-0 top-full z-10 mt-1.5 rounded-[--radius-lg] border border-border bg-surface-2 p-2 shadow-lg"
                  role="dialog"
                >
                  <form
                    onSubmit={(e) => {
                      e.preventDefault();
                      handleAddLabel(labelText);
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
                </div>
              )}
            </div>
          </div>

          {/* Assignees */}
          <MetaBlock
            icon={<UserPlus size={13} strokeWidth={1.75} aria-hidden />}
            label={t("detail_assignees")}
          >
            {assignees.length === 0 ? (
              <p className="text-[13px] text-text-faint">
                {t("detail_no_assignees")}
              </p>
            ) : (
              <ul className="flex flex-col gap-1.5">
                {assignees.map((a) => (
                  <li key={a.login} className="flex items-center gap-2">
                    <Avatar login={a.login} src={a.avatar_url} size={20} />
                    <span className="text-[13px] text-text">{a.login}</span>
                  </li>
                ))}
              </ul>
            )}
          </MetaBlock>

          {/* Labels */}
          <MetaBlock
            icon={<Tag size={13} strokeWidth={1.75} aria-hidden />}
            label={t("detail_labels")}
          >
            {labels.length === 0 ? (
              <p className="text-[13px] text-text-faint">—</p>
            ) : (
              <div className="flex flex-wrap gap-1.5">
                {labels.map((l) => (
                  <RemovableLabel
                    key={l.name}
                    name={l.name}
                    color={l.color}
                    disabled={busy === "label"}
                    removeTitle={t("triage_remove_label")}
                    onRemove={() => handleRemoveLabel(l.name)}
                  />
                ))}
              </div>
            )}
          </MetaBlock>

          {/* Milestone */}
          {detail.milestone_title && (
            <MetaBlock
              icon={<Milestone size={13} strokeWidth={1.75} aria-hidden />}
              label={t("detail_milestone")}
            >
              <span className="text-[13px] text-text">
                {detail.milestone_title}
              </span>
            </MetaBlock>
          )}
        </aside>
      </div>
    </SectionShell>
  );
}

/** Thin route wrapper for the issue detail route. */
export function IssueDetailView() {
  return <DetailView kind="issue" />;
}

/** Thin route wrapper for the PR detail route. */
export function PullRequestDetailView() {
  return <DetailView kind="pr" />;
}
