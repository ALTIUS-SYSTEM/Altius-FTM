import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createApiAdapter } from "@/data/api-adapter";
import type { DemoTask } from "@/data/model";

const sampleTask = (overrides: Partial<DemoTask> = {}): DemoTask => ({
  id: "ALT-001",
  title: "Sample task",
  address: "Jl. Sudirman",
  hub: "jakarta",
  flow: "Delivery",
  assignee: "",
  status: "assigned",
  date: "2026-09-07",
  time: "08:00",
  priority: "Normal",
  notes: "",
  ...overrides,
});

describe("api adapter", () => {
  const originalFetch = globalThis.fetch;
  let calls: Array<{ url: string; init?: RequestInit }>;

  beforeEach(() => {
    calls = [];
    vi.stubGlobal(
      "localStorage",
      (() => {
        const map = new Map<string, string>();
        return {
          getItem: (k: string) => map.get(k) ?? null,
          setItem: (k: string, v: string) => {
            map.set(k, v);
          },
          removeItem: (k: string) => {
            map.delete(k);
          },
        };
      })(),
    );
  });

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("maps Postgres task envelopes into demo tasks with lat/lng and hub_id", async () => {
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      calls.push({ url });
      if (url.endsWith("/api/v3/tasks")) {
        return new Response(
          JSON.stringify({
            data: [
              {
                task: {
                  id: "T-1",
                  title: "Depot",
                  hub_id: "jakarta",
                  stage: "in_progress",
                  day: "2026-09-07",
                  assignee: "driver-sub-1",
                },
                stops: [{ id: "S-1", address: "Jl. Sudirman", lat: -6.2, lng: 106.8, stage: "arrived" }],
              },
            ],
            meta: { mode: "live", organization: "org-1" },
          }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      throw new Error(`unexpected ${url}`);
    }) as typeof fetch;

    const adapter = createApiAdapter("https://api.test", async () => "token");
    const state = await adapter.load();
    expect(state.tasks).toHaveLength(1);
    expect(state.tasks[0]).toMatchObject({
      id: "T-1",
      hub: "jakarta",
      address: "Jl. Sudirman",
      status: "in-progress",
      assignee: "driver-sub-1",
      lat: -6.2,
      lng: 106.8,
    });
  });

  it("persists new tasks via task-create and edits via PUT", async () => {
    globalThis.fetch = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const url = String(input);
      calls.push({ url, init });
      if (url.endsWith("/api/v3/tasks") && (!init || !init.method || init.method === "GET")) {
        return new Response(JSON.stringify({ data: [], meta: { mode: "live" } }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url.endsWith("/api/v3/auth/me")) {
        return new Response(
          JSON.stringify({ data: { organization: "org-1", hub: "jakarta", subject: "admin-1" } }),
          { status: 200, headers: { "content-type": "application/json" } },
        );
      }
      if (url.endsWith("/api/v3/task-create")) {
        return new Response(JSON.stringify({ data: { taskId: "ALT-001" } }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      if (url.includes("/api/v3/task/ALT-001") && init?.method === "PUT") {
        return new Response(JSON.stringify({ data: { taskId: "ALT-001" } }), {
          status: 200,
          headers: { "content-type": "application/json" },
        });
      }
      throw new Error(`unexpected ${url} ${init?.method}`);
    }) as typeof fetch;

    const adapter = createApiAdapter("https://api.test", async () => "token");
    const empty = await adapter.load();
    const created = sampleTask();
    await adapter.save({ ...empty, tasks: [created] });
    expect(calls.some((c) => c.url.endsWith("/api/v3/task-create"))).toBe(true);

    await adapter.save({
      ...empty,
      tasks: [sampleTask({ title: "Updated title" })],
    });
    // After create, lastLoaded holds the created task; a title edit must PUT.
    expect(calls.some((c) => c.url.includes("/api/v3/task/ALT-001") && c.init?.method === "PUT")).toBe(true);
  });
});
