import { describe, expect, it } from "vitest";
import { countOpen, filterTasks, type Filter } from "./filter";
import type { TaskDto } from "./api";

const t = (id: string, status: "open" | "done"): TaskDto => ({
  id, title: id, body: "", status, labels: [], due_at: null,
  updated_at: "2026-06-15T00:00:00Z", source_url: null,
});

describe("filterTasks", () => {
  const tasks = [t("a", "open"), t("b", "done"), t("c", "open")];
  it("returns all when filter=all", () => {
    expect(filterTasks(tasks, "all" as Filter)).toHaveLength(3);
  });
  it("filters open", () => {
    expect(filterTasks(tasks, "open").map((x) => x.id)).toEqual(["a", "c"]);
  });
  it("filters done", () => {
    expect(filterTasks(tasks, "done").map((x) => x.id)).toEqual(["b"]);
  });
});

describe("countOpen", () => {
  it("counts open tasks", () => {
    expect(countOpen([t("a", "open"), t("b", "done"), t("c", "open")])).toBe(2);
  });
});
