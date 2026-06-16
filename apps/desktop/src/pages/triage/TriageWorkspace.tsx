import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { ListChecks } from "lucide-react";
import { type Issue } from "../../lib/api";
import {
  useIssues,
  useSetIssueState,
  useAddIssueLabels,
  useRemoveIssueLabel,
  useAddIssueAssignees,
  useCreateIssueComment,
} from "../../lib/query/queries";
import {
  DEFAULT_FILTERS,
  distinctRepos,
  filterAndSortItems,
  type DashboardFilters,
} from "../../lib/dashboard";
import { splitFullName } from "../../lib/repoRoute";
import { FilterBar } from "../../components/dashboard/FilterBar";
import { CardSkeleton } from "../../components/dashboard/IssueCard";
import { useAccounts } from "../../contexts/AccountContext";
import {
  NoAccount,
  SectionError,
  SectionHeader,
  SectionShell,
} from "../dashboard/sectionScaffold";
import { TriageRow, type BusyAction } from "./TriageRow";

/** Stable identity for an issue across optimistic updates (its web url). */
function keyOf(item: Issue): string {
  return item.html_url;
}

function errMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

export default function TriageWorkspace() {
  const { t } = useTranslation();
  const { accounts, activeAccountId } = useAccounts();
  const activeAccount = accounts.find((a) => a.id === activeAccountId) ?? null;
  // The account's GitHub login IS its display_name (set at OAuth connect time),
  // so we use it as the "me" handle for the Assign-to-me action.
  const meLogin = activeAccount?.display_name ?? null;
  const account = activeAccount?.display_name ?? activeAccountId ?? null;

  const [filters, setFilters] = useState<DashboardFilters>(DEFAULT_FILTERS);

  // Per-row UI state, keyed by the issue's html_url.
  const [busy, setBusy] = useState<Record<string, BusyAction>>({});
  const [rowError, setRowError] = useState<Record<string, string | null>>({});
  const [commentUrl, setCommentUrl] = useState<Record<string, string | null>>({});

  const activeRef = useRef(true);

  // The issue list is a cached query; local `items` layers optimistic triage
  // edits on top so the row reacts instantly while the write runs. We re-seed
  // `items` from the query whenever fresh server data arrives.
  const query = useIssues(activeAccountId);
  const loading = query.isPending && query.fetchStatus !== "idle";
  const error = query.isError ? errMessage(query.error) : null;

  const [items, setItems] = useState<Issue[]>([]);
  useEffect(() => {
    if (Array.isArray(query.data)) setItems(query.data);
  }, [query.data]);

  // Mutation hooks invalidate the issues/pulls caches on success; the local
  // optimistic patch keeps the UI in sync until the refetch lands.
  const setStateMutation = useSetIssueState(activeAccountId);
  const addLabelsMutation = useAddIssueLabels(activeAccountId);
  const removeLabelMutation = useRemoveIssueLabel(activeAccountId);
  const addAssigneesMutation = useAddIssueAssignees(activeAccountId);
  const createCommentMutation = useCreateIssueComment(activeAccountId);

  const repos = useMemo(() => distinctRepos(items), [items]);
  const visible = useMemo(
    () => filterAndSortItems(items, filters),
    [items, filters],
  );
  // Label names seen per repo — offered as quick-add chips in the label popover.
  const labelPoolByRepo = useMemo(() => {
    const map: Record<string, Set<string>> = {};
    for (const it of items) {
      const set = (map[it.repo_name_with_owner] ??= new Set());
      for (const l of it.labels || []) set.add(l.name);
    }
    return map;
  }, [items]);

  const setRowBusy = (key: string, action: BusyAction) =>
    setBusy((b) => ({ ...b, [key]: action }));

  const patchItem = useCallback(
    (key: string, patch: (it: Issue) => Issue) => {
      setItems((prev) =>
        prev.map((it) => (keyOf(it) === key ? patch(it) : it)),
      );
    },
    [],
  );

  /**
   * Run a write action with an optimistic patch + automatic revert on failure.
   * `optimistic` is applied immediately; on rejection we restore `before` and
   * surface the error on the row. Guards against unmount via `activeRef`.
   */
  const runAction = useCallback(
    async (
      key: string,
      action: BusyAction,
      optimistic: (it: Issue) => Issue,
      call: () => Promise<unknown>,
    ) => {
      const before = items.find((it) => keyOf(it) === key);
      if (!before) return;
      setRowError((e) => ({ ...e, [key]: null }));
      setRowBusy(key, action);
      patchItem(key, optimistic);
      try {
        await call();
      } catch (err) {
        if (!activeRef.current) return;
        // Revert the optimistic change to exactly the prior snapshot.
        setItems((prev) => prev.map((it) => (keyOf(it) === key ? before : it)));
        setRowError((e) => ({ ...e, [key]: errMessage(err) }));
      } finally {
        if (activeRef.current) setRowBusy(key, null);
      }
    },
    [items, patchItem],
  );

  function coords(it: Issue) {
    const { owner, repo } = splitFullName(it.repo_name_with_owner);
    return { id: activeAccountId as string, owner, repo, number: it.number };
  }

  const handleSetState = (it: Issue, next: Issue["state"]) => {
    if (!activeAccountId) return;
    const key = keyOf(it);
    void runAction(
      key,
      "state",
      (cur) => ({ ...cur, state: next }),
      () => setStateMutation.mutateAsync({ ...coords(it), state: next }),
    );
  };

  const handleAssignMe = (it: Issue) => {
    if (!activeAccountId || !meLogin) return;
    const key = keyOf(it);
    if ((it.assignees || []).includes(meLogin)) return;
    void runAction(
      key,
      "assign",
      (cur) => ({ ...cur, assignees: [...(cur.assignees || []), meLogin] }),
      () =>
        addAssigneesMutation.mutateAsync({
          ...coords(it),
          assignees: [meLogin],
        }),
    );
  };

  const handleAddLabel = (it: Issue, label: string) => {
    if (!activeAccountId) return;
    const key = keyOf(it);
    if ((it.labels || []).some((l) => l.name === label)) return;
    void runAction(
      key,
      "label",
      (cur) => ({
        ...cur,
        labels: [...(cur.labels || []), { name: label, color: "" }],
      }),
      () => addLabelsMutation.mutateAsync({ ...coords(it), labels: [label] }),
    );
  };

  const handleRemoveLabel = (it: Issue, label: string) => {
    if (!activeAccountId) return;
    const key = keyOf(it);
    void runAction(
      key,
      "label",
      (cur) => ({
        ...cur,
        labels: (cur.labels || []).filter((l) => l.name !== label),
      }),
      () => removeLabelMutation.mutateAsync({ ...coords(it), label }),
    );
  };

  const handleComment = (it: Issue, body: string) => {
    if (!activeAccountId) return;
    const key = keyOf(it);
    setRowError((e) => ({ ...e, [key]: null }));
    setRowBusy(key, "comment");
    createCommentMutation
      .mutateAsync({ ...coords(it), body })
      .then((url) => {
        if (!activeRef.current) return;
        setCommentUrl((c) => ({ ...c, [key]: url }));
        // Reflect the new comment in the local count.
        patchItem(key, (cur) => ({
          ...cur,
          comments_count: cur.comments_count + 1,
        }));
      })
      .catch((err) => {
        if (!activeRef.current) return;
        setRowError((e) => ({ ...e, [key]: errMessage(err) }));
      })
      .finally(() => {
        if (activeRef.current) setRowBusy(key, null);
      });
  };

  return (
    <SectionShell>
      <SectionHeader title={t("triage_title")} account={account} />

      {accounts.length === 0 ? (
        <NoAccount />
      ) : loading ? (
        <CardSkeleton />
      ) : error ? (
        <SectionError message={error} />
      ) : (
        <>
          <FilterBar filters={filters} onChange={setFilters} repos={repos} />
          {visible.length === 0 ? (
            <div className="flex flex-col items-start gap-2 rounded-[--radius-lg] border border-border bg-surface px-6 py-12">
              <ListChecks
                size={20}
                strokeWidth={1.75}
                aria-hidden
                className="text-text-faint"
              />
              <p className="text-sm text-text-muted">{t("triage_empty")}</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-3 xl:grid-cols-2">
              {visible.map((it) => {
                const key = keyOf(it);
                return (
                  <TriageRow
                    key={key}
                    item={it}
                    meLogin={meLogin}
                    repoLabelPool={[
                      ...(labelPoolByRepo[it.repo_name_with_owner] ?? []),
                    ]}
                    busy={busy[key] ?? null}
                    error={rowError[key] ?? null}
                    commentUrl={commentUrl[key] ?? null}
                    onSetState={(next) => handleSetState(it, next)}
                    onAssignMe={() => handleAssignMe(it)}
                    onAddLabel={(label) => handleAddLabel(it, label)}
                    onRemoveLabel={(label) => handleRemoveLabel(it, label)}
                    onComment={(body) => handleComment(it, body)}
                    onDismissError={() =>
                      setRowError((e) => ({ ...e, [key]: null }))
                    }
                  />
                );
              })}
            </div>
          )}
        </>
      )}
    </SectionShell>
  );
}
