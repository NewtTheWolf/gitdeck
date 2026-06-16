import { useTranslation } from "react-i18next";
import { CircleDot, Inbox } from "lucide-react";
import type { Issue } from "../../lib/api";
import { StateDot } from "./listRow";
import { IssueCard } from "./IssueCard";

function IssueKind({ state }: { state: Issue["state"] }) {
  return (
    <span className="flex items-center gap-1.5">
      <StateDot className={state === "open" ? "bg-open" : "bg-closed"} />
      <CircleDot
        size={13}
        strokeWidth={1.75}
        aria-hidden
        className={state === "open" ? "text-open" : "text-closed"}
      />
    </span>
  );
}

export function IssueList({ issues }: { issues: Issue[] }) {
  const { t } = useTranslation();

  if (!issues.length) {
    return (
      <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
        <Inbox size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
        <p className="text-sm text-text-muted">{t("issues_empty")}</p>
      </div>
    );
  }

  return (
    <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
      {issues.map((issue) => (
        <IssueCard
          key={issue.html_url}
          item={issue}
          kindKey="issue"
          kind={<IssueKind state={issue.state} />}
        />
      ))}
    </div>
  );
}

export default IssueList;
