import { stateSchema, demoTaskSchema } from "./model";
import type { DemoState, DemoTask } from "./model";
import { createFixtures } from "./fixtures";

export const STORAGE_KEY = "altius.web.demo.v1";
export interface DemoAdapter {
  load(): Promise<DemoState>;
  save(state: DemoState): void;
  reset(): DemoState;
}
export const createLocalAdapter = (storage: Pick<Storage, "getItem" | "setItem" | "removeItem">): DemoAdapter => ({
  async load() {
    const raw = storage.getItem(STORAGE_KEY);
    if (!raw) return createFixtures();
    let parsed: unknown;
    try {
      parsed = JSON.parse(raw);
    } catch {
      throw new Error("Stored demo data is not valid JSON.");
    }
    const result = stateSchema.safeParse(parsed);
    if (!result.success) throw new Error("Saved demo data is incompatible. Reset the demo to load a clean synthetic dataset.");
    return result.data;
  },
  save(state) { storage.setItem(STORAGE_KEY, JSON.stringify(stateSchema.parse(state))); },
  reset() { storage.removeItem(STORAGE_KEY); return createFixtures(); }
});

export const filterTasks = (tasks: readonly DemoTask[], filters: { hub: string; search?: string; status?: string; assignee?: string; flow?: string; date?: string }) => tasks.filter(task =>
  task.hub === filters.hub && (!filters.search || `${task.title} ${task.id} ${task.address}`.toLowerCase().includes(filters.search.toLowerCase())) &&
  (!filters.status || task.status === filters.status) && (!filters.assignee || task.assignee === filters.assignee) &&
  (!filters.flow || task.flow === filters.flow) && (!filters.date || task.date === filters.date)
);
export const validateTask = (task: DemoTask) => {
  const parsed = demoTaskSchema.parse(task);
  if (parsed.status !== "unassigned" && !parsed.assignee) throw new Error("Assign a driver before changing the task status.");
  return parsed;
};
export const csvCell = (value: unknown): string => {
  const text = String(value ?? "");
  return `"${(/^[=+@\-\t\r]/.test(text) ? `'${text}` : text).replaceAll('"', '""')}"`;
};
export const tasksCsv = (tasks: readonly DemoTask[]) => [
  ["ID", "Title", "Address", "Hub", "Status", "Driver", "Date"].map(csvCell).join(","),
  ...tasks.map(task => [task.id, task.title, task.address, task.hub, task.status, task.assignee, task.date].map(csvCell).join(","))
].join("\r\n");
export const downloadText = (name: string, text: string, type = "text/csv;charset=utf-8") => {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const anchor = document.createElement("a");
  anchor.href = url; anchor.download = name; anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
};
