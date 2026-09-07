"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { useDemo } from "@/components/demo-provider";
import { authConfig, decodeJwt, finishLogin, type JwtClaims } from "@/lib/auth";
import type { DemoState } from "@/data/model";

const ROLE_MAP: Record<string, DemoState["role"]> = {
  admin: "Admin",
  supervisor: "Supervisor",
  lead: "Lead",
  driver: "Lead",
};

const KNOWN_ROLES = ["admin", "supervisor", "lead", "driver"] as const;

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

  useEffect(() => {
    if (!ready) return;
    const cfg = authConfig();
    if (!cfg) {
      setError("Keycloak is not configured in this build.");
      return;
    }
    const params = new URLSearchParams(window.location.search);
    const code = params.get("code");
    if (!code) {
      setError("Authorization code is missing from the callback URL.");
      return;
    }
    const redirectUri = `${window.location.origin}/callback`;
    finishLogin(cfg, code, redirectUri)
      .then(() => {
        const token = sessionStorage.getItem("altius.token.access");
        const claims = token ? decodeJwt(token) : {};
        const role = extractRole(claims);
        update((s) => ({ ...s, session: true, role }));
        router.replace("/dashboard/task");
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
