"use client";

import type { DemoAdapter } from "./adapter";
import { createLocalAdapter } from "./adapter";
import type { DemoState, Task, TaskStatus } from "./model";
import { createFixtures } from "./fixtures";

/**
 * Live adapter: task data comes from the Altius API; non-task preferences
 * still persist locally so UI settings survive reloads. Selected when
 * `NEXT_PUBLIC_API_BASE` is configured and the caller is authenticated.
 */
export const apiBase = () => process.env.NEXT_PUBLIC_API_BASE ?? "";

interface TaskDoc {
  task?: Record<string, unknown>;
  stop?: Record<string, unknown>;
  stops?: Array<{ stop?: Record<string, unknown> } | Record<string, unknown>>;
  [key: string]: unknown;
}

const leaf = (v: unknown): unknown => {
  if (v && typeof v === "object" && "value" in v) {
    return (v as { value: unknown }).value;
  }
  if (Array.isArray(v) && v.length === 1) {
    return leaf(v[0]);
  }
  return v;
};

const flatten = (obj: Record<string, unknown>): Record<string, unknown> =>
  Object.fromEntries(Object.entries(obj).map(([k, v]) => [k, leaf(v)]));

const mapStatus = (stage: string): TaskStatus => {
  switch (stage) {
    case "completed":
      return "completed";
    case "in_progress":
      return "in-progress";
    case "unassigned":
      return "unassigned";
    case "cancelled":
    case "failed":
      return "failed";
    default:
      return "assigned";
  }
};

const normalizeStops = (doc: TaskDoc): Record<string, unknown>[] => {
  const stops = doc.stops ?? [];
  const out: Record<string, unknown>[] = [];
  for (const s of stops) {
    if (!s || typeof s !== "object") continue;
    const withStop = "stop" in s ? (s as { stop: Record<string, unknown> }).stop : (s as Record<string, unknown>);
    if (withStop) out.push(flatten(withStop));
  }
  return out;
};

const toTask = (doc: TaskDoc): Task => {
  const root = doc.task ? flatten(doc.task) : flatten(doc);
  const stops = normalizeStops(doc);
  const firstStop = stops[0];
  return {
    id: String(root["task-id"] ?? root.id ?? ""),
    title: String(root.title ?? ""),
    address: String(firstStop?.address ?? firstStop?.name ?? ""),
    hub: String(root["hub-id"] ?? ""),
    flow: String(root.flow ?? "Delivery"),
    assignee: String(root.assignee ?? ""),
    status: mapStatus(String(root.stage ?? "assigned")),
    date: String(root.day ?? ""),
    time: String(root.time ?? ""),
    priority: ["Normal", "High"].includes(String(root.priority)) ? (String(root.priority) as Task["priority"]) : "Normal",
    notes: String(root.notes ?? ""),
    arrival: root.arrival ? String(root.arrival) : undefined,
    departure: root.departure ? String(root.departure) : undefined,
  };
};

export const createApiAdapter = (
  baseUrl: string,
  getToken: () => Promise<string | null>,
): DemoAdapter => {
  const local = createLocalAdapter({
    getItem: (k) => window.localStorage.getItem(k),
    setItem: (k, v) => window.localStorage.setItem(k, v),
    removeItem: (k) => window.localStorage.removeItem(k),
  });

  const request = async (path: string, init?: RequestInit) => {
    const token = await getToken();
    if (!token) throw new Error("not authenticated");
    const res = await fetch(`${baseUrl}${path}`, {
      ...init,
      headers: {
        authorization: `Bearer ${token}`,
        "content-type": "application/json",
        ...(init?.headers ?? {}),
      },
    });
    if (!res.ok) throw new Error(`api ${res.status}: ${path}`);
    return res.json() as Promise<{ data: unknown }>;
  };

  return {
    async load(): Promise<DemoState> {
      const localState = await local.load();
      const { data } = await request("/api/v3/tasks");
      const tasks = (Array.isArray(data) ? data : []).map((d) =>
        toTask(d as TaskDoc),
      );
      return { ...localState, tasks };
    },
    save(state) {
      local.save(state);
    },
    reset() {
      const base = local.reset();
      return { ...createFixtures(), ...base };
    },
  };
};

export const isLive = () => Boolean(apiBase());
