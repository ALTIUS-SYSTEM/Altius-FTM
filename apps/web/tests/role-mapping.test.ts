import { describe, it, expect } from "vitest";
import { extractRole } from "@/features/callback";

const claims = (...roles: string[]) => ({ realm_access: { roles } });

describe("extractRole", () => {
  it("maps every realm role the API can issue to a shell persona", () => {
    // Regression: `super-admin` was listed as a known role but missing from the
    // map, so it resolved to undefined and every subsequent save failed schema
    // validation with a raw ZodError on screen. TypeScript did not catch it —
    // indexing a Record<string, T> yields T, not T | undefined.
    expect(extractRole(claims("super-admin"))).toBe("Admin");
    expect(extractRole(claims("admin"))).toBe("Admin");
    expect(extractRole(claims("supervisor"))).toBe("Supervisor");
    expect(extractRole(claims("lead"))).toBe("Lead");
    expect(extractRole(claims("driver"))).toBe("Lead");
  });

  it("never returns undefined, whatever the token carries", () => {
    for (const c of [claims(), claims("nonsense"), {} as never]) {
      expect(extractRole(c)).toBeTruthy();
    }
  });

  it("prefers the most privileged role when a token carries several", () => {
    expect(extractRole(claims("driver", "admin", "super-admin"))).toBe("Admin");
    expect(extractRole(claims("lead", "supervisor"))).toBe("Supervisor");
  });

  it("is case-insensitive, as Keycloak role casing is not guaranteed", () => {
    expect(extractRole(claims("SUPER-ADMIN"))).toBe("Admin");
    expect(extractRole(claims("Supervisor"))).toBe("Supervisor");
  });

  it("reads roles from resource_access as well as realm_access", () => {
    expect(extractRole({ resource_access: { "altius-web": { roles: ["supervisor"] } } })).toBe("Supervisor");
  });
});
