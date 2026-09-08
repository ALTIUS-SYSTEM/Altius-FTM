import { z } from "zod";

export const demoTaskStatusSchema = z.enum(["unassigned", "assigned", "in-progress", "completed", "failed"]);
export const demoTaskSchema = z.object({
  id: z.string(), title: z.string().trim().min(2).max(120), address: z.string().trim().min(3).max(240),
  hub: z.string(), flow: z.string(), assignee: z.string(), status: demoTaskStatusSchema,
  date: z.string(), time: z.string(), priority: z.enum(["Normal", "High"]),
  notes: z.string().max(2000), arrival: z.string().optional(), departure: z.string().optional(),
  // Stop geometry, when the API supplies it. Optional because locally created
  // tasks have only an address until it is geocoded.
  lat: z.number().finite().min(-90).max(90).optional(),
  lng: z.number().finite().min(-180).max(180).optional()
});
export type DemoTask = z.infer<typeof demoTaskSchema>;
export type DemoTaskStatus = z.infer<typeof demoTaskStatusSchema>;
export const recordSchema = z.object({
  id: z.string(), kind: z.string(), name: z.string().trim().min(2).max(120),
  detail: z.string().max(500), hub: z.string(), status: z.string(), extra: z.string().max(500), archived: z.boolean().default(false)
});
export type DemoRecord = z.infer<typeof recordSchema>;
export const routeConfigSchema = z.object({ speed: z.number().min(5).max(100), service: z.number().min(0).max(120), capacity: z.number().min(1).max(5000), returnHub: z.boolean(), avoidTolls: z.boolean(), vehicles: z.array(z.string()) });
export const stateSchema = z.object({
  version: z.literal(1), tasks: z.array(demoTaskSchema).max(2000), records: z.array(recordSchema).max(2000),
  hub: z.string(), role: z.enum(["Admin", "Supervisor", "Lead"]), session: z.boolean(), locale: z.enum(["en", "id", "th", "ja", "zh", "fil-PH", "vi"]),
  routeConfig: routeConfigSchema,
  selectedVisits: z.array(z.string()), routeGenerated: z.boolean(),
  reviews: z.record(z.object({ status: z.string(), note: z.string() })),
  workflow: z.array(z.object({ id: z.string(), label: z.string().min(1), type: z.string(), required: z.boolean() })),
  permissions: z.record(z.boolean()), organization: z.object({ name: z.string(), currency: z.string(), passwordDays: z.string() })
});
export type DemoState = z.infer<typeof stateSchema>;
export const STATUS_LABELS: Record<DemoTaskStatus, string> = { unassigned: "Unassigned", assigned: "Assigned", "in-progress": "In progress", completed: "Completed", failed: "Failed" };
export const FLOWS = ["Delivery", "Pickup", "House inspection", "Field sales", "Home cleaning", "Field canvassing"];
/**
 * Fallback roster for screens not yet backed by the API. Live screens read the
 * real roster from `/api/v3/users` — assigning a task to a name that is not an
 * org member is rejected by the API with "assign task".
 */
export const DRIVERS: string[] = [];

/** Today, as `YYYY-MM-DD`. A function, not a constant: a workspace left open
 *  overnight would otherwise keep defaulting new work to the day it loaded. */
export const today = () => {
  const now = new Date();
  const local = new Date(now.getTime() - now.getTimezoneOffset() * 60_000);
  return local.toISOString().slice(0, 10);
};

export const createEmptyState = (overrides?: Partial<DemoState>): DemoState => ({
  version: 1,
  tasks: [],
  records: [],
  hub: "",
  role: "Admin",
  session: false,
  locale: "en",
  routeConfig: { speed: 40, service: 20, capacity: 500, returnHub: true, avoidTolls: false, vehicles: [] },
  selectedVisits: [],
  routeGenerated: false,
  reviews: {},
  workflow: [],
  permissions: {},
  organization: { name: "", currency: "IDR", passwordDays: "90" },
  ...overrides,
});
