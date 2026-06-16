import { invoke } from "@tauri-apps/api/core";
import type { QueryClient } from "@tanstack/react-query";
import { isRemote } from "../transport";
import { queryKeys } from "../query/queries";
import {
  pullRemote,
  pushRemote,
  protoToLocal,
  watchRemote,
  type LocalSyncChange,
  type SyncRef,
} from "./wrappers";

// ---------------------------------------------------------------------------
// Persisted cursor
// ---------------------------------------------------------------------------

const CURSOR_KEY = "gitdeck.sync.cursor";

function readCursor(): string {
  try {
    return localStorage.getItem(CURSOR_KEY) ?? "";
  } catch {
    return "";
  }
}

function writeCursor(cursor: string): void {
  try {
    localStorage.setItem(CURSOR_KEY, cursor);
  } catch {
    // localStorage may be unavailable (e.g. in some test envs) — ignore.
  }
}

// ---------------------------------------------------------------------------
// Status store (tiny pub/sub for K4's UI)
// ---------------------------------------------------------------------------

export type SyncState = "idle" | "syncing" | "offline" | "error";

export interface SyncStatus {
  state: SyncState;
  lastSyncedAt: number | null;
}

let status: SyncStatus = { state: "idle", lastSyncedAt: null };
const listeners = new Set<() => void>();

function setStatus(next: Partial<SyncStatus>): void {
  status = { ...status, ...next };
  for (const l of listeners) l();
}

/** Current sync status snapshot. */
export function getSyncStatus(): SyncStatus {
  return status;
}

/** Subscribe to status changes. Returns an unsubscribe fn. */
export function subscribeSyncStatus(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

// ---------------------------------------------------------------------------
// Cache invalidation
// ---------------------------------------------------------------------------

/** Invalidate every synced-entity query so the UI re-reads the local store. */
function invalidateSynced(qc: QueryClient): void {
  void qc.invalidateQueries({ queryKey: queryKeys.tasks() });
  void qc.invalidateQueries({ queryKey: queryKeys.boards() });
  // `board(id)` keys are ["board", id] — invalidate the whole family.
  void qc.invalidateQueries({
    predicate: (q) => q.queryKey[0] === "board",
  });
}

// ---------------------------------------------------------------------------
// syncNow — one push+pull cycle, serialized
// ---------------------------------------------------------------------------

let inFlight: Promise<void> | null = null;
let activeClient: QueryClient | null = null;

/**
 * Run one sync cycle: push local dirty rows, pull remote changes since the
 * cursor, apply them locally, and invalidate caches if anything moved. Only
 * runs in remote mode. Concurrent calls share the in-flight promise so two
 * triggers never overlap.
 */
export function syncNow(queryClient?: QueryClient): Promise<void> {
  const qc = queryClient ?? activeClient ?? undefined;
  if (inFlight) return inFlight;
  inFlight = runSync(qc).finally(() => {
    inFlight = null;
  });
  return inFlight;
}

async function runSync(qc: QueryClient | undefined): Promise<void> {
  if (!isRemote()) return;
  setStatus({ state: "syncing" });
  try {
    let touched = false;

    // (a) Push local dirty rows.
    const dirty = await invoke<LocalSyncChange[]>("sync_local_dirty");
    if (dirty.length > 0) {
      await pushRemote(dirty);
      const refs: SyncRef[] = dirty.map((d) => ({ kind: d.kind, id: d.id }));
      await invoke<void>("sync_mark_synced", { refs });
      touched = true;
    }

    // (b) Pull remote changes since the cursor.
    const cursor = readCursor();
    const { changes, cursor: nextCursor } = await pullRemote(cursor);
    if (changes.length > 0) {
      await invoke<number>("sync_apply_remote", { changes });
      touched = true;
    }
    if (nextCursor && nextCursor !== cursor) {
      writeCursor(nextCursor);
    }

    // (c) Refresh the UI if anything changed.
    if (touched && qc) invalidateSynced(qc);

    setStatus({ state: "idle", lastSyncedAt: Date.now() });
  } catch (err) {
    setStatus({ state: "error" });
    throw err;
  }
}

// ---------------------------------------------------------------------------
// startSync — initial cycle + periodic timer + watch loop
// ---------------------------------------------------------------------------

const PERIODIC_MS = 20_000;
const BACKOFF_MIN_MS = 1_000;
const BACKOFF_MAX_MS = 30_000;

/**
 * Apply a single watched change locally and refresh caches. Cursor advance is
 * left to the next pull cycle (watch carries no cursor).
 */
async function applyWatched(
  qc: QueryClient,
  mapped: LocalSyncChange,
): Promise<void> {
  await invoke<number>("sync_apply_remote", { changes: [mapped] });
  invalidateSynced(qc);
  setStatus({ state: "idle", lastSyncedAt: Date.now() });
}

/**
 * Start the background sync loop for remote mode: an immediate cycle, a periodic
 * timer, and a server-streaming watch (reconnecting with backoff). Returns a
 * stop() that cancels the timer and aborts the stream.
 */
export function startSync(queryClient: QueryClient): () => void {
  activeClient = queryClient;
  let stopped = false;

  // Initial reconcile.
  void syncNow(queryClient).catch(() => {});

  // Periodic pull/push.
  const timer = setInterval(() => {
    void syncNow(queryClient).catch(() => {});
  }, PERIODIC_MS);

  // Watch loop with reconnect/backoff.
  let controller: AbortController | null = null;
  let backoff = BACKOFF_MIN_MS;

  const watchLoop = async () => {
    while (!stopped && isRemote()) {
      controller = new AbortController();
      try {
        for await (const change of watchRemote(controller.signal)) {
          if (stopped) break;
          await applyWatched(queryClient, protoToLocal(change));
          backoff = BACKOFF_MIN_MS; // healthy stream resets backoff
        }
      } catch {
        if (stopped) break;
        setStatus({ state: "offline" });
      }
      if (stopped) break;
      // Stream ended or errored — wait, then reconnect.
      await new Promise((r) => setTimeout(r, backoff));
      backoff = Math.min(backoff * 2, BACKOFF_MAX_MS);
    }
  };
  void watchLoop();

  return () => {
    stopped = true;
    clearInterval(timer);
    controller?.abort();
    if (activeClient === queryClient) activeClient = null;
  };
}
