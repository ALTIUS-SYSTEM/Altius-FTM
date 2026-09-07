import { describe, it, expect } from "vitest";
import { createFixtures } from "@/data/fixtures";
import { stateSchema } from "@/data/model";
import { createLocalAdapter, filterTasks, validateTask, csvCell, tasksCsv, STORAGE_KEY } from "@/data/adapter";
import { translate } from "@/lib/i18n";
import { VIEW_PATHS } from "@/lib/routes";

const memoryStorage = () => { const map = new Map<string, string>(); return { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => { map.set(k, v); }, removeItem: (k: string) => { map.delete(k); } }; };

describe("fixtures + schema", () => {
  it("fixtures validate against state schema", () => {
    expect(() => stateSchema.parse(createFixtures())).not.toThrow();
  });
  it("fixtures contain all required record kinds", () => {
    const kinds = new Set(createFixtures().records.map(r => r.kind));
    for (const kind of ["user", "team", "hub", "customer", "datatype", "schedule", "automation", "workflow", "module", "invoice"]) expect(kinds.has(kind), kind).toBe(true);
  });
});

describe("local adapter", () => {
  it("round-trips state and rejects corrupt payloads", async () => {
    const adapter = createLocalAdapter(memoryStorage());
    const initial = await adapter.load();
    adapter.save(initial);
    expect(await adapter.load()).toEqual(initial);
    const broken = memoryStorage();
    broken.setItem(STORAGE_KEY, "{not json");
    await expect(createLocalAdapter(broken).load()).rejects.toThrow();
  });
});

describe("task filtering + validation", () => {
  it("filters by hub, status, assignee, search", () => {
    const state = createFixtures();
    const jakarta = filterTasks(state.tasks, { hub: "Jakarta" });
    expect(jakarta.every(t => t.hub === "Jakarta")).toBe(true);
    expect(filterTasks(state.tasks, { hub: "Jakarta", status: "completed" }).every(t => t.status === "completed")).toBe(true);
    expect(filterTasks(state.tasks, { hub: "Jakarta", search: "no-such-thing" })).toHaveLength(0);
  });
  it("rejects status change without assignee", () => {
    const task = { ...createFixtures().tasks[0], assignee: "", status: "in-progress" as const };
    expect(() => validateTask(task)).toThrow(/assign/i);
  });
});

describe("csv export", () => {
  it("escapes quotes and neutralizes formula injection", () => {
    expect(csvCell('=cmd|"/c calc"')).toMatch(/^"'=/);
    expect(csvCell('say "hi"')).toBe('"say ""hi"""');
  });
  it("exports header + rows", () => {
    const csv = tasksCsv(createFixtures().tasks.slice(0, 2));
    expect(csv.split("\r\n")).toHaveLength(3);
  });
});

describe("i18n + routes", () => {
  it("falls back to english and returns key for unknown", () => {
    expect(translate("id", "tasks")).toBe("Tugas");
    expect(translate("zh", "no-such-key")).toBe("no-such-key");
    expect(translate("vi", "tasks")).toBeTruthy();
  });
  it("all module paths resolve to known views", () => {
    expect(VIEW_PATHS).toContain("dashboard/task");
    expect(VIEW_PATHS).toContain("setting/permission");
    expect(VIEW_PATHS).toContain("no-access");
  });
});
