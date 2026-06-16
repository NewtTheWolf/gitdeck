import { QueryClient } from "@tanstack/react-query";
import { createSyncStoragePersister } from "@tanstack/query-sync-storage-persister";

/** One day, in milliseconds — used for both the in-memory GC window and the
 * persisted-cache max age, so revisits across restarts stay instant for a day. */
export const ONE_DAY = 1000 * 60 * 60 * 24;

/**
 * The app-wide query client. Defaults favour a desktop app: data is considered
 * fresh for a minute (so navigating between views reuses the cache without a
 * refetch), kept for a day, retried once, and never refetched on window focus
 * (a desktop window gains/loses focus constantly). We do refetch on reconnect
 * so a dropped network heals itself.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60_000,
      gcTime: ONE_DAY,
      retry: 1,
      refetchOnWindowFocus: false,
      refetchOnReconnect: true,
    },
  },
});

/**
 * Persists the cache to localStorage so a relaunch paints the last-seen data
 * immediately while a background revalidate runs (stale-while-revalidate).
 * `localStorage` is always available in the Tauri webview.
 */
export const persister = createSyncStoragePersister({
  storage: typeof window !== "undefined" ? window.localStorage : undefined,
  key: "newt-todo-query-cache",
});
