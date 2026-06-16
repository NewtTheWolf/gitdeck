/**
 * Helpers mapping a repo `full_name` ("owner/name") onto the detail route
 * `/repos/:owner/:repo`. Repo names can't contain "/", so we split on the first
 * separator and treat everything after it as the repo segment.
 */
export function splitFullName(fullName: string): { owner: string; repo: string } {
  const idx = fullName.indexOf("/");
  if (idx < 0) return { owner: fullName, repo: "" };
  return { owner: fullName.slice(0, idx), repo: fullName.slice(idx + 1) };
}

/** Build the route path for a repo's detail view from its `full_name`. */
export function repoDetailPath(fullName: string): string {
  const { owner, repo } = splitFullName(fullName);
  return `/repos/${encodeURIComponent(owner)}/${encodeURIComponent(repo)}`;
}
