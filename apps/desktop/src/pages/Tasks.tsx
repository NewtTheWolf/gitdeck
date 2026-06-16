import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Plus, Trash2, ListTodo, AlertCircle } from "lucide-react";
import { api, type TaskDto } from "../lib/api";
import { countOpen, filterTasks, type Filter } from "../lib/filter";
import { Button } from "../components/ui/Button";
import { Checkbox } from "../components/ui/Checkbox";

const filters: Filter[] = ["all", "open", "done"];

const inputClass =
  "w-full rounded-[--radius] border border-border bg-surface-2 px-3 py-2 text-sm text-text " +
  "outline-none transition-[color,border-color] duration-150 ease-out placeholder:text-text-faint " +
  "focus-visible:border-accent focus-visible:ring-2 focus-visible:ring-accent " +
  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none";

export default function Tasks() {
  const { t } = useTranslation();
  const [tasks, setTasks] = useState<TaskDto[]>([]);
  const [filter, setFilter] = useState<Filter>("all");
  const [newTitle, setNewTitle] = useState("");
  const [error, setError] = useState<string | null>(null);

  const visible = useMemo(() => filterTasks(tasks, filter), [tasks, filter]);
  const openCount = useMemo(() => countOpen(tasks), [tasks]);

  async function load() {
    try {
      setTasks(await api.listTasks());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function add() {
    const title = newTitle.trim();
    if (!title) return;
    try {
      await api.createTask({ title });
      setNewTitle("");
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  async function toggle(task: TaskDto) {
    try {
      if (task.status === "open") await api.completeTask(task.id);
      else await api.updateTask({ id: task.id, status: "open" });
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  async function remove(task: TaskDto) {
    try {
      await api.deleteTask(task.id);
      await load();
    } catch (e) {
      setError(String(e));
    }
  }

  function filterLabel(f: Filter) {
    return f === "all"
      ? t("filter_all")
      : f === "open"
        ? t("filter_open")
        : t("filter_done");
  }

  return (
    <div className="mx-auto max-w-2xl px-8 py-10">
      <header className="mb-6">
        <h1 className="text-[22px] font-semibold tracking-tight text-text">
          {t("app_title")}
        </h1>
      </header>

      <form
        className="mb-5 flex gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          add();
        }}
      >
        <input
          className={inputClass + " flex-1"}
          placeholder={t("new_task_placeholder")}
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
        />
        <Button variant="primary" type="submit" disabled={!newTitle.trim()}>
          <Plus size={16} strokeWidth={1.75} aria-hidden />
          {t("add_task")}
        </Button>
      </form>

      <div className="mb-3 flex items-center justify-between">
        <div className="flex gap-0.5">
          {filters.map((f) => (
            <button
              key={f}
              type="button"
              className={
                "rounded-[--radius-sm] px-2.5 py-1 text-[13px] " +
                "transition-[color,background-color] duration-150 ease-out " +
                "outline-none focus-visible:ring-2 focus-visible:ring-accent " +
                "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none " +
                (filter === f
                  ? "bg-surface-2 font-medium text-text"
                  : "text-text-muted hover:text-text")
              }
              onClick={() => setFilter(f)}
            >
              {filterLabel(f)}
            </button>
          ))}
        </div>
        <span className="tnum text-xs text-text-faint">
          {t("tasks_remaining", { count: openCount })}
        </span>
      </div>

      {error && (
        <div className="mb-3 flex items-start gap-2 rounded-[--radius] border border-border bg-surface-2 px-3 py-2 text-sm text-text">
          <AlertCircle
            size={15}
            strokeWidth={1.75}
            aria-hidden
            className="mt-0.5 shrink-0 text-closed"
          />
          <span className="break-words">{error}</span>
        </div>
      )}

      {visible.length === 0 ? (
        <div className="flex flex-col items-start gap-2 py-12">
          <ListTodo
            size={20}
            strokeWidth={1.75}
            aria-hidden
            className="text-text-faint"
          />
          <p className="text-sm text-text-muted">{t("empty_state")}</p>
        </div>
      ) : (
        <ul className="-mx-2">
          {visible.map((task) => (
            <li
              key={task.id}
              className="group flex items-center gap-3 rounded-[--radius] px-2 py-2 transition-colors duration-150 ease-out hover:bg-surface-2 motion-reduce:transition-none"
            >
              <Checkbox
                checked={task.status === "done"}
                onChange={() => toggle(task)}
                aria-label={task.title}
              />
              <span
                className={
                  "flex-1 text-sm " +
                  (task.status === "done"
                    ? "text-text-faint line-through"
                    : "text-text")
                }
              >
                {task.title}
              </span>
              <button
                type="button"
                aria-label={t("delete_task")}
                title={t("delete_task")}
                className={
                  "flex size-7 items-center justify-center rounded-[--radius-sm] text-text-faint opacity-0 " +
                  "transition-[opacity,color] duration-150 ease-out hover:text-closed group-hover:opacity-100 " +
                  "outline-none focus-visible:opacity-100 focus-visible:ring-2 focus-visible:ring-accent " +
                  "focus-visible:ring-offset-2 focus-visible:ring-offset-canvas motion-reduce:transition-none"
                }
                onClick={() => remove(task)}
              >
                <Trash2 size={15} strokeWidth={1.75} aria-hidden />
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
