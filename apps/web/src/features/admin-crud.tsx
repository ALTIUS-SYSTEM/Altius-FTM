"use client";

import { useCallback, useEffect, useState } from "react";
import { Card, Empty, Field, Table } from "@/components/ui";
import {
  createHub, createTeam, deleteHub, deleteTeam, listHubs, listTeams,
  renameOrganization, updateHub, updateTeam, type Hub, type Team,
} from "@/data/admin-api";

/** Shared load/error/busy plumbing for the admin panels. */
function useResource<T>(load: () => Promise<T[]>) {
  const [rows, setRows] = useState<T[]>([]);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(true);
  const refresh = useCallback(() => {
    setBusy(true);
    load()
      .then(r => { setRows(r); setError(""); })
      .catch((e: Error) => setError(e.message))
      .finally(() => setBusy(false));
  }, [load]);
  useEffect(refresh, [refresh]);
  const run = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try { await action(); refresh(); }
    catch (e) { setError(e instanceof Error ? e.message : "Action failed."); setBusy(false); }
  };
  return { rows, error, busy, run, refresh };
}

export function HubsAdmin() {
  const { rows, error, busy, run } = useResource<Hub>(listHubs);
  const [draft, setDraft] = useState({ id: "", name: "", lat: "", lng: "" });
  const editing = draft.id !== "";

  const submit = () => run(() => {
    const lat = Number(draft.lat), lng = Number(draft.lng);
    if (!Number.isFinite(lat) || !Number.isFinite(lng)) throw new Error("Latitude and longitude must be numbers.");
    const body = { name: draft.name, lat, lng };
    const done = editing ? updateHub(draft.id, body) : createHub(body);
    setDraft({ id: "", name: "", lat: "", lng: "" });
    return done;
  });

  return <div className="grid-2">
    <Card title={`Hubs (${rows.length})`}>
      {error && <div className="error-banner" role="alert">{error}</div>}
      {rows.length === 0 && !busy
        ? <Empty title="No hubs" description="Create the first hub for this organization."/>
        : <Table headings={["Hub", "Position", ""]} count={rows.length}>
            {rows.map(h => <tr key={h.id}>
              <td>{h.name || h.id}</td>
              <td>{h.lat.toFixed(4)}, {h.lng.toFixed(4)}</td>
              <td className="row-actions">
                <button onClick={() => setDraft({ id: h.id, name: h.name, lat: String(h.lat), lng: String(h.lng) })}>Edit</button>
                <button onClick={() => run(() => deleteHub(h.id))} disabled={busy}>Delete</button>
              </td>
            </tr>)}
          </Table>}
    </Card>
    <Card title={editing ? "Edit hub" : "New hub"}>
      <Field label="Name"><input value={draft.name} onChange={e => setDraft({ ...draft, name: e.target.value })} maxLength={200}/></Field>
      <Field label="Latitude"><input value={draft.lat} onChange={e => setDraft({ ...draft, lat: e.target.value })} inputMode="decimal"/></Field>
      <Field label="Longitude"><input value={draft.lng} onChange={e => setDraft({ ...draft, lng: e.target.value })} inputMode="decimal"/></Field>
      <div className="row-actions">
        <button className="primary" onClick={submit} disabled={busy || !draft.name.trim()}>{editing ? "Save" : "Create"}</button>
        {editing && <button onClick={() => setDraft({ id: "", name: "", lat: "", lng: "" })}>Cancel</button>}
      </div>
      <div className="info-box">A hub with tasks or teams attached cannot be deleted — reassign them first, so nothing is orphaned.</div>
    </Card>
  </div>;
}

export function TeamsAdmin() {
  const { rows, error, busy, run } = useResource<Team>(listTeams);
  const [draft, setDraft] = useState({ id: "", name: "", shift: "" });
  const editing = draft.id !== "";

  const submit = () => run(() => {
    const done = editing
      ? updateTeam(draft.id, { name: draft.name, shift: draft.shift })
      : createTeam({ name: draft.name, shift: draft.shift });
    setDraft({ id: "", name: "", shift: "" });
    return done;
  });

  return <div className="grid-2">
    <Card title={`Teams (${rows.length})`}>
      {error && <div className="error-banner" role="alert">{error}</div>}
      {rows.length === 0 && !busy
        ? <Empty title="No teams" description="Create a team to roster drivers within a hub."/>
        : <Table headings={["Team", "Shift", "Members", ""]} count={rows.length}>
            {rows.map(t => <tr key={t.id}>
              <td>{t.name || t.id}</td>
              <td>{t.shift || "—"}</td>
              <td>{t.members.length}</td>
              <td className="row-actions">
                <button onClick={() => setDraft({ id: t.id, name: t.name, shift: t.shift })}>Edit</button>
                <button onClick={() => run(() => deleteTeam(t.id))} disabled={busy}>Delete</button>
              </td>
            </tr>)}
          </Table>}
    </Card>
    <Card title={editing ? "Edit team" : "New team"}>
      <Field label="Name"><input value={draft.name} onChange={e => setDraft({ ...draft, name: e.target.value })} maxLength={200}/></Field>
      <Field label="Shift"><input value={draft.shift} onChange={e => setDraft({ ...draft, shift: e.target.value })} placeholder="e.g. 06:00–14:00"/></Field>
      <div className="row-actions">
        <button className="primary" onClick={submit} disabled={busy || !draft.name.trim()}>{editing ? "Save" : "Create"}</button>
        {editing && <button onClick={() => setDraft({ id: "", name: "", shift: "" })}>Cancel</button>}
      </div>
      <div className="info-box">New teams are stationed at your own hub. Members are drivers already in this organization.</div>
    </Card>
  </div>;
}

export function OrganizationAdmin() {
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");

  const save = async () => {
    setBusy(true); setError(""); setMessage("");
    try {
      await renameOrganization(name);
      setMessage("Organization renamed.");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not rename the organization.");
    } finally { setBusy(false); }
  };

  return <Card title="Organization">
    {error && <div className="error-banner" role="alert">{error}</div>}
    {message && <div className="info-box" role="status">{message}</div>}
    <Field label="Display name"><input value={name} onChange={e => setName(e.target.value)} maxLength={200}/></Field>
    <button className="primary" onClick={save} disabled={busy || !name.trim()}>Save</button>
    <div className="info-box">
      Renaming only. Creating and deleting organizations is tenant lifecycle, not an in-app form —
      a delete would cascade every hub, task and report the tenant owns.
    </div>
  </Card>;
}
