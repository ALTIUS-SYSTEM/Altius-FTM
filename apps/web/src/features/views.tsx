"use client";

import { useCallback, useEffect, useState } from "react";
import Link from "next/link";
import { useDemo } from "@/components/demo-provider";
import { Badge, Card, Empty, Field, Icon, Modal, Metric, Table } from "@/components/ui";
import { downloadText, tasksCsv } from "@/data/adapter";
import { DRIVERS } from "@/data/model";
import { listDrivers } from "@/data/admin-api";
import type { DemoRecord } from "@/data/model";
import { optimizeRoute, staticMapUrl, type OptimizedRoute } from "@/data/route-api";
import { ProvisionUser } from "./provision-user";
import { HubsAdmin, OrganizationAdmin, TeamsAdmin } from "./admin-crud";
import { snapshot, markReviewed, type GpsReview } from "@/data/monitoring-api";
import { listUsers, setUserRoles, ROLES, type User } from "@/data/admin-api";
import { listReports, listCosts, listVehicleChecks, reviewReport, formatMoney, type DailyReport, type VehicleCheck, type CostEntry as CostEntryRow } from "@/data/reports-api";

/**
 * Driver roster from the API, shared by the screens that summarise activity.
 *
 * `DRIVERS` is an empty fallback: the roster used to be four hard-coded names,
 * which made every count on the dashboard fiction and made task assignment fail
 * server-side. An empty list is honest — the cards read zero until the API answers.
 */
function useDriverRoster(): string[] {
  const [drivers, setDrivers] = useState<string[]>(DRIVERS);
  useEffect(() => {
    let active = true;
    listDrivers().then(list => { if (active) setDrivers(list); }).catch(() => { /* leave it empty */ });
    return () => { active = false; };
  }, []);
  return drivers;
}

/* ---------- shared helpers ---------- */
function SearchField({ value, onChange, placeholder = "Search" }: { value: string; onChange: (v: string) => void; placeholder?: string }) {
  return <div className="search-field"><Icon name="search" size={16}/><input aria-label={placeholder} placeholder={placeholder} value={value} onChange={e => onChange(e.target.value)}/></div>;
}
function useFiltered(kind: string) {
  const { state } = useDemo();
  const [search, setSearch] = useState("");
  const rows = state.records.filter(r => r.kind === kind && !r.archived && (r.hub === state.hub || kind === "datatype") && (!search || `${r.name} ${r.detail}`.toLowerCase().includes(search.toLowerCase())));
  return { rows, search, setSearch };
}
function RecordTable({ kind, headings, render, empty, onCreate }: { kind: string; headings: string[]; render: (r: DemoRecord) => React.ReactNode; empty: string; onCreate?: () => void }) {
  const { rows, search, setSearch } = useFiltered(kind);
  return <><div className="toolbar"><SearchField value={search} onChange={setSearch}/><div className="grow"/>{onCreate && <button className="primary" onClick={onCreate}><Icon name="plus"/>Create</button>}</div>
    <Card>{rows.length ? <Table count={rows.length} headings={headings}>{rows.map(render)}</Table> : <Empty title={empty}/>}</Card></>;
}
function RecordEditor({ kind, record, onClose }: { kind: string; record?: DemoRecord; onClose: () => void }) {
  const { state, update } = useDemo();
  const [draft, setDraft] = useState(record ?? { id: `${kind}-${crypto.randomUUID().slice(0, 8)}`, kind, name: "", detail: "", extra: "", hub: state.hub, status: "Active", archived: false });
  const [err, setErr] = useState("");
  return <Modal title={record ? `Edit ${record.name}` : `Create ${kind}`} onClose={onClose}><form onSubmit={e => { e.preventDefault(); if (draft.name.trim().length < 2) { setErr("Name requires at least 2 characters."); return; } update(c => ({ ...c, records: record ? c.records.map(r => r.id === record.id ? draft : r) : [...c.records, draft] }), "Saved locally."); onClose(); }}><div className="stack"><Field label="Name"><input required minLength={2} maxLength={120} value={draft.name} onChange={e => setDraft(d => ({ ...d, name: e.target.value }))}/></Field><Field label="Detail"><input maxLength={500} value={draft.detail} onChange={e => setDraft(d => ({ ...d, detail: e.target.value }))}/></Field><Field label="Extra"><input maxLength={500} value={draft.extra} onChange={e => setDraft(d => ({ ...d, extra: e.target.value }))}/></Field>{err && <p role="alert" className="error-banner">{err}</p>}<div className="modal-actions"><button type="button" onClick={onClose}>Cancel</button><button className="primary" type="submit">Save locally</button></div></div></form></Modal>;
}
const act = (r: DemoRecord, actions?: React.ReactNode) => <tr key={r.id}><td><button className="text-button">{r.name}</button><small className="cell-sub">{r.detail}</small></td><td>{r.extra}</td><td><Badge tone={r.status === "Active" || r.status === "Published demo" ? "completed" : "assigned"}>{r.status}</Badge></td><td>{actions ?? "—"}</td></tr>;

