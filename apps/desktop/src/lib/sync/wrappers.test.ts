import { describe, expect, it } from "vitest";
import type { Change } from "../gen/gitdeck/v1/gitdeck_pb";
import { localToProto, protoToLocal, type LocalSyncChange } from "./wrappers";

describe("sync wrappers mapping", () => {
  it("localToProto: snake_case local change → camelCase proto input", () => {
    const local: LocalSyncChange = {
      kind: "card",
      id: "card-1",
      updated_at: "2026-06-16T00:00:00Z",
      deleted: false,
      data_json: '{"position":0}',
      board_id: "b-1",
      column_id: "c-1",
    };

    expect(localToProto(local)).toEqual({
      kind: "card",
      id: "card-1",
      updatedAt: "2026-06-16T00:00:00Z",
      deleted: false,
      dataJson: '{"position":0}',
      boardId: "b-1",
      columnId: "c-1",
    });
  });

  it("protoToLocal: camelCase proto Change → snake_case local change", () => {
    const proto = {
      $typeName: "gitdeck.v1.Change",
      kind: "task",
      id: "t-1",
      updatedAt: "2026-06-16T01:00:00Z",
      deleted: true,
      dataJson: '{"title":"x"}',
      boardId: "",
      columnId: "",
    } as Change;

    expect(protoToLocal(proto)).toEqual({
      kind: "task",
      id: "t-1",
      updated_at: "2026-06-16T01:00:00Z",
      deleted: true,
      data_json: '{"title":"x"}',
      board_id: "",
      column_id: "",
    });
  });

  it("round-trips local → proto → local", () => {
    const local: LocalSyncChange = {
      kind: "board",
      id: "b-9",
      updated_at: "2026-06-16T02:00:00Z",
      deleted: false,
      data_json: '{"name":"Sprint"}',
      board_id: "",
      column_id: "",
    };
    const proto = { $typeName: "gitdeck.v1.Change", ...localToProto(local) } as Change;
    expect(protoToLocal(proto)).toEqual(local);
  });
});
