import { describe, expect, it, vi, beforeEach } from "vitest";
import { render, screen, act, cleanup } from "@testing-library/react";
import type { DemoState } from "@/data/model";
import { createEmptyState } from "@/data/model";

/**
 * Tokens live in memory, so every reload of the tab starts signed out. The
 * provider's first load therefore runs before the session has been restored
 * and comes back empty-handed; the silent re-auth that follows lands via a
 * client-side redirect, which does not remount the provider.
 *
 * Without a way to ask the API again, the tab kept that empty state for as
 * long as it stayed open: signed in, the user's own name in the header, and
 * every record already in the database invisible.
 */

const loads: (() => Promise<DemoState>)[] = [];
const adapter = {
  load: () => (loads.shift() ?? (() => Promise.resolve(createEmptyState())))(),
  save: vi.fn(),
  reset: () => createEmptyState(),
};

vi.mock("@/data/api-adapter", () => ({
  apiBase: () => "https://api.example",
  createApiAdapter: () => adapter,
}));
vi.mock("@/lib/auth", () => ({
  authConfig: () => ({ url: "https://sso.example", realm: "altius", clientId: "web" }),
  accessToken: () => Promise.resolve("token"),
}));

const { DemoProvider, useDemo } = await import("@/components/demo-provider");

let reload: () => Promise<void>;
let update: (change: (s: DemoState) => DemoState) => void;
let role: string;
function Probe() {
  const ctx = useDemo();
  reload = ctx.reload;
  update = ctx.update;
  role = ctx.state.role;
  return <div data-testid="probe">{ctx.ready ? `${ctx.state.tasks.length} tasks` : "loading"}</div>;
}

const stateWithTasks = (n: number): DemoState => ({
  ...createEmptyState(),
  tasks: Array.from({ length: n }, (_, i) => ({ ...createEmptyState().tasks[0], id: `T-${i}` })) as DemoState["tasks"],
});

describe("workspace loading after a session is restored", () => {
  beforeEach(() => { cleanup(); loads.length = 0; });

  it("recovers the workspace when the first load ran before the token existed", async () => {
    // Mount fails the way a reload does: no token yet.
    loads.push(() => Promise.reject(new Error("not authenticated")));
    // The silent re-auth completes and the API now answers.
    loads.push(() => Promise.resolve(stateWithTasks(3)));

    await act(async () => { render(<DemoProvider><Probe/></DemoProvider>); });
    expect(screen.getByTestId("probe").textContent).toBe("0 tasks");

    await act(async () => { await reload(); });
    expect(screen.getByTestId("probe").textContent).toBe("3 tasks");
  });

  it("keeps the sign-in that just completed rather than the stored one", async () => {
    loads.push(() => Promise.reject(new Error("not authenticated")));
    // What the API returns still carries the pre-sign-in prefs, because they
    // were stored before this token existed.
    loads.push(() => Promise.resolve({ ...stateWithTasks(1), session: false, role: "Lead" as const }));

    await act(async () => { render(<DemoProvider><Probe/></DemoProvider>); });
    // The callback records the identity it just proved, then asks for data.
    await act(async () => { update(s => ({ ...s, session: true, role: "Admin" })); });
    await act(async () => { await reload(); });

    expect(screen.getByTestId("probe").textContent).toBe("1 tasks");
    expect(role).toBe("Admin");
  });
});