/* ---------- dashboard ---------- */
const LINE = [18, 22, 17, 26, 30, 24, 34];
export function Dashboard() {
  const { state, t } = useDemo();
  const roster = useDriverRoster();
  const tasks = state.tasks.filter(x => x.hub === state.hub);
  const done = tasks.filter(x => x.status === "completed").length;
  const running = tasks.filter(x => x.status === "in-progress").length;
  const failed = tasks.filter(x => x.status === "failed").length;
  const max = Math.max(...LINE);
  // An empty board makes every slice 0/0 = NaN, which React renders as a
  // literal "NaN" attribute and the browser rejects.
  const pct = (n: number) => (tasks.length ? (n / tasks.length) * 100 : 0);
  return <>
    <div className="metric-grid"><Metric label="Tasks today" value={tasks.length} detail={`${done} completed`} tone="primary"/><Metric label="In progress" value={running} detail="Across all drivers" tone="amber"/><Metric label="Failed" value={failed} detail="Needs review" tone="danger"/><Metric label="Drivers" value={roster.length} detail="On this org" /></div>
    <div className="grid-2">
      <Card title={t("tasks") + " · weekly activity"}><svg className="chart" viewBox="0 0 300 120" role="img" aria-label="Task activity line chart"><polyline fill="none" stroke="var(--cyan)" strokeWidth="2.5" points={LINE.map((v, i) => `${20 + i * 43},${105 - v / max * 80}`).join(" ")}/>{LINE.map((v, i) => <circle key={i} cx={20 + i * 43} cy={105 - v / max * 80} r="3" fill="var(--teal)"/>)}</svg></Card>
      <Card title="Status breakdown"><div className="donut-wrap"><svg viewBox="0 0 42 42" className="donut" role="img" aria-label="Task status distribution"><circle cx="21" cy="21" r="15.9" fill="none" stroke="var(--surface-high)" strokeWidth="6"/>{[["completed", done, "var(--teal)", 0], ["in-progress", running, "var(--amber)", done], ["failed", failed, "var(--danger)", done + running]].map(([k, v, c, off]) => <circle key={k as string} cx="21" cy="21" r="15.9" fill="none" stroke={c as string} strokeWidth="6" strokeDasharray={`${pct(v as number)} ${100 - pct(v as number)}`} strokeDashoffset={25 - pct(off as number)}/>)}</svg><ul className="legend">{[["Completed", done, "var(--teal)"], ["In progress", running, "var(--amber)"], ["Failed", failed, "var(--danger)"], ["Unassigned/assigned", tasks.length - done - running - failed, "var(--outline)"]].map(([l, v, c]) => <li key={l as string}><i style={{ background: c as string }}/>{l} <strong>{v}</strong></li>)}</ul></div></Card>
    </div>
    <Card title="Driver activity"><Table headings={["Driver", "Assigned", "Completed", "In progress", "Failed"]}>{roster.map(d => { const mine = tasks.filter(x => x.assignee === d); return <tr key={d}><td>{d}</td><td>{mine.length}</td><td>{mine.filter(x => x.status === "completed").length}</td><td>{mine.filter(x => x.status === "in-progress").length}</td><td>{mine.filter(x => x.status === "failed").length}</td></tr>; })}</Table></Card>
  </>;
}

/* ---------- tasks: tracking / schedule / gallery ---------- */
export { Schedule } from "./schedule-planner";

