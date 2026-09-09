"use client";

import Link from "next/link";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { useDemo } from "./demo-provider";
import { Brand, Icon, Modal, Badge, Field } from "./ui";
import { MODULES, PAGE_DESCRIPTIONS } from "@/lib/routes";
import { LOCALES } from "@/lib/i18n";
import type { Locale } from "@/lib/i18n";
import type { DemoState } from "@/data/model";
import { authConfig, decodeJwt, endSession, peekAccessToken, type JwtClaims } from "@/lib/auth";

/**
 * Who is actually signed in, from the access token.
 *
 * Read in an effect rather than during render: tokens are memory-only, so a
 * server render has none and inlining this would produce a hydration mismatch.
 * Display only — the API re-validates every token against the realm JWKS.
 */
function useIdentity(): { name: string; email: string; initials: string } {
  const [claims, setClaims] = useState<JwtClaims>({});
  useEffect(() => {
    const token = peekAccessToken();
    if (token) setClaims(decodeJwt(token));
  }, []);
  const name = claims.name?.trim()
    || [claims.given_name, claims.family_name].filter(Boolean).join(" ").trim()
    || claims.preferred_username
    || "Signed in";
  const initials = name.split(/\s+/).filter(Boolean).map(word => word[0]).slice(0, 2).join("").toUpperCase();
  return { name, email: claims.email ?? "", initials: initials || "?" };
}

