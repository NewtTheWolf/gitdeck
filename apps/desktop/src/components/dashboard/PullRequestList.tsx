import { useTranslation } from "react-i18next";
import { GitPullRequest, Inbox } from "lucide-react";
import type { PullRequest } from "../../lib/api";
import { StateDot } from "./listRow";
import { IssueCard } from "./IssueCard";

function PullRequestKind({ pr }: { pr: PullRequest }) {
  // open=accent-neutral green, closed=red. A draft reads as muted.
  const tone = pr.is_draft
    ? "text-text-faint"
    : pr.state === "open"
      ? "text-open"
      : "text-closed";
  const dot = pr.is_draft
    ? "bg-text-faint"
    : pr.state === "open"
      ? "bg-open"
      : "bg-closed";
  return (
    <span className="flex items-center gap-1.5">
      <StateDot className={dot} />
      <GitPullRequest size={13} strokeWidth={1.75} aria-hidden className={tone} />
    </span>
  );
}

function DraftBadge() {
  const { t } = useTranslation();
  return (
    <span className="inline-flex items-center rounded-full border border-border bg-surface-2 px-1.5 py-0.5 text-[11px] font-medium uppercase tracking-wide text-text-faint">
      {t("draft_badge")}
    </span>
  );
}

export function PullRequestList({
  pullRequests,
}: {
  pullRequests: PullRequest[];
}) {
  const { t } = useTranslation();

  if (!pullRequests.length) {
    return (
      <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
        <Inbox size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
        <p className="text-sm text-text-muted">{t("pulls_empty")}</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {pullRequests.map((pr) => (
        <IssueCard
          key={pr.html_url}
          item={pr}
          kindKey="pr"
          kind={<PullRequestKind pr={pr} />}
          extraMeta={pr.is_draft ? <DraftBadge /> : null}
        />
      ))}
    </div>
  );
}

export default PullRequestList;
