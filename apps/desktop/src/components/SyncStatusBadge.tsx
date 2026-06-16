import { useTranslation } from "react-i18next";
import { useSyncStatus } from "../lib/sync/SyncProvider";

/**
 * Relative-time formatter for the "synced N ago" label. Falls back to "just now"
 * for very recent syncs; degrades to coarse units otherwise.
 */
function relativeTime(ms: number): string {
  const seconds = Math.max(0, Math.floor((Date.now() - ms) / 1000));
  if (seconds < 5) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.floor(hours / 24);
  return `${days}d ago`;
}

const DOT: Record<string, string> = {
  idle: "bg-open",
  syncing: "bg-accent",
  offline: "bg-text-faint",
  error: "bg-closed",
};

/**
 * Compact sync indicator: a colored dot + label reflecting the background sync
 * engine status. Only meaningful in remote mode; in local mode it shows
 * "Local only" with a muted dot. Shared by Settings and the Shell sidebar.
 */
export function SyncStatusBadge({
  remote,
  compact = false,
}: {
  remote: boolean;
  compact?: boolean;
}) {
  const { t } = useTranslation();
  const { state, lastSyncedAt } = useSyncStatus();

  if (!remote) {
    return (
      <span className="inline-flex items-center gap-1.5 text-xs text-text-faint">
        <span
          aria-hidden
          className="size-1.5 shrink-0 rounded-full bg-text-faint"
        />
        {t("sync_local_only")}
      </span>
    );
  }

  let label: string;
  switch (state) {
    case "syncing":
      label = t("sync_status_syncing");
      break;
    case "offline":
      label = t("sync_status_offline");
      break;
    case "error":
      label = t("sync_status_error");
      break;
    default:
      label =
        lastSyncedAt != null
          ? t("sync_last_synced", { when: relativeTime(lastSyncedAt) })
          : t("sync_status_idle");
  }

  return (
    <span
      className={
        "inline-flex items-center gap-1.5 " +
        (compact ? "text-[11px] text-text-muted" : "text-xs text-text-muted")
      }
      title={label}
    >
      <span
        aria-hidden
        className={
          "size-1.5 shrink-0 rounded-full " +
          (DOT[state] ?? "bg-text-faint") +
          (state === "syncing" ? " animate-pulse" : "")
        }
      />
      {label}
    </span>
  );
}