export function Shell({ path, children }: { path: string; children: ReactNode }) {
  const { state, update, t, error, reset } = useDemo();
  const identity = useIdentity();
  // Rendered client-side: the server and the reader can sit in different time
  // zones, and formatting during SSR would hydrate one date over another.
  const [today, setToday] = useState({ date: "", weekday: "", zone: "" });
  useEffect(() => {
    const now = new Date();
    setToday({
      date: now.toLocaleDateString(undefined, { day: "2-digit", month: "short", year: "numeric" }),
      weekday: now.toLocaleDateString(undefined, { weekday: "long" }),
      zone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    });
  }, []);
  // With Keycloak configured the workspace is backed by the API. Without it
  // nothing can load at all, so the demo notices only make sense in that case.
  const live = authConfig() !== null;
  const [collapsed, setCollapsed] = useState(false);
  const [navOpen, setNavOpen] = useState(false);
  const [panel, setPanel] = useState("");
  // Escape must dismiss the mobile drawer: it covers the page, so leaving the
  // keyboard with no exit would trap a user who opened it by accident.
  useEffect(() => {
    if (!navOpen) return;
    const onKey = (event: KeyboardEvent) => { if (event.key === "Escape") setNavOpen(false); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [navOpen]);
  const moduleKey = path === "no-access" ? "setting" : path.split("/")[0];
  const module = MODULES.find(item => item.key === moduleKey) ?? MODULES[0];
  const hubRecords = state.records.filter(record => record.kind === "hub" && !record.archived);
  const title = path === "dashboard/task" ? "Operations overview" : t(path.split("/").at(-1) ?? moduleKey);
  return <div className={`workspace ${collapsed ? "collapsed" : ""} ${navOpen ? "nav-open" : ""}`}>
    <a className="skip-link" href="#main">Skip to main content</a>
    <aside className="sidebar" id="sidebar-nav" aria-label="Main navigation"><Link href="/dashboard/task" className="brand-link"><Brand compact={collapsed}/></Link><div className="workspace-label">{collapsed ? "FTM" : "OPERATIONS WORKSPACE"}</div><button className="nav-item nav-close" onClick={() => setNavOpen(false)}><Icon name="close"/>Close menu</button><nav aria-label="Main navigation">{MODULES.map((item, index) => <Link key={item.key} href={`/${item.home}`} title={t(item.key)} aria-current={moduleKey === item.key ? "page" : undefined} onClick={() => setNavOpen(false)} className={`nav-item ${moduleKey === item.key ? "active" : ""} ${index === 5 ? "nav-divider" : ""}`}><Icon name={item.key}/>{!collapsed && <><span>{t(item.key)}</span></>}</Link>)}</nav><div className="sidebar-bottom">{!collapsed && <div className="workspace-health"><span className="health-dot"/><div>{live ? "Connected workspace" : "Local demo environment"}<small>{live ? "Live API · changes are saved" : "No live integrations"}</small></div></div>}<button className="nav-item collapse-button" aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"} aria-expanded={!collapsed} onClick={() => setCollapsed(!collapsed)}><Icon name="menu"/>{!collapsed && "Collapse sidebar"}</button></div></aside>
    <div className="main-shell" inert={navOpen || undefined}><header className="topbar"><button className="icon-button nav-toggle" aria-label="Open navigation" aria-expanded={navOpen} aria-controls="sidebar-nav" onClick={() => setNavOpen(true)}><Icon name="menu"/></button><div className="breadcrumb">Workspace <span>/</span> <strong>{t(moduleKey)}</strong></div><div className="topbar-actions"><label className="hub-select"><Icon name="pin" size={17}/><span className="sr-only">Current hub</span><select aria-label="Current hub" value={state.hub} onChange={event => { const id = event.target.value; const name = hubRecords.find(hub => hub.id === id)?.name ?? id; update(current => ({ ...current, hub: id, routeGenerated: false }), `Switched to ${name}`); }}>{hubRecords.map(hub => <option key={hub.id} value={hub.id}>{hub.name}</option>)}</select></label><select className="language-select" aria-label="Language" value={state.locale} onChange={event => update(current => ({ ...current, locale: event.target.value as Locale }))}>{Object.entries(LOCALES).map(([key, label]) => <option key={key} value={key}>{label}</option>)}</select><button className="icon-button notification-button" aria-label="Open notifications" onClick={() => setPanel("notifications")}><Icon name="bell"/><span/></button><button className="profile-button" onClick={() => setPanel("profile")} aria-label="Open profile"><span className="avatar">{live ? identity.initials : "AM"}</span><span>{live ? identity.name : "Alex Morgan"}<small>{state.role}{live ? "" : " · Demo"}</small></span></button></div></header>
    {live ? null : <div className="demo-banner"><Badge tone="demo">{t("demo")}</Badge><span>Synthetic data · Saved in this browser only · No backend authorization</span><button onClick={() => setPanel("help")}>How this demo works <Icon name="arrow" size={15}/></button></div>}
    {error && <div className="error-banner" role="alert">{error}<button onClick={reset}>Reset demo</button></div>}
    {state.locale !== "en" && <div className="locale-note">Navigation and core controls: {LOCALES[state.locale]}. Untranslated operational details explicitly fall back to English.</div>}
    <main id="main" tabIndex={-1}><div className="page-heading"><div><div className="eyebrow">{state.hub.toUpperCase()} HUB <span className="eyebrow-dot">/</span> FIELD TASK MANAGEMENT</div><h1>{title}</h1><p>{PAGE_DESCRIPTIONS[moduleKey]}</p></div><span className="date-chip">{today.date} <span>{today.weekday}</span></span></div>
    {module.tabs.length > 1 && <nav className="tabs" aria-label={`${t(moduleKey)} views`}>{module.tabs.map(tab => { const target = tab === "no-access" ? tab : `${module.key}/${tab}`; return <Link key={tab} href={`/${target}`} aria-current={path === target ? "page" : undefined} className={path === target ? "active" : ""}>{t(tab)}</Link>; })}</nav>}
    <div className="page-content">{children}</div><footer className="page-footer"><span>© 2026 Altius · Built for the way your team moves.</span><span>{today.zone}</span></footer></main></div>
    {navOpen && <button className="nav-backdrop" aria-label="Close navigation" onClick={() => setNavOpen(false)}/>}
    {panel && <Modal title={panel === "profile" ? "Profile" : panel === "notifications" ? "Operations inbox" : "About this workspace"} onClose={() => setPanel("")}>
      {panel === "profile" ? <><div className="profile-summary"><span className="avatar large">{live ? identity.initials : "AM"}</span><div><h3>{live ? identity.name : "Alex Morgan"}</h3><p>{live ? (identity.email || "no email on this account") : "alex@example.test"}</p></div></div>{live
      ? <><p>Signed in through Keycloak. Your role comes from the access token and is enforced by the API — it cannot be changed here.</p><Field label="Role"><input value={state.role} readOnly aria-readonly="true"/></Field><div className="modal-actions"><button onClick={() => { const cfg = authConfig(); if (cfg) endSession(cfg, window.location.origin); }}>Sign out</button><button className="primary" onClick={() => setPanel("")}>Done</button></div></>
      : <><p>Role selection changes the displayed persona only. It does not enforce authorization.</p><Field label="Demo persona"><select value={state.role} onChange={event => update(current => ({ ...current, role: event.target.value as DemoState["role"] }))}>{["Admin", "Supervisor", "Lead"].map(role => <option key={role}>{role}</option>)}</select></Field><div className="modal-actions"><button onClick={() => update(current => ({ ...current, session: false }))}>Leave demo session</button><button className="primary" onClick={() => setPanel("")}>Done</button></div></>}</> : panel === "notifications" ? <div className="stack"><Link className="inbox-item" href="/anomaly" onClick={() => setPanel("")}><Badge tone="failed">Review</Badge><div><strong>GPS variance requires review</strong><p>3 synthetic app/vehicle comparisons exceed the demo threshold.</p></div></Link><Link className="inbox-item" href="/lhs" onClick={() => setPanel("")}><Badge tone="assigned">LHS</Badge><div><strong>Daily driver reports are ready</strong><p>Review visit activity and sample operational expenses.</p></div></Link></div> : <div className="stack">{live
        ? <><p>Create and assign tasks, compare routes, review driver reports, and configure workflows.</p><div className="info-box">Tasks, hubs, teams, users and organization settings are stored by the API in the shared database — creating, editing and deleting them affects real records for everyone in your organization. Screens not yet connected to the API keep their state in this browser only; those still say so on the page.</div></>
        : <><p>Create and assign tasks, compare schematic routes, review driver reports, and configure workflows. Changes stay in localStorage on this browser.</p><div className="info-box">All names, email addresses, locations, invoices, and GPS records are synthetic. No emails, payments, backend requests, or real deletions occur. Do not enter personal or confidential information.</div></>}<p>Maps default to a schematic, not a street map. Optional Google Maps uses only the official Embed API with a configured key.</p><button className="danger" onClick={() => setPanel("reset")}>Reset local demo data</button></div>}
      {panel === "reset" && <div className="confirm-box"><p>This replaces edits in this browser with the original synthetic fixtures. It does not affect any external system.</p><button className="danger" onClick={() => { reset(); setPanel(""); }}>Confirm local reset</button></div>}
    </Modal>}
  </div>;
}