export function Tracking() {
  const { state } = useDemo();
  const roster = useDriverRoster();
  const mine = state.tasks.filter(x => x.hub === state.hub);
  // Live positions come from /api/v3/monitoring/vehicles, which the API gates
  // behind super-admin. A plain admin is legitimately refused, so a failure
  // here is reported as "not available to your role" rather than an error —
  // everything else on this screen still works without it.
  const [gps, setGps] = useState<{ count: string | number; detail: string }>({ count: "—", detail: "Checking…" });
  useEffect(() => {
    let active = true;
    snapshot()
      .then(data => { if (active) setGps({ count: data.observations.length, detail: "Last hour" }); })
      .catch(() => { if (active) setGps({ count: "—", detail: "Needs super-admin" }); });
    return () => { active = false; };
  }, []);
  return <><div className="metric-grid"><Metric label="Drivers" value={roster.length} detail="On this org"/><Metric label="Completed" value={mine.filter(x => x.status === "completed").length} detail="Today"/><Metric label="Running" value={mine.filter(x => x.status === "in-progress").length} detail="Active now" tone="amber"/><Metric label="Live GPS fixes" value={gps.count} detail={gps.detail}/></div>
    <div className="split"><Card title="Active drivers">{roster.map(d => <div className="driver-row" key={d}><span className="avatar">{d.split(" ").map(w => w[0]).join("")}</span><div><strong>{d}</strong><small className="cell-sub">{mine.filter(x => x.assignee === d && x.status === "in-progress").length} active · {mine.filter(x => x.assignee === d).length} total</small></div><Badge tone={mine.some(x => x.assignee === d && x.status === "in-progress") ? "completed" : "unassigned"}>{mine.some(x => x.assignee === d && x.status === "in-progress") ? "On route" : "Idle"}</Badge></div>)}</Card>
    <div className="map-panel"><span className="map-tag">Offline schematic · not a navigation map</span><svg viewBox="0 0 400 220" className="schematic"><rect width="400" height="220" fill="var(--surface-low)"/><path d="M40 180 L120 140 L200 150 L280 90 L350 60" stroke="var(--teal)" strokeWidth="3" fill="none"/><path d="M40 180 L120 140 L200 150 L300 130 L350 60" stroke="var(--amber)" strokeWidth="2" strokeDasharray="6 5" fill="none"/>{[[40, 180], [120, 140], [200, 150], [280, 90], [350, 60]].map(([x, y], i) => <g key={i}><circle cx={x} cy={y} r="8" fill="var(--cyan)"/><text x={x} y={y + 4} textAnchor="middle" fontSize="9" fill="var(--cyan-dark)" fontWeight="700">{i + 1}</text></g>)}</svg></div></div></>;
}
export function Gallery() {
  const { state } = useDemo();
  const done = state.tasks.filter(x => x.hub === state.hub && x.status === "completed");
  return <Card title="Task media gallery"><div className="info-box">Photo and video evidence collection is not part of this demo. Completed tasks appear as placeholders.</div><div className="gallery-grid">{done.map(x => <div key={x.id} className="gallery-item"><Icon name="tasks" size={28}/><span>{x.title}</span><small>No media attached</small></div>)}</div></Card>;
}

