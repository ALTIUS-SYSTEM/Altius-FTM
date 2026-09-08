"use client";

import { useState } from "react";
import { Card, Field } from "@/components/ui";
import { apiBase } from "@/data/api-adapter";
import { accessToken, authConfig } from "@/lib/auth";

const ROLES = ["super-admin", "admin", "supervisor", "lead", "driver"] as const;

interface Created {
  subject: string;
  organization: string;
  hub: string;
  temporaryPassword: string;
}

/**
 * Admin-only account provisioning. Creates the Keycloak user and its
 * organization membership in one call; the organization and hub come from the
 * caller's own token server-side, never from this form.
 */
export function ProvisionUser({ onClose }: { onClose: () => void }) {
  const [username, setUsername] = useState("");
  const [email, setEmail] = useState("");
  const [displayName, setDisplayName] = useState("");
  const [role, setRole] = useState<(typeof ROLES)[number]>("driver");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [created, setCreated] = useState<Created | null>(null);

  const submit = async () => {
    setBusy(true);
    setError("");
    try {
      const base = apiBase();
      const cfg = authConfig();
      if (!base || !cfg) throw new Error("The Altius API and Keycloak must be configured.");
      const token = await accessToken(cfg);
      if (!token) throw new Error("Your session expired. Sign in again.");
      const res = await fetch(`${base}/api/v3/users`, {
        method: "POST",
        headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
        body: JSON.stringify({
          username: username.trim(),
          email: email.trim(),
          display_name: displayName.trim(),
          realm_roles: [role],
        }),
      });
      const body = (await res.json().catch(() => ({}))) as { data?: Created; error?: { message?: string } };
      if (!res.ok) throw new Error(body.error?.message ?? `Could not create the user (${res.status}).`);
      if (!body.data) throw new Error("The server did not return the new account.");
      setCreated(body.data);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not create the user.");
    } finally {
      setBusy(false);
    }
  };

  if (created) {
    return (
      <Card title="Account created">
        <p>
          <strong>{displayName || username}</strong> can now sign in to {created.organization} · {created.hub}.
        </p>
        <div className="info-box">
          <p>
            One-time password — <strong>shown once</strong>. Hand it over in person or through a channel
            you trust, not by email alongside the username. Keycloak will require a change at first login.
          </p>
          <p><code>{created.temporaryPassword}</code></p>
        </div>
        <button className="primary" onClick={onClose}>Done</button>
      </Card>
    );
  }

  return (
    <Card title="Create user">
      {error && <div className="error-banner" role="alert">{error}</div>}
      <Field label="Username"><input value={username} onChange={e => setUsername(e.target.value)} autoComplete="off"/></Field>
      <Field label="Email"><input type="email" value={email} onChange={e => setEmail(e.target.value)} autoComplete="off"/></Field>
      <Field label="Full name"><input value={displayName} onChange={e => setDisplayName(e.target.value)} maxLength={200}/></Field>
      <Field label="Role">
        <select value={role} onChange={e => setRole(e.target.value as typeof role)}>
          {ROLES.map(r => <option key={r} value={r}>{r}</option>)}
        </select>
      </Field>
      <div className="row-actions">
        <button className="primary" disabled={busy || !username.trim() || !email.trim()} onClick={submit}>
          {busy ? "Creating…" : "Create user"}
        </button>
        <button onClick={onClose} disabled={busy}>Cancel</button>
      </div>
    </Card>
  );
}
