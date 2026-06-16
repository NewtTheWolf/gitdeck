/**
 * Helpers mapping an issue / pull-request (its repo `full_name` + number) onto
 * the in-app detail routes:
 *   /repos/:owner/:repo/issues/:number   (issue)
 *   /repos/:owner/:repo/pull/:number      (PR)
 *
 * Mirrors GitHub's own URL scheme (`/issues/N` vs `/pull/N`). Segments are
 * URL-encoded; the repo full_name is split via the shared `splitFullName`.
 */
import { splitFullName } from "./repoRoute";

/** Build the in-app issue detail path from a repo `full_name` + issue number. */
export function issueDetailPath(repoFullName: string, number: number): string {
  const { owner, repo } = splitFullName(repoFullName);
  return `/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/issues/${number}`;
}

/** Build the in-app PR detail path from a repo `full_name` + PR number. */
export function pullDetailPath(repoFullName: string, number: number): string {
  const { owner, repo } = splitFullName(repoFullName);
  return `/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}/pull/${number}`;
}
