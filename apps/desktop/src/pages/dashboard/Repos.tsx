import { useTranslation } from "react-i18next";
import { Inbox } from "lucide-react";
import type { Repo } from "../../lib/api";
import { useRepos } from "../../lib/query/queries";
import RepoGrid, { RepoGridSkeleton } from "../../components/dashboard/RepoGrid";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
  useSectionData,
} from "./sectionScaffold";

export default function Repos() {
  const { t } = useTranslation();
  const { data, loading, error, account, hasAccounts } =
    useSectionData<Repo>(useRepos);

  return (
    <SectionShell>
      <SectionHeader title={t("nav_repos")} account={account} />
      {!hasAccounts ? (
        <NoAccount />
      ) : loading ? (
        <RepoGridSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : data.length === 0 ? (
        <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
          <Inbox size={20} strokeWidth={1.75} aria-hidden className="text-text-faint" />
          <p className="text-sm text-text-muted">{t("dashboard_no_repos")}</p>
        </div>
      ) : (
        <RepoGrid repos={data} />
      )}
    </SectionShell>
  );
}