/* ---------- route ---------- */
export function RouteVisit() {
  const { state, update, notify } = useDemo();
  const [search, setSearch] = useState("");
  const visits = state.tasks.filter(x => x.hub === state.hub && (!search || x.title.toLowerCase().includes(search.toLowerCase())));
  const toggle = (id: string) => update(c => ({ ...c, selectedVisits: c.selectedVisits.includes(id) ? c.selectedVisits.filter(v => v !== id) : [...c.selectedVisits, id], routeGenerated: false }));
  return <div className="split"><Card title={`Visits (${state.selectedVisits.length}/${visits.length})`} action={<SearchField value={search} onChange={setSearch}/>}><div className="visit-list">{visits.map((v, i) => <label key={v.id} className="visit-row"><input type="checkbox" checked={state.selectedVisits.includes(v.id)} onChange={() => toggle(v.id)}/><span className="visit-num">{i + 1}</span><span className="visit-name">{v.title}<small className="cell-sub">{v.address}</small></span><Icon name="pin" size={15}/></label>)}</div><button className="primary wide-btn" onClick={() => { update(c => ({ ...c, routeGenerated: true })); notify("Demo route computed on a fixed graph — not road routing."); }}>Optimalkan · demo graph</button></Card>
    <div className="map-panel"><span className="map-tag">Schematic stops — simulated positions</span><svg viewBox="0 0 400 260" className="schematic"><rect width="400" height="260" fill="var(--surface-low)"/>{visits.slice(0, 9).map((v, i) => { const x = 40 + (i % 3) * 130, y = 50 + Math.floor(i / 3) * 70; return <g key={v.id}><circle cx={x} cy={y} r="10" fill={state.selectedVisits.includes(v.id) ? "var(--cyan)" : "var(--outline)"}/><text x={x} y={y + 4} textAnchor="middle" fontSize="10" fill="var(--card)" fontWeight="700">{i + 1}</text></g>; })}</svg></div></div>;
}
export function RouteConfig() {
  const { state, update } = useDemo();
  const cfg = state.routeConfig;
  const set = (patch: Partial<typeof cfg>) => update(c => ({ ...c, routeConfig: { ...c.routeConfig, ...patch } }));
  return <div className="grid-3"><Card title="Vehicle speed"><Field label="Average speed (km/h)"><input type="number" min={5} max={100} value={cfg.speed} onChange={e => set({ speed: Number(e.target.value) })}/></Field><Field label="Default visit time (min)"><input type="number" min={0} max={120} value={cfg.service} onChange={e => set({ service: Number(e.target.value) })}/></Field></Card>
  <Card title="Constraints"><Field label="Capacity limit (kg)"><input type="number" min={1} max={5000} value={cfg.capacity} onChange={e => set({ capacity: Number(e.target.value) })}/></Field><label className="check-row"><input type="checkbox" checked={cfg.returnHub} onChange={e => set({ returnHub: e.target.checked })}/>Return to hub</label><label className="check-row"><input type="checkbox" checked={cfg.avoidTolls} onChange={e => set({ avoidTolls: e.target.checked })}/>Avoid tolls</label></Card>
  <Card title="Vehicles">{cfg.vehicles.map(v => <div key={v} className="visit-row"><span className="visit-name">{v}</span></div>)}<div className="info-box">Demo configuration only. No route engine is contacted.</div></Card></div>;
}

export function RouteResult() {
  const { state } = useDemo();
  const [route, setRoute] = useState<OptimizedRoute | null>(null);
  const [mapUrl, setMapUrl] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  // Only stops the API gave coordinates for can be routed. Saying so beats
  // silently planning a route over a subset of the selection.
  const selected = state.tasks.filter(t => state.selectedVisits.includes(t.id));
  const located = selected.filter((t): t is typeof t & { lat: number; lng: number } =>
    typeof t.lat === "number" && typeof t.lng === "number");
  const missing = selected.length - located.length;

  useEffect(() => {
    if (!state.routeGenerated || located.length < 2) return;
    let objectUrl = "";
    let live = true;
    setLoading(true);
    setError("");
    const waypoints = located.map(t => ({ lat: t.lat, lng: t.lng }));
    const origin = waypoints[0]!;
    optimizeRoute(origin, waypoints.slice(1))
      .then(async result => {
        if (!live) return;
        setRoute(result);
        const ordered = [origin, ...result.order.map(i => waypoints[i + 1]!).filter(Boolean)];
        try {
          objectUrl = await staticMapUrl(ordered, result.polyline);
          if (live) setMapUrl(objectUrl); else URL.revokeObjectURL(objectUrl);
        } catch {
          // A missing map is not a failed route — keep the order and ETA.
        }
      })
      .catch((e: Error) => { if (live) setError(e.message); })
      .finally(() => { if (live) setLoading(false); });
    return () => { live = false; if (objectUrl) URL.revokeObjectURL(objectUrl); };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.routeGenerated, state.selectedVisits.join(",")]);

  if (!state.routeGenerated) return <Empty title="No route computed yet" description="Select visits and optimise." action={<Link className="primary link-btn" href="/route/visit">Go to visits</Link>}/>;
  if (located.length < 2) return <Empty title="Not enough located stops" description={`Routing needs at least two stops with coordinates. ${missing} of ${selected.length} selected stop(s) have no position yet.`} action={<Link className="primary link-btn" href="/route/visit">Back to visits</Link>}/>;

  const minutes = route ? Math.round(route.legSeconds.reduce((a, b) => a + b, 0) / 60) : null;
  const km = route ? (route.totalMeters / 1000).toFixed(1) : null;
  const ordered = route ? [located[0]!, ...route.order.map(i => located[i + 1]!).filter(Boolean)] : [];

  // Per-stop ETA as a clock time: cumulative leg seconds from now. Leg 0 is
  // the drive TO ordered[1], so ordered[0] (origin) has ETA = now. With N
  // stops there are N-1 legs.
  const now = new Date();
  const etaClock = (index: number): string | null => {
    if (!route || index >= route.legSeconds.length + 1) return null;
    const cumulative = route.legSeconds.slice(0, index).reduce((a, b) => a + b, 0);
    const arrival = new Date(now.getTime() + cumulative * 1000);
    return arrival.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  };

  return <div className="grid-2">
    <Card title="Optimised route">
      {loading && <p>Planning route…</p>}
      {error && <div className="error-banner" role="alert">{error}</div>}
      {route && <>
        <ol className="route-steps">{ordered.map((t, i) => {
          const eta = etaClock(i);
          const ata = t.arrival ?? null;
          return <li key={t.id}>
            <Badge tone="assigned">{i + 1}</Badge> {t.title}
            <small className="cell-sub">{t.address}</small>
            <small className="cell-sub">ETA: {eta ?? "—"} · ATA: {ata ?? "—"}</small>
          </li>;
        })}</ol>
        <div className="info-box">
          {km} km · {minutes} min drive time.
          {route.source === "live"
            ? " Google Directions with live road geometry."
            : " Offline nearest-neighbour fallback — straight-line distances, not road routing."}
          {missing > 0 && ` ${missing} selected stop(s) omitted for lack of coordinates.`}
        </div>
      </>}
    </Card>
    <div className="map-panel">
      <span className="map-tag">{route?.source === "live" ? "Google Static Maps" : "Schematic — no live routing"}</span>
      {mapUrl ? <img src={mapUrl} alt={`Route through ${ordered.length} stops`} className="route-map"/> : <p className="cell-sub">No map image available.</p>}
    </div>
  </div>;
}


