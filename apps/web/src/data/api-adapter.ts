"use client";

import { z } from "zod";
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

/**
 * Tasks the server last gave us, so `save` can tell a real edit from a no-op.
 *
 * Seeded with the serialization of an empty list, not `""`: when `load()` fails
 * — no session yet, API down — this is never assigned, and an empty baseline is
 * what the state actually holds. An `""` sentinel made the very first `update()`
 * (the demo sign-in, which touches only `role` and `session`) compare `"[]"`
 * against `""` and report a task edit that never happened.
 */
let lastLoaded = JSON.stringify([]);

/** Stop ids from the last successful load, keyed by task id — needed so an
 *  edit can PUT the same stop row the server already has. */
let lastStopIds = new Map<string, string>();

const mapStatus = (stage: string): DemoTaskStatus => {
  switch (stage) {
    case "completed":
      return "completed";
    case "in_progress":
    case "in-progress":
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

const toApiStatus = (status: DemoTaskStatus): string => {
  switch (status) {
    case "completed":
      return "completed";
    case "in-progress":
      return "in_progress";
    case "unassigned":
      return "unassigned";
    case "failed":
      return "cancelled";
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

/** Backend numbers arrive as strings often enough to be worth normalising. */
const finiteOrUndefined = (v: unknown): number | undefined => {
  const n = typeof v === "number" ? v : typeof v === "string" ? Number(v) : NaN;
  return Number.isFinite(n) ? n : undefined;
};

const pick = (root: Record<string, unknown>, ...keys: string[]): unknown => {
  for (const k of keys) {
    if (root[k] !== undefined && root[k] !== null && root[k] !== "") return root[k];
  }
  return undefined;
};

const toTask = (doc: TaskDoc): DemoTask & { _stopId?: string } => {
  const root = doc.task ? flatten(doc.task) : flatten(doc);
  const stops = normalizeStops(doc);
  const firstStop = stops[0];
  const lat =
    finiteOrUndefined(firstStop?.lat) ??
    finiteOrUndefined(firstStop?.latitude) ??
    (firstStop?.location && typeof firstStop.location === "object"
      ? finiteOrUndefined((firstStop.location as { lat?: unknown; latitude?: unknown }).lat) ??
        finiteOrUndefined((firstStop.location as { lat?: unknown; latitude?: unknown }).latitude)
      : undefined);
  const lng =
    finiteOrUndefined(firstStop?.lng) ??
    finiteOrUndefined(firstStop?.longitude) ??
    (firstStop?.location && typeof firstStop.location === "object"
      ? finiteOrUndefined((firstStop.location as { lng?: unknown; longitude?: unknown }).lng) ??
        finiteOrUndefined((firstStop.location as { lng?: unknown; longitude?: unknown }).longitude)
      : undefined);
  const stopId = firstStop ? String(pick(firstStop, "id", "stop-id", "stop_id") ?? "") : "";
  return {
    id: String(pick(root, "id", "task-id", "task_id") ?? ""),
    title: String(root.title ?? ""),
    address: String(firstStop?.address ?? firstStop?.name ?? ""),
    hub: String(pick(root, "hub_id", "hub-id", "hub") ?? ""),
    flow: String(root.flow ?? "") || "Delivery",
    assignee: String(pick(root, "assignee", "assignee_id", "driver_id") ?? ""),
    status: mapStatus(String(pick(root, "stage", "status") ?? "assigned")),
    date: String(root.day ?? ""),
    // Stored as start_time / lower-case priority. The older `time` and
    // capitalised forms are still accepted so a row written before these
    // columns existed does not lose its shape on read.
    time: String(pick(root, "start_time", "start-time", "time") ?? ""),
    priority: String(pick(root, "priority") ?? "").toLowerCase() === "high" ? "High" : "Normal",
    notes: String(root.notes ?? ""),
    arrival: root.arrival ? String(root.arrival) : undefined,
    departure: root.departure ? String(root.departure) : undefined,
    lat,
    lng,
    ...(stopId ? { _stopId: stopId } : {}),
  };
};

/** Wire shape expected by the Rust `Task` deserializer (snake_case). */
const toApiTask = (
  task: DemoTask,
  scope: { tenantId: string; hubId: string },
  stopId?: string,
) => {
  const resolvedStop = stopId || `${task.id}-stop-1`;
  const status = toApiStatus(task.status);
  // Staff may type a Keycloak subject into assignee; demo display names are not
  // valid driver_sub values and would fail the FK — omit them.
  const assigneeLooksLikeSub = /^[A-Za-z0-9][A-Za-z0-9_.:-]{2,127}$/.test(task.assignee) && !/\s/.test(task.assignee);
  return {
    id: task.id,
    tenant_id: scope.tenantId,
    hub_id: task.hub || scope.hubId,
    title: task.title,
    // API has no unassigned stage; promote on write regardless of assignee shape.
    status: status === "unassigned" ? "assigned" : status,
    assignee_id: assigneeLooksLikeSub ? task.assignee : null,
    stops: [
      {
        id: resolvedStop,
        sequence: 0,
        name: task.title,
        address: task.address,
        location: { lat: task.lat ?? 0, lng: task.lng ?? 0 },
        status: status === "completed" ? "completed" : status === "in_progress" ? "working" : "pending",
        service_seconds: 0,
      },
    ],
    created_at: new Date().toISOString(),
    // Dispatch detail. These used to be collected by the editor and dropped on
    // the floor: the API had no columns for them, so instructions typed for a
    // driver never left the browser. Empty stays undefined rather than "" so
    // "not set" and "cleared" do not collapse into the same stored value.
    day: task.date || undefined,
    start_time: task.time || undefined,
    flow: task.flow || undefined,
    priority: task.priority === "High" ? "high" : "normal",
    notes: task.notes || undefined,
  };
};

const apiEnvelopeSchema = z.object({ data: z.unknown() }).passthrough();

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
    const json: unknown = await res.json();
    const parsed = apiEnvelopeSchema.safeParse(json);
    if (!parsed.success) throw new Error(`api response shape invalid: ${path}`);
    return parsed.data;
  };

  const resolveScope = async (): Promise<{ tenantId: string; hubId: string }> => {
    const { data } = await request("/api/v3/auth/me");
    const profile = (data && typeof data === "object" ? data : {}) as Record<string, unknown>;
    const tenantId = String(profile.organization ?? "");
    const hubId = String(profile.hub ?? "");
    if (!tenantId || !hubId) throw new Error("auth/me did not return organization and hub");
    return { tenantId, hubId };
  };

  const applyTaskSnapshot = (tasks: DemoTask[], stopIds: Map<string, string>) => {
    lastLoaded = JSON.stringify(tasks);
    lastStopIds = stopIds;
  };

  return {
    async load(): Promise<DemoState> {
      const { data } = await request("/api/v3/tasks");
      // The API is another trust boundary: its rows carry text written by
      // other users of the tenant. `toTask` only String()-coerces, so without
      // this the model's length and shape bounds hold for locally created
      // tasks and are silently waived for live ones.
      const stopIds = new Map<string, string>();
      const tasks = (Array.isArray(data) ? data : []).flatMap((d) => {
        const mapped = toTask(d as TaskDoc);
        const { _stopId, ...rest } = mapped;
        const parsed = demoTaskSchema.safeParse(rest);
        if (!parsed.success) {
          console.warn("dropping malformed task from API", parsed.error.issues);
          return [];
        }
        if (_stopId) stopIds.set(parsed.data.id, _stopId);
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
      applyTaskSnapshot(tasks, stopIds);
      return { ...base, ...localState, tasks };
    },
    async save(state) {
      const prefs = { ...createEmptyState(), ...state, tasks: [] };
      local.save(prefs);
      const prev: DemoTask[] = JSON.parse(lastLoaded) as DemoTask[];
      if (JSON.stringify(state.tasks) === lastLoaded) return;

      const prevById = new Map(prev.map((t) => [t.id, t]));
      const nextById = new Map(state.tasks.map((t) => [t.id, t]));
      const scope = await resolveScope();
      const stopIds = new Map(lastStopIds);

      for (const task of state.tasks) {
        const before = prevById.get(task.id);
        const body = toApiTask(task, scope, stopIds.get(task.id));
        if (!before) {
          await request("/api/v3/task-create", { method: "POST", body: JSON.stringify(body) });
          stopIds.set(task.id, body.stops[0].id);
          continue;
        }
        if (JSON.stringify(before) === JSON.stringify(task)) continue;
        await request(`/api/v3/task/${encodeURIComponent(task.id)}`, {
          method: "PUT",
          body: JSON.stringify(body),
        });
        stopIds.set(task.id, body.stops[0].id);
      }

      for (const id of prevById.keys()) {
        if (!nextById.has(id)) {
          // Soft-delete is not exposed on the API; refuse rather than claim success.
          throw new Error(`Task ${id} was removed locally but the API has no delete path. Reload to restore.`);
        }
      }

      applyTaskSnapshot(state.tasks, stopIds);
    },
    reset() {
      return createEmptyState();
    },
  };
};

export const isLive = () => Boolean(apiBase());
