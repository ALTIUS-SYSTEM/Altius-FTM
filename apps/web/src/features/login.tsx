"use client";

import { useState } from "react";
import { useDemo } from "@/components/demo-provider";
import { authConfig, startLogin } from "@/lib/auth";
import { Brand, Field, Icon } from "@/components/ui";
import type { DemoState } from "@/data/model";

export function DemoLogin() {
  const { update, t, error, reset } = useDemo();
  const [role, setRole] = useState<DemoState["role"]>("Admin");
  const cfg = authConfig();

  const onDemoSubmit = (event: React.FormEvent) => {
    event.preventDefault();
    update((current) => ({ ...current, role, session: true }), "Demo session started. No authentication was performed.");
  };

  const onKeycloak = () => {
    if (!cfg) return;
    const redirectUri = `${window.location.origin}/callback`;
    startLogin(cfg, redirectUri).catch(() => {});
  };

  return (
    <main className="login-page">
      <section className="login-story">
        <Brand />
        <div>
          <span className="eyebrow">FIELD OPERATIONS, IN SYNC</span>
          <h1>
            Every team.<br />Every task.<br /><em>Moving forward.</em>
          </h1>
          <p>Bring clarity to the complexity of field operations. Plan with confidence. Deliver with Altius.</p>
          <div className="login-route" aria-hidden="true">
            <span>01<br /><small>Plan</small></span>
            <i />
            <span>02<br /><small>Dispatch</small></span>
            <i />
            <span>03<br /><small>Deliver</small></span>
          </div>
        </div>
        <small>© 2026 Altius · Field Task Management</small>
      </section>
      <section className="login-form-wrap">
        <form className="login-form" onSubmit={onDemoSubmit}>
          <span className="login-badge"><Icon name="tasks" /> INTERACTIVE DEMO</span>
          <h2>Your operations.<br />One clear view.</h2>
          <p>Explore a synthetic workspace. No account, password, or backend connection is required.</p>
          <Field label="Choose a demo persona">
            <select value={role} onChange={(event) => setRole(event.target.value as DemoState["role"])}>
              <option>Admin</option>
              <option>Supervisor</option>
              <option>Lead</option>
            </select>
          </Field>
          <Field label="Organization">
            <input value="Altius Logistics · Demo" readOnly />
          </Field>
          <button className="primary login-submit" type="submit">{t("signIn")}<Icon name="arrow" /></button>
          {cfg && (
            <>
              <div className="login-divider"><span>or</span></div>
              <button type="button" className="secondary login-submit" onClick={onKeycloak}>
                Sign in with Keycloak
              </button>
            </>
          )}
          <div className="info-box">Local demo session only, not authentication. Role selection does not grant or restrict access. Use synthetic data only.</div>
          {error && <div className="error-banner" role="alert">{error}<button type="button" onClick={reset}>Reset local demo</button></div>}
        </form>
      </section>
    </main>
  );
}