/* ---------- flow ---------- */
export function FlowBuilder() {
  const { state, update, notify } = useDemo();
  const add = (type: string) => update(c => ({ ...c, workflow: [...c.workflow, { id: `step-${crypto.randomUUID().slice(0, 6)}`, label: `New ${type} step`, type, required: false }] }), "Step added locally.");
  const move = (i: number, dir: -1 | 1) => update(c => { const w = [...c.workflow]; const j = i + dir; if (j < 0 || j >= w.length) return c; [w[i], w[j]] = [w[j], w[i]]; return { ...c, workflow: w }; });
  return <div className="split"><Card title="Workflow canvas" action={<div className="button-row">{["Text", "Number", "Photo", "Location"].map(tp => <button key={tp} onClick={() => add(tp)}><Icon name="plus"/>{tp}</button>)}</div>}><ol className="flow-steps">{state.workflow.map((s, i) => <li key={s.id} className="flow-step"><span className="visit-num">{i + 1}</span><div className="flow-step-body"><strong>{s.label}</strong><small className="cell-sub">{s.type}{s.required ? " · required" : ""}</small></div><div className="button-row"><button aria-label={`Move ${s.label} up`} disabled={i === 0} onClick={() => move(i, -1)}>↑</button><button aria-label={`Move ${s.label} down`} disabled={i === state.workflow.length - 1} onClick={() => move(i, 1)}>↓</button><button className="danger" onClick={() => update(c => ({ ...c, workflow: c.workflow.filter(x => x.id !== s.id) }))}>Remove</button></div></li>)}</ol><button className="primary wide-btn" onClick={() => notify("Workflow saved locally — synthetic only.")}>Save workflow</button></Card>
    <Card title="Mobile preview"><div className="phone-preview"><div className="phone-bar"/>{state.workflow.slice(0, 4).map(s => <div key={s.id} className="preview-field"><span>{s.label}{s.required ? " *" : ""}</span></div>)}</div></Card></div>;
}
export function Automation() { const [editing, setEditing] = useState(false); return <><RecordTable kind="automation" headings={["Automation", "Rule", "Status", "Actions"]} render={r => act(r, <button onClick={() => setEditing(true)}>Edit</button>)} empty="No automations" onCreate={() => setEditing(true)}/>{editing && <RecordEditor kind="automation" onClose={() => setEditing(false)}/>}</>; }
export function WorkflowList() { const [editing, setEditing] = useState(false); return <><RecordTable kind="workflow" headings={["Workflow", "Steps", "Status", "Actions"]} render={r => act(r, <button onClick={() => setEditing(true)}>Edit</button>)} empty="No workflows" onCreate={() => setEditing(true)}/>{editing && <RecordEditor kind="workflow" onClose={() => setEditing(false)}/>}</>; }

