import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Issue } from "../../lib/api";
import { useIssues } from "../../lib/query/queries";
import {
  DEFAULT_FILTERS,
  distinctRepos,
  filterAndSortItems,
  type DashboardFilters,
} from "../../lib/dashboard";
import { FilterBar } from "../../components/dashboard/FilterBar";
import { IssueList } from "../../components/dashboard/IssueList";
import { CardSkeleton } from "../../components/dashboard/IssueCard";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
  useSectionData,
} from "./sectionScaffold";

export default function Issues() {
  const { t } = useTranslation();
  const { data, loading, error, account, hasAccounts } =
    useSectionData<Issue>(useIssues);
  const [filters, setFilters] = useState<DashboardFilters>(DEFAULT_FILTERS);

  const repos = useMemo(() => distinctRepos(data), [data]);
  const visible = useMemo(
    () => filterAndSortItems(data, filters),
    [data, filters],
  );

  return (
    <SectionShell>
      <SectionHeader title={t("nav_issues")} account={account} />
      {!hasAccounts ? (
        <NoAccount />
      ) : loading ? (
        <CardSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : (
        <>
          <FilterBar filters={filters} onChange={setFilters} repos={repos} />
          <IssueList issues={visible} />
        </>
      )}
    </SectionShell>
  );
}
