import { getGrpcClient } from "../transport";
import type { Change } from "../gen/gitdeck/v1/gitdeck_pb";

/**
 * A change row as the LOCAL embedded store speaks it (snake_case). This is the
 * exact shape `sync_local_dirty` returns and `sync_apply_remote` expects.
 */
export interface LocalSyncChange {
  kind: string; // "task" | "board" | "column" | "card"
  id: string;
  updated_at: string; // RFC3339; the LWW clock
  deleted: boolean; // tombstone
  data_json: string;
  board_id: string;
  column_id: string;
}

/** A `{kind,id}` reference, as `sync_mark_synced` expects. */
export interface SyncRef {
  kind: string;
  id: string;
}

/**
 * Local snake_case change → proto request input (camelCase). We pass plain
 * objects to the connect client; it builds the message from them.
 */
export function localToProto(c: LocalSyncChange) {
  return {
    kind: c.kind,
    id: c.id,
    updatedAt: c.updated_at,
    deleted: c.deleted,
    dataJson: c.data_json,
    boardId: c.board_id,
    columnId: c.column_id,
  };
}

/** Proto Change (camelCase) → the snake_case shape `sync_apply_remote` wants. */
export function protoToLocal(c: Change): LocalSyncChange {
  return {
    kind: c.kind,
    id: c.id,
    updated_at: c.updatedAt,
    deleted: c.deleted,
    data_json: c.dataJson,
    board_id: c.boardId,
    column_id: c.columnId,
  };
}

/** Push local dirty rows to the server. Returns the new server cursor. */
export async function pushRemote(
  changes: LocalSyncChange[],
): Promise<{ applied: number; serverCursor: string }> {
  const res = await getGrpcClient().pushChanges({
    changes: changes.map(localToProto),
  });
  return { applied: res.applied, serverCursor: res.serverCursor };
}

/** Pull server changes since a cursor. Returns mapped local changes + cursor. */
export async function pullRemote(
  sinceCursor: string,
): Promise<{ changes: LocalSyncChange[]; cursor: string }> {
  const res = await getGrpcClient().pullChanges({ sinceCursor });
  return { changes: res.changes.map(protoToLocal), cursor: res.cursor };
}

/**
 * Open the server-streaming watch. Returns the async iterable of proto Changes;
 * callers map each via `protoToLocal`. `signal` aborts the stream.
 */
export function watchRemote(signal?: AbortSignal): AsyncIterable<Change> {
  return getGrpcClient().watchChanges({}, { signal });
}