/* ---------- data & import-export ---------- */
export function DataList() { const [editing, setEditing] = useState(false); return <><RecordTable kind="customer" headings={["Customer", "Address", "Code / location", "Status"]} render={r => act(r, <button onClick={() => setEditing(true)}>Edit</button>)} empty="No customers" onCreate={() => setEditing(true)}/>{editing && <RecordEditor kind="customer" onClose={() => setEditing(false)}/>}</>; }
export function DataType() { return <RecordTable kind="datatype" headings={["Type", "Fields", "Scope", "Status"]} render={r => act(r)} empty="No data types"/>; }
export function DataImport() {
  const { notify } = useDemo();
  return <Card title="Import demo data"><div className="info-box">Imported rows stay in this browser. Nothing is uploaded.</div><label className="upload-box"><input type="file" accept=".csv" onChange={e => { const f = e.target.files?.[0]; if (f) notify(`"${f.name}" selected — demo import does not parse or persist files.`); }}/><Icon name="download" size={28}/><span>Choose a CSV file to preview import</span></label></Card>;
}
export function DataExport() {
  const { state } = useDemo();
  return <Card title="Export demo data"><p>Download the current synthetic task set as CSV.</p><button className="primary" onClick={() => downloadText("altius-demo-tasks.csv", tasksCsv(state.tasks))}><Icon name="download"/>Export tasks CSV</button></Card>;
}

/* ---------- settings ---------- */
export function Users() {
  const [editing, setEditing] = useState(false);
  const [provisioning, setProvisioning] = useState(false);
  return <>
    <RecordTable kind="user" headings={["User", "Contact", "Role", "Status"]} render={r => act(r, <button onClick={() => setEditing(true)}>Edit</button>)} empty="No users" onCreate={() => setEditing(true)}/>
    <div className="row-actions"><button className="primary" onClick={() => setProvisioning(true)}>Create real account</button></div>
    {provisioning && <ProvisionUser onClose={() => setProvisioning(false)}/>}
    {editing && <RecordEditor kind="user" onClose={() => setEditing(false)}/>}
  </>;
}
export function Teams() { return <TeamsAdmin/>; }
export function Permissions() {
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let live = true;
    setLoading(true);
    listUsers()
      .then(u => { if (live) setUsers(u); })
      .catch(e => setError(e instanceof Error ? e.message : "Failed to load users."))
      .finally(() => setLoading(false));
    return () => { live = false; };
  }, []);

  const toggle = async (subject: string, role: string, checked: boolean) => {
    const u = users.find(x => x.id === subject);
    if (!u) return;
    const next = checked ? [...new Set([...u.roles, role])] : u.roles.filter(r => r !== role);
    setUsers(prev => prev.map(x => x.id === subject ? { ...x, roles: next } : x));
    try {
      await setUserRoles(subject, next);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Role update failed.");
      setUsers(prev => prev.map(x => x.id === subject ? u : x));
    }
  };

  return <><Card title="Role assignment"><Table count={users.length} headings={["User", ...ROLES]}>{users.map(u => <tr key={u.id}><td>{u.name || u.id}</td>{ROLES.map(r => <td key={r}><input type="checkbox" checked={u.roles.includes(r)} onChange={e => toggle(u.id, r, e.target.checked)}/></td>)}</tr>)}</Table>{loading && <p>Loading users...</p>}{error && <p role="alert" className="error-banner">{error}</p>}</Card><div className="info-box">Roles are stored in Keycloak and enforced by the API. Changes here call the backend; they do not persist only in this browser.</div></>;
}
export function Hubs() { return <HubsAdmin/>; }
export function Organization() { return <OrganizationAdmin/>; }
export function CustomModule() { const [editing, setEditing] = useState(false); return <><RecordTable kind="module" headings={["Module", "Description", "Type", "Status"]} render={r => act(r, <button onClick={() => setEditing(true)}>Edit</button>)} empty="No modules" onCreate={() => setEditing(true)}/>{editing && <RecordEditor kind="module" onClose={() => setEditing(false)}/>}</>; }
export function Trash() {
  const { state, update } = useDemo();
  const rows = state.records.filter(r => r.archived);
  return <Card title="Trash"><div className="info-box">Demo trash. Restoring affects only this browser.</div>{rows.length ? <Table headings={["Name", "Type", "Actions"]} count={rows.length}>{rows.map(r => <tr key={r.id}><td>{r.name}</td><td>{r.kind}</td><td><button onClick={() => update(c => ({ ...c, records: c.records.map(x => x.id === r.id ? { ...x, archived: false } : x) }), "Restored locally.")}>Restore</button></td></tr>)}</Table> : <Empty title="Trash is empty"/>}</Card>;
}
export function NoAccess() { return <Empty title="Add-ons are not part of this demo" description="Custom add-ons require a backend integration that this demo does not include."/>; }

