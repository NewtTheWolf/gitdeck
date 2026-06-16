import type { TaskDto } from "./api";

export type Filter = "all" | "open" | "done";

export function filterTasks(tasks: TaskDto[], filter: Filter): TaskDto[] {
  if (filter === "all") return tasks;
  return tasks.filter((t) => t.status === filter);
}

export function countOpen(tasks: TaskDto[]): number {
  return tasks.filter((t) => t.status === "open").length;
}
