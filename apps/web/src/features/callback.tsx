"use client";

import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
import { useDemo } from "@/components/demo-provider";
import { authConfig, clearLoginState, decodeJwt, finishLogin, silentAuth, type JwtClaims } from "@/lib/auth";
import type { DemoState } from "@/data/model";

const ROLE_MAP: Record<string, DemoState["role"]> = {
  admin: "Admin",
  supervisor: "Supervisor",
  lead: "Lead",
  // DemoState personas are staff-only (no Driver). Map driver→Lead so the
  // documented driver/changeme Keycloak login still enters the demo shell;
  // API authz remains realm-role based, not this UI label.
  driver: "Lead",
};

const KNOWN_ROLES = ["super-admin", "admin", "supervisor", "lead", "driver"] as const;

function extractRole(claims: JwtClaims): DemoState["role"] {
  const lowerRoles = new Set<string>();
  for (const r of claims.realm_access?.roles ?? []) lowerRoles.add(r.toLowerCase());
  for (const client of Object.values(claims.resource_access ?? {})) {
    for (const r of client.roles ?? []) lowerRoles.add(r.toLowerCase());
  }
  const matchedRole = KNOWN_ROLES.find((r) => lowerRoles.has(r));
  return matchedRole ? ROLE_MAP[matchedRole] : "Lead";
}

export function Callback() {
  const { update, ready } = useDemo();
  const router = useRouter();
  const [error, setError] = useState("");
  // The effect consumes the query string and then strips it, so it is not safe
  // to run twice — and StrictMode runs every effect twice in development. The
  // second pass would find an empty query and report a missing code over a
  // sign-in that had in fact already been handled.
  const handled = useRef(false);

  useEffect(() => {
    if (!ready || handled.current) return;
    handled.current = true;
    const cfg = authConfig();
    if (!cfg) {
      setError("Keycloak is not configured in this build.");
      return;
    }
    const params = new URLSearchParams(window.location.search);
    // Strip the query before doing anything else: on a failure the code is
    // still unredeemed, and leaving it in the address bar puts it in history,
    // session restore, and the Referer of any later subresource.
    window.history.replaceState({}, "", window.location.pathname);

    const oauthError = params.get("error");
    if (oauthError) {
      clearLoginState();
      // A silent attempt with no SSO session answers with one of these. That is
      // the expected negative case, not a failure worth showing anyone — send
      // them to the sign-in form instead of an error banner.
      if (["login_required", "interaction_required", "consent_required", "account_selection_required"].includes(oauthError)) {
        silentAuth.markTried();
        router.replace("/login");
        return;
      }
      setError(params.get("error_description") ?? `Sign-in failed: ${oauthError}`);
      return;
    }
    const code = params.get("code");
    if (!code) {
      clearLoginState();
      setError("Authorization code is missing from the callback URL.");
      return;
    }
    const redirectUri = `${window.location.origin}/callback`;
    finishLogin(cfg, code, redirectUri, params.get("state"))
      .then((token) => {
        const claims = token ? decodeJwt(token) : {};
        const role = extractRole(claims);
        update((s) => ({ ...s, session: true, role }));
        // A successful sign-in means the SSO session is good, so allow silent
        // restore again on the next reload.
        silentAuth.reset();
        router.replace(silentAuth.takeReturn() ?? "/dashboard/task");
      })
      .catch((e: Error) => setError(e.message));
  }, [ready, update, router]);

  if (error) {
    return (
      <main className="login-page">
        <section className="login-form-wrap">
          <div className="error-banner" role="alert">{error}</div>
        </section>
      </main>
    );
  }

  return (
    <main className="login-page">
      <section className="login-form-wrap">
        <p>Completing sign-in…</p>
      </section>
    </main>
  );
}