/* ---------- billing ---------- */
const PLANS = [["Starter", "1,000 tasks/mo", "2 data sources"], ["Pro", "2,500 tasks/mo", "5 data sources · automation"], ["Enterprise", "Custom volume", "SLA · SSO · audit"]] as const;
export function Plan() { return <div className="grid-3">{PLANS.map(([name, quota, feat]) => <Card key={name} title={name}><div className="plan-price">{name === "Enterprise" ? "Custom" : "IDR 0"}<small>demo</small></div><ul className="plan-list"><li>{quota}</li><li>{feat}</li><li>Synthetic preview</li></ul><button disabled title="Billing is disabled in the demo">Not available in demo</button></Card>)}</div>; }
export function Subscription() { return <Card title="Current license"><h3 className="plan-name">Demo plan</h3><p>0 / 1,000 tasks used this cycle (synthetic).</p><div className="info-box">No real subscription or billing exists in this demo.</div></Card>; }
export function History() { return <RecordTable kind="invoice" headings={["Invoice", "Period", "Total", "Status"]} render={r => act(r)} empty="No invoices"/>; }

/* ---------- LHS (PRD b) ---------- */
export function Lhs() {
  const [reports, setReports] = useState<DailyReport[]>([]);
  const [costs, setCosts] = useState<CostEntryRow[]>([]);
  const [checks, setChecks] = useState<VehicleCheck[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState("");

  const refresh = useCallback(() => {
    setLoading(true);
    // Costs and checks are staff-only; a lead may legitimately be refused them
    // while still being allowed to see reports, so one 403 must not blank the
    // whole page. Settle all three and keep whatever came back.
    return Promise.allSettled([listReports(), listCosts(), listVehicleChecks()])
      .then(([r, c, v]) => {
        if (r.status === "fulfilled") setReports(r.value);
        setCosts(c.status === "fulfilled" ? c.value : []);
        setChecks(v.status === "fulfilled" ? v.value : []);
        setError(r.status === "rejected" ? (r.reason instanceof Error ? r.reason.message : "Reports could not be loaded.") : "");
      })
      .finally(() => setLoading(false));
  }, []);
  useEffect(() => { void refresh(); }, [refresh]);

  const decide = async (r: DailyReport, decision: "approved" | "revision_requested") => {
    setBusy(`${r.driver}/${r.day}`);
    try {
      await reviewReport(r.driver, r.day, decision);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : "The review could not be saved.");
    } finally {
      setBusy("");
    }
  };

  const costFor = (driver: string, day: string) =>
    costs.filter(c => c.driver === driver && c.day === day).reduce((sum, c) => sum + c.amountMinor, 0);
  const currency = costs[0]?.currency ?? "IDR";
  const totalCosts = costs.reduce((sum, c) => sum + c.amountMinor, 0);
  const visited = reports.reduce((sum, r) => sum + r.visited, 0);
  const completed = reports.reduce((sum, r) => sum + r.completed, 0);

  return <><div className="metric-grid">
    <Metric label="Reports" value={reports.length} detail="Submitted by drivers"/>
    <Metric label="Stops visited" value={visited} detail="From device events"/>
    <Metric label="Stops completed" value={completed} detail="Confirmed on device" tone="primary"/>
    <Metric label="Recorded costs" value={formatMoney(totalCosts, currency)} detail={`${costs.length} entries`}/>
  </div>
  {error && <p role="alert" className="error-banner">{error}</p>}
  <Card title="Laporan Harian Sopir">
    {loading ? <p>Loading reports…</p> : reports.length === 0
      ? <Empty title="No driver reports yet" />
      : <Table count={reports.length} headings={["Driver", "Date", "Vehicle", "Visited", "Completed", "Odometer", "Costs", "Status", "Actions"]}>
          {reports.map(r => <tr key={`${r.driver}-${r.day}`}>
            <td>{r.driverName}</td>
            <td>{r.day}</td>
            <td>{r.vehicle || "—"}</td>
            <td>{r.visited}</td>
            <td>{r.completed}</td>
            <td>{r.odometerEnd > r.odometerStart ? `${r.odometerEnd - r.odometerStart} km` : "—"}</td>
            <td>{formatMoney(costFor(r.driver, r.day), currency)}</td>
            <td><Badge tone={r.status === "approved" ? "completed" : r.status === "revision_requested" ? "failed" : "assigned"}>{r.status.replace("_", " ")}</Badge></td>
            <td>{r.status === "submitted"
              ? <div className="button-row">
                  <button disabled={busy === `${r.driver}/${r.day}`} onClick={() => void decide(r, "approved")}>Approve</button>
                  <button disabled={busy === `${r.driver}/${r.day}`} onClick={() => void decide(r, "revision_requested")}>Request revision</button>
                </div>
              : <span className="cell-sub">rev {r.revision}</span>}</td>
          </tr>)}
        </Table>}
  </Card>
  <Card title="Vehicle checks">
    {checks.length === 0
      ? <Empty title="No vehicle checks recorded" />
      : <Table count={checks.length} headings={["Driver", "Date", "Plate", "Type", "Distance", "Condition", "Notes"]}>
          {checks.map(v => <tr key={v.id}>
            <td>{v.driverName}</td>
            <td>{v.day}</td>
            <td>{v.plate || "—"}</td>
            <td>{v.vehicleType || "—"}</td>
            <td>{v.kmEnd > v.kmStart ? `${v.kmEnd - v.kmStart} km` : "—"}</td>
            <td><Badge tone={v.condition === "good" ? "completed" : "failed"}>{v.condition || "unknown"}</Badge></td>
            <td>{v.notes || "—"}</td>
          </tr>)}
        </Table>}
  </Card></>;
}


