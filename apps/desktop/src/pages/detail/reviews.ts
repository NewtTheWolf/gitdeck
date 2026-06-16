import type { Review, ReviewState, User } from "../../lib/api";

/**
 * The "latest" review decision for one reviewer. GitHub lets a reviewer submit
 * multiple reviews over time; for the per-reviewer status we take their most
 * recent NON-pending review (by `submitted_at`). PENDING reviews are not yet
 * submitted and carry no decision, so they're ignored for the summary.
 */
export interface LatestReview {
  reviewer_login: string;
  reviewer_avatar_url: string | null;
  state: ReviewState;
  html_url: string;
  submitted_at: string | null;
}

/** Most-recent submitted review per reviewer (excludes PENDING). */
export function latestReviewsByReviewer(reviews: Review[]): LatestReview[] {
  const byReviewer = new Map<string, LatestReview>();
  for (const r of reviews) {
    if (r.state === "PENDING") continue;
    const prev = byReviewer.get(r.reviewer_login);
    // Higher submitted_at wins; missing timestamps sort as oldest.
    const isNewer =
      !prev ||
      (r.submitted_at ?? "") >= (prev.submitted_at ?? "");
    if (isNewer) {
      byReviewer.set(r.reviewer_login, {
        reviewer_login: r.reviewer_login,
        reviewer_avatar_url: r.reviewer_avatar_url,
        state: r.state,
        html_url: r.html_url,
        submitted_at: r.submitted_at,
      });
    }
  }
  return [...byReviewer.values()];
}

export type ReviewSummary =
  | "changes_requested"
  | "approved"
  | "required"
  | "none";

/**
 * Compute the overall review summary for a PR:
 *   - "changes_requested" if any reviewer's latest decision is CHANGES_REQUESTED
 *   - else "approved" if ≥1 APPROVED and no changes-requested
 *   - else "required" if a review is still requested from someone
 *   - else "none"
 */
export function reviewSummary(
  latest: LatestReview[],
  requestedReviewers: User[],
  requestedTeams: string[],
): ReviewSummary {
  const anyChanges = latest.some((r) => r.state === "CHANGES_REQUESTED");
  if (anyChanges) return "changes_requested";
  const anyApproved = latest.some((r) => r.state === "APPROVED");
  if (anyApproved) return "approved";
  if (requestedReviewers.length > 0 || requestedTeams.length > 0) {
    return "required";
  }
  return "none";
}

/** Whether the active account's login is among the requested reviewers. */
export function isReviewRequestedFromMe(
  requestedReviewers: User[],
  meLogin: string | null,
): boolean {
  if (!meLogin) return false;
  return requestedReviewers.some((r) => r.login === meLogin);
}
