import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { createApiAdapter } from "@/data/api-adapter";
import { STORAGE_KEY } from "@/data/adapter";
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

  /**
   * The board filters every list on the selected hub, and tasks carry a hub
   * *id*. With no hubs loaded the selector had no options and the selection
   * stayed "", so tasks fetched from the API were all filtered out of view —
   * an account with records in the database looked completely empty.
   */
  describe("hub selection", () => {
    const stubApi = (opts: { hubs: unknown[]; ownHub?: string; taskHub?: string }) => {
      globalThis.fetch = vi.fn(async (input: RequestInfo | URL) => {
        const url = String(input);
        const json = (body: unknown) =>
          new Response(JSON.stringify(body), { status: 200, headers: { "content-type": "application/json" } });
        if (url.endsWith("/api/v3/tasks"))
          return json({
            data: [
              {
                task: { id: "T-1", title: "Depot", hub_id: opts.taskHub ?? "jakarta", stage: "assigned", day: "2026-09-07" },
                stops: [{ id: "S-1", address: "Jl. Sudirman" }],
              },
            ],
          });
        if (url.endsWith("/api/v3/hubs")) return json({ data: opts.hubs });
        if (url.endsWith("/api/v3/auth/me"))
          return json({ data: { organization: "altius", hub: opts.ownHub ?? "jakarta" } });
        throw new Error(`unexpected ${url}`);
      }) as typeof fetch;
      return createApiAdapter("https://api.test", async () => "token");
    };

    // Postgres serialises the row as-is; TypeDB uses its attribute names.
    const pgHub = (id: string, name: string) => ({ hub: { id, name, lat: null, lng: null } });
    const tdbHub = (id: string, name: string) => ({ hub: { "hub-id": id, "display-name": name } });

    it("loads hubs as records and selects this account's hub", async () => {
      const state = await stubApi({ hubs: [pgHub("jakarta", "Jakarta"), pgHub("bandung", "Bandung")], ownHub: "bandung" }).load();
      expect(state.records.filter(r => r.kind === "hub").map(r => ({ id: r.id, name: r.name }))).toEqual([
        { id: "jakarta", name: "Jakarta" },
        { id: "bandung", name: "Bandung" },
      ]);
      expect(state.hub).toBe("bandung");
    });

    it("reads the TypeDB attribute names too", async () => {
      const state = await stubApi({ hubs: [tdbHub("jakarta", "Jakarta")] }).load();
      expect(state.records.filter(r => r.kind === "hub")[0]).toMatchObject({ id: "jakarta", name: "Jakarta" });
    });

    it("selects a hub the loaded tasks can actually match", async () => {
      const state = await stubApi({ hubs: [pgHub("jakarta", "Jakarta")] }).load();
      expect(state.tasks).toHaveLength(1);
      expect(state.tasks.filter(t => t.hub === state.hub)).toHaveLength(1);
    });

    it("replaces a stored hub that no longer exists", async () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ ...(await stubApi({ hubs: [pgHub("jakarta", "Jakarta")] }).load()), hub: "surabaya", tasks: [] }),
      );
      const state = await stubApi({ hubs: [pgHub("jakarta", "Jakarta")], ownHub: "" }).load();
      expect(state.hub).toBe("jakarta");
    });
  });
});