/* ---------- anomaly + geofence (PRD c,e,f) ---------- */
export function Anomaly() {
  const [rows, setRows] = useState<GpsReview[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    let live = true;
    setLoading(true);
    snapshot()
      .then(data => {
        if (live) setRows(data.reviews);
      })
      .catch(e => {
        // The API gates every /monitoring route behind super-admin, so an org
        // admin is refused. "forbidden" on its own reads like a broken page;
        // say which role is missing so it is actionable.
        const message = e instanceof Error ? e.message : "";
        setError(/forbidden/i.test(message)
          ? "GPS review needs the super-admin realm role. Your account does not have it, so this page has nothing to show."
          : (message || "Failed to load monitoring data."));
      })
      .finally(() => setLoading(false));
    return () => { live = false; };
  }, []);

  const flagged = rows.filter(r => r.classification === "review_required").length;
  const maxVariance = rows.reduce((m, r) => Math.max(m, r.variance), 0);
  const handleResolve = async (id: string) => {
    try {
      await markReviewed(id);
      setRows(prev => prev.map(r => r.id === id ? { ...r, reviewedBy: "me" } : r));
    } catch (e) {
      setError(e instanceof Error ? e.message : "Failed to mark reviewed.");
    }
  };

  return <><div className="metric-grid"><Metric label="Comparisons" value={rows.length} detail="Live comparisons"/><Metric label="Flagged" value={flagged} detail="Above threshold" tone={flagged ? "danger" : "primary"}/><Metric label="Max variance" value={`${maxVariance} m`} detail="App vs vehicle GPS"/><Metric label="Vehicles" value={loading ? "..." : "live"} detail="McEasy-linked" tone="primary"/></div>
  {error && <p role="alert" className="error-banner">{error}</p>}
  <Card title="App vs vehicle GPS comparison"><Table count={rows.length} headings={["Driver", "Vehicle", "Variance", "Reason", "Review", "Actions"]}>{rows.map(r => <tr key={r.id}><td>{r.driver}</td><td>{r.vehicle || "—"}</td><td>{r.variance} m</td><td><Badge tone={r.classification === "review_required" ? "failed" : r.classification === "insufficient_data" ? "assigned" : "completed"}>{r.reason || r.classification}</Badge></td><td><Badge tone={r.reviewedBy ? "completed" : "assigned"}>{r.reviewedBy ? "Resolved" : "Open"}</Badge></td><td><button disabled={!!r.reviewedBy} onClick={() => handleResolve(r.id)}>Resolve</button></td></tr>)}</Table></Card>
  <div className="info-box"><strong>Live data.</strong> Missing or unmatched samples are reported as insufficient data, never as fraud. Flags mean "review", not proof. Geofence uses corridor distance with a 2-sample hysteresis and 50 m accuracy gate.</div></>;
}
