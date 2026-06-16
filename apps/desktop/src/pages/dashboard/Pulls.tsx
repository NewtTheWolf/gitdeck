import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { PullRequest } from "../../lib/api";
import { usePulls } from "../../lib/query/queries";
import {
  DEFAULT_FILTERS,
  distinctRepos,
  filterAndSortItems,
  type DashboardFilters,
} from "../../lib/dashboard";
import { FilterBar } from "../../components/dashboard/FilterBar";
import { PullRequestList } from "../../components/dashboard/PullRequestList";
import { CardSkeleton } from "../../components/dashboard/IssueCard";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
  useSectionData,
} from "./sectionScaffold";

export default function Pulls() {
  const { t } = useTranslation();
  const { data, loading, error, account, hasAccounts } =
    useSectionData<PullRequest>(usePulls);
  const [filters, setFilters] = useState<DashboardFilters>(DEFAULT_FILTERS);

  const repos = useMemo(() => distinctRepos(data), [data]);
  const visible = useMemo(
    () => filterAndSortItems(data, filters),
    [data, filters],
  );

  return (
    <SectionShell>
      <SectionHeader title={t("nav_pulls")} account={account} />
      {!hasAccounts ? (
        <NoAccount />
      ) : loading ? (
        <CardSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : (
        <>
          <FilterBar filters={filters} onChange={setFilters} repos={repos} />
          <PullRequestList pullRequests={visible} />
        </>
      )}
    </SectionShell>
  );
}
