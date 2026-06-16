import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));
const { getGrpcClient, isRemote } = vi.hoisted(() => ({
  getGrpcClient: vi.fn(),
  isRemote: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...a: unknown[]) => invoke(...a),
}));
vi.mock("../transport", () => ({
  getGrpcClient: () => getGrpcClient(),
  isRemote: () => isRemote(),
  subscribeServerConfig: () => () => {},
}));

import { syncNow, startSync, getSyncStatus } from "./engine";

const CURSOR_KEY = "gitdeck.sync.cursor";

/** A queryClient stub recording invalidateQueries calls. */
function fakeClient() {
  const invalidateQueries = vi.fn();
  return { invalidateQueries } as unknown as import("@tanstack/react-query").QueryClient & {
    invalidateQueries: ReturnType<typeof vi.fn>;
  };
}

const dirtyChange = {
  kind: "task",
  id: "t-1",
  updated_at: "2026-06-16T00:00:00Z",
  deleted: false,
  data_json: '{"title":"x"}',
  board_id: "",
  column_id: "",
};

const remoteChange = {
  $typeName: "gitdeck.v1.Change",
  kind: "board",
  id: "b-1",
  updatedAt: "2026-06-16T03:00:00Z",
  deleted: false,
  dataJson: '{"name":"Sprint"}',
  boardId: "",
  columnId: "",
};

describe("sync engine: syncNow", () => {
  beforeEach(() => {
    invoke.mockReset();
    getGrpcClient.mockReset();
    isRemote.mockReset().mockReturnValue(true);
    localStorage.clear();
  });

  it("pushes dirty, marks synced, applies pulled changes, persists cursor, invalidates caches", async () => {
    // sync_local_dirty → 1 dirty; sync_mark_synced → void; sync_apply_remote → 1.
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "sync_local_dirty") return Promise.resolve([dirtyChange]);
      if (cmd === "sync_mark_synced") return Promise.resolve(undefined);
      if (cmd === "sync_apply_remote") return Promise.resolve(1);
      return Promise.resolve(undefined);
    });

    const pushChanges = vi
      .fn()
      .mockResolvedValue({ applied: 1, serverCursor: "srv-1" });
    const pullChanges = vi
      .fn()
      .mockResolvedValue({ changes: [remoteChange], cursor: "cur-2" });
    getGrpcClient.mockReturnValue({ pushChanges, pullChanges });

    const qc = fakeClient();
    await syncNow(qc);

    // (a) pushed the dirty change (mapped to camelCase) ...
    expect(pushChanges).toHaveBeenCalledWith({
      changes: [
        {
          kind: "task",
          id: "t-1",
          updatedAt: "2026-06-16T00:00:00Z",
          deleted: false,
          dataJson: '{"title":"x"}',
          boardId: "",
          columnId: "",
        },
      ],
    });
    // ... and marked it synced.
    expect(invoke).toHaveBeenCalledWith("sync_mark_synced", {
      refs: [{ kind: "task", id: "t-1" }],
    });

    // (b) pulled with the empty initial cursor, applied the mapped change.
    expect(pullChanges).toHaveBeenCalledWith({ sinceCursor: "" });
    expect(invoke).toHaveBeenCalledWith("sync_apply_remote", {
      changes: [
        {
          kind: "board",
          id: "b-1",
          updated_at: "2026-06-16T03:00:00Z",
          deleted: false,
          data_json: '{"name":"Sprint"}',
          board_id: "",
          column_id: "",
        },
      ],
    });

    // cursor persisted.
    expect(localStorage.getItem(CURSOR_KEY)).toBe("cur-2");

    // caches invalidated (tasks + boards + board family).
    expect(qc.invalidateQueries).toHaveBeenCalled();

    // status reflects a successful sync.
    expect(getSyncStatus().state).toBe("idle");
    expect(getSyncStatus().lastSyncedAt).toBeTypeOf("number");
  });

  it("no-op when not remote", async () => {
    isRemote.mockReturnValue(false);
    const qc = fakeClient();
    await syncNow(qc);
    expect(invoke).not.toHaveBeenCalled();
  });

  it("skips push and mark when there is nothing dirty", async () => {
    invoke.mockImplementation((cmd: string) => {
      if (cmd === "sync_local_dirty") return Promise.resolve([]);
      return Promise.resolve(undefined);
    });
    const pushChanges = vi.fn();
    const pullChanges = vi
      .fn()
      .mockResolvedValue({ changes: [], cursor: "" });
    getGrpcClient.mockReturnValue({ pushChanges, pullChanges });

    await syncNow(fakeClient());

    expect(pushChanges).not.toHaveBeenCalled();
    expect(invoke).not.toHaveBeenCalledWith(
      "sync_mark_synced",
      expect.anything(),
    );
  });
});

describe("sync engine: startSync watch loop", () => {
  beforeEach(() => {
    invoke.mockReset().mockResolvedValue(undefined);
    getGrpcClient.mockReset();
    isRemote.mockReset().mockReturnValue(true);
    localStorage.clear();
  });

  it("applies a watched change locally and invalidates caches", async () => {
    // One watched change, then the stream ends.
    async function* stream() {
      yield remoteChange;
    }

    invoke.mockImplementation((cmd: string) => {
      if (cmd === "sync_local_dirty") return Promise.resolve([]);
      if (cmd === "sync_apply_remote") return Promise.resolve(1);
      return Promise.resolve(undefined);
    });

    const watchChanges = vi.fn().mockReturnValue(stream());
    getGrpcClient.mockReturnValue({
      pushChanges: vi.fn(),
      pullChanges: vi.fn().mockResolvedValue({ changes: [], cursor: "" }),
      watchChanges,
    });

    const qc = fakeClient();
    const stop = startSync(qc);

    // Let the initial syncNow + watch microtasks flush.
    await vi.waitFor(() => {
      expect(invoke).toHaveBeenCalledWith("sync_apply_remote", {
        changes: [
          {
            kind: "board",
            id: "b-1",
            updated_at: "2026-06-16T03:00:00Z",
            deleted: false,
            data_json: '{"name":"Sprint"}',
            board_id: "",
            column_id: "",
          },
        ],
      });
    });

    expect(qc.invalidateQueries).toHaveBeenCalled();
    stop();
  });
});
