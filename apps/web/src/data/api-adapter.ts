"use client";

import type { DemoAdapter } from "./adapter";
import { createLocalAdapter, STORAGE_KEY } from "./adapter";
import type { DemoState, DemoTask, DemoTaskStatus } from "./model";
import { createEmptyState, demoTaskSchema } from "./model";

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

/** Depth-bounded: a deeply nested single-element array in a response would
 *  otherwise blow the stack and blank the whole board for every user. */
const MAX_LEAF_DEPTH = 16;
const leaf = (v: unknown, depth = 0): unknown => {
  if (depth >= MAX_LEAF_DEPTH) return v;
  if (v && typeof v === "object" && "value" in v) {
    return (v as { value: unknown }).value;
  }
  if (Array.isArray(v) && v.length === 1) {
    return leaf(v[0], depth + 1);
  }
  return v;
};

const flatten = (obj: Record<string, unknown>): Record<string, unknown> =>
  Object.fromEntries(Object.entries(obj).map(([k, v]) => [k, leaf(v)]));

/** Tasks the server last gave us, so `save` can tell a real edit from a no-op. */
let lastLoaded = "";

const mapStatus = (stage: string): DemoTaskStatus => {
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

const toTask = (doc: TaskDoc): DemoTask => {
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
    priority: ["Normal", "High"].includes(String(root.priority)) ? (String(root.priority) as DemoTask["priority"]) : "Normal",
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
      const { data } = await request("/api/v3/tasks");
      // The API is another trust boundary: its rows carry text written by
      // other users of the tenant. `toTask` only String()-coerces, so without
      // this the model's length and shape bounds hold for locally created
      // tasks and are silently waived for live ones.
      const tasks = (Array.isArray(data) ? data : []).flatMap((d) => {
        const parsed = demoTaskSchema.safeParse(toTask(d as TaskDoc));
        if (!parsed.success) {
          console.warn("dropping malformed task from API", parsed.error.issues);
          return [];
        }
        return [parsed.data];
      });
      const base = createEmptyState();
      // Distinguish "nothing stored" from "stored data rejected": swallowing
      // both discarded every persisted preference, record and permission with
      // no notice, and the next save overwrote the original beyond recovery.
      const localState = await local.load().catch((err: unknown) => {
        try {
          const raw = window.localStorage.getItem(STORAGE_KEY);
          if (raw) window.localStorage.setItem(`${STORAGE_KEY}.corrupt`, raw);
          window.localStorage.removeItem(STORAGE_KEY);
        } catch {
          // Storage unavailable; nothing to preserve.
        }
        console.warn("stored workspace preferences were rejected and set aside", err);
        return base;
      });
      lastLoaded = JSON.stringify(tasks);
      return { ...base, ...localState, tasks };
    },
    save(state) {
      const prefs = { ...createEmptyState(), ...state, tasks: [] };
      local.save(prefs);
      // There is no write path to the API yet. Returning normally here made
      // every assignment, arrival and completion vanish on reload while the
      // UI reported success — a silent integrity failure in the audit trail.
      // Fail loudly instead, so the provider surfaces it.
      if (JSON.stringify(state.tasks) !== lastLoaded) {
        throw new Error(
          "Task changes are not saved: this build has no write path to the Altius API. Your edit was not persisted.",
        );
      }
    },
    reset() {
      return createEmptyState();
    },
  };
};

export const isLive = () => Boolean(apiBase());
