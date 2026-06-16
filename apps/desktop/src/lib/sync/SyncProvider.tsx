import { useEffect, useSyncExternalStore } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { isRemote, subscribeServerConfig } from "../transport";
import {
  getSyncStatus,
  startSync,
  subscribeSyncStatus,
  syncNow,
  type SyncStatus,
} from "./engine";

/**
 * React hook over the sync status store. Re-renders on every status change so
 * K4's UI can show synced / syncing / offline / error + lastSyncedAt.
 */
export function useSyncStatus(): SyncStatus {
  return useSyncExternalStore(subscribeSyncStatus, getSyncStatus, getSyncStatus);
}

/**
 * Drives the background sync engine. Mounted once in App. Starts the engine when
 * mode=remote and tears it down/restarts when the server config changes. Also
 * nudges a sync after local mutations (synced board/task writes) so a change
 * propagates without waiting for the periodic timer.
 */
export function SyncProvider({ children }: { children: React.ReactNode }) {
  const queryClient = useQueryClient();

  // Start/stop the engine, re-running whenever the server config changes.
  useEffect(() => {
    let stop: (() => void) | null = null;

    const restart = () => {
      stop?.();
      stop = isRemote() ? startSync(queryClient) : null;
    };

    restart();
    const unsubscribe = subscribeServerConfig(restart);

    return () => {
      unsubscribe();
      stop?.();
    };
  }, [queryClient]);

  // Post-mutation nudge: any successful mutation triggers a sync cycle (a no-op
  // when not in remote mode, and serialized so it can't overlap the timer/watch).
  useEffect(() => {
    const cache = queryClient.getMutationCache();
    const unsubscribe = cache.subscribe((event) => {
      if (event.type === "updated" && event.mutation.state.status === "success") {
        void syncNow(queryClient).catch(() => {});
      }
    });
    return unsubscribe;
  }, [queryClient]);

  return <>{children}</>;
}
