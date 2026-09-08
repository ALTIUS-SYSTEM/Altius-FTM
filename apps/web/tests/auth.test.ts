import { describe, it, expect, beforeEach, vi } from "vitest";
import {
  authConfig,
  clearLoginState,
  finishLogin,
  hasSession,
  logout,
  peekAccessToken,
} from "@/lib/auth";

describe("authConfig", () => {
  it("returns null when Keycloak env is incomplete", () => {
    // Vitest does not inject NEXT_PUBLIC_*; authConfig reads process.env at call time.
    const prev = {
      url: process.env.NEXT_PUBLIC_KEYCLOAK_URL,
      realm: process.env.NEXT_PUBLIC_KEYCLOAK_REALM,
      clientId: process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID,
    };
    delete process.env.NEXT_PUBLIC_KEYCLOAK_URL;
    delete process.env.NEXT_PUBLIC_KEYCLOAK_REALM;
    delete process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID;
    expect(authConfig()).toBeNull();
    process.env.NEXT_PUBLIC_KEYCLOAK_URL = prev.url;
    process.env.NEXT_PUBLIC_KEYCLOAK_REALM = prev.realm;
    process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID = prev.clientId;
  });

  it("returns config when all Keycloak env vars are set", () => {
    process.env.NEXT_PUBLIC_KEYCLOAK_URL = "http://127.0.0.1:8081";
    process.env.NEXT_PUBLIC_KEYCLOAK_REALM = "altius";
    process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID = "altius-web";
    expect(authConfig()).toEqual({
      url: "http://127.0.0.1:8081",
      realm: "altius",
      clientId: "altius-web",
    });
  });
});

describe("finishLogin state + PKCE", () => {
  beforeEach(() => {
    logout();
    clearLoginState();
    sessionStorage.clear();
    vi.restoreAllMocks();
  });

  it("rejects a missing login session", async () => {
    process.env.NEXT_PUBLIC_KEYCLOAK_URL = "http://127.0.0.1:8081";
    process.env.NEXT_PUBLIC_KEYCLOAK_REALM = "altius";
    process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID = "altius-web";
    const cfg = authConfig()!;
    await expect(finishLogin(cfg, "code", "http://localhost:3000/callback", "state")).rejects.toThrow(
      /Login session missing/,
    );
    expect(hasSession()).toBe(false);
  });

  it("rejects a state mismatch and clears one-shot PKCE values", async () => {
    process.env.NEXT_PUBLIC_KEYCLOAK_URL = "http://127.0.0.1:8081";
    process.env.NEXT_PUBLIC_KEYCLOAK_REALM = "altius";
    process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID = "altius-web";
    sessionStorage.setItem("altius.pkce.verifier", "verifier");
    sessionStorage.setItem("altius.pkce.state", "expected");
    const cfg = authConfig()!;
    await expect(finishLogin(cfg, "code", "http://localhost:3000/callback", "other")).rejects.toThrow(
      /state mismatch/,
    );
    expect(sessionStorage.getItem("altius.pkce.verifier")).toBeNull();
    expect(sessionStorage.getItem("altius.pkce.state")).toBeNull();
    expect(hasSession()).toBe(false);
  });

  it("stores tokens in memory and returns the access token", async () => {
    process.env.NEXT_PUBLIC_KEYCLOAK_URL = "http://127.0.0.1:8081";
    process.env.NEXT_PUBLIC_KEYCLOAK_REALM = "altius";
    process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID = "altius-web";
    sessionStorage.setItem("altius.pkce.verifier", "verifier");
    sessionStorage.setItem("altius.pkce.state", "ok");
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        json: async () => ({
          access_token: "access-abc",
          refresh_token: "refresh-xyz",
          expires_in: 300,
        }),
      })),
    );
    const cfg = authConfig()!;
    const token = await finishLogin(cfg, "code", "http://localhost:3000/callback", "ok");
    expect(token).toBe("access-abc");
    expect(hasSession()).toBe(true);
    expect(peekAccessToken()).toBe("access-abc");
    expect(sessionStorage.getItem("altius.token.access")).toBeNull();
    expect(sessionStorage.getItem("altius.pkce.verifier")).toBeNull();
  });
});
