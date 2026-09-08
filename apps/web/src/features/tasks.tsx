"use client";

import { useEffect, useState } from "react";
import { useDemo } from "@/components/demo-provider";
import { Badge, Card, Empty, Field, Icon, Modal, StatusBadge, Table } from "@/components/ui";
import { filterTasks, validateTask, downloadText, tasksCsv } from "@/data/adapter";
import { today, DRIVERS, FLOWS, STATUS_LABELS } from "@/data/model";
import type { DemoTask, DemoTaskStatus } from "@/data/model";
import { listDrivers } from "@/data/admin-api";

/**
 * The roster a task can be assigned to, from the API.
 *
 * Assignees are validated server-side against org membership, so offering a
 * name the org does not have produces a 400 on save with no hint as to why.
 * Existing assignees are merged in, otherwise editing a task assigned to
 * someone since removed would silently blank the field on the next save.
 */
function useAssignees(current: string[]): string[] {
  const [drivers, setDrivers] = useState<string[]>(DRIVERS);
  useEffect(() => {
    let active = true;
    listDrivers()
      .then(list => { if (active) setDrivers(list); })
      .catch(() => { /* keep whatever we have; the field stays usable */ });
    return () => { active = false; };
  }, []);
  return Array.from(new Set([...drivers, ...current.filter(Boolean)])).sort((a, b) => a.localeCompare(b));
}

export function TaskEditor({ task, onClose }: { task?: DemoTask; onClose: () => void }) {
  const { state, update, t } = useDemo();
  const [draft, setDraft] = useState<DemoTask>(task ?? { id: `ALT-${crypto.randomUUID().slice(0, 8)}`, title: "", address: "", hub: state.hub, flow: "Delivery", assignee: "", status: "unassigned", date: today(), time: "08:00", priority: "Normal", notes: "" });
  const [error, setError] = useState("");
  const assignees = useAssignees(draft.assignee ? [draft.assignee] : []);
  const set = (key: keyof DemoTask, value: string) => setDraft(current => ({ ...current, [key]: value }));
  return <Modal title={task ? `Edit ${task.id}` : "Create task"} onClose={onClose}><form onSubmit={event => { event.preventDefault(); try { const valid = validateTask(draft); update(current => ({ ...current, tasks: task ? current.tasks.map(item => item.id === task.id ? valid : item) : [...current.tasks, valid], routeGenerated: false }), "Task saved in this browser."); onClose(); } catch (error) { setError(error instanceof Error ? error.message : "Invalid task"); } }}><div className="form-grid"><Field label="Task title"><input required minLength={2} maxLength={120} value={draft.title} onChange={event => set("title", event.target.value)}/></Field><Field label="Address"><input required minLength={3} maxLength={240} value={draft.address} onChange={event => set("address", event.target.value)}/></Field><Field label={t("flow")}><select value={draft.flow} onChange={event => set("flow", event.target.value)}>{FLOWS.map(flow => <option key={flow}>{flow}</option>)}</select></Field><Field label={t("driver")}><select value={draft.assignee} onChange={event => setDraft(current => ({ ...current, assignee: event.target.value, status: event.target.value ? "assigned" : "unassigned" }))}><option value="">{t("unassigned")}</option>{assignees.map(driver => <option key={driver}>{driver}</option>)}</select></Field><Field label="Date"><input required type="date" value={draft.date} onChange={event => set("date", event.target.value)}/></Field><Field label="Start time"><input required type="time" value={draft.time} onChange={event => set("time", event.target.value)}/></Field><Field label={t("status")}><select value={draft.status} onChange={event => set("status", event.target.value)}>{Object.keys(STATUS_LABELS).map(status => <option key={status} value={status}>{t(status)}</option>)}</select></Field><Field label="Priority"><select value={draft.priority} onChange={event => set("priority", event.target.value)}><option>Normal</option><option>High</option></select></Field></div><Field label="Instructions"><textarea maxLength={2000} value={draft.notes} onChange={event => set("notes", event.target.value)}/></Field>{error && <p role="alert" className="error-banner">{error}</p>}<div className="modal-actions"><button type="button" onClick={onClose}>{t("cancel")}</button><button className="primary" type="submit">{t("save")}</button></div></form></Modal>;
}

export function TaskBoard() {
  const { state, update, t, notify } = useDemo();
  const rowAssignees = useAssignees(state.tasks.map(task => task.assignee));
  const [search, setSearch] = useState("");
  const [status, setStatus] = useState("");
  const [assignee, setAssignee] = useState("");
  const [flow, setFlow] = useState("");
  const [date, setDate] = useState("");
  const [selected, setSelected] = useState<string[]>([]);
  const [editor, setEditor] = useState<DemoTask | "new" | null>(null);
  const [detail, setDetail] = useState<string | null>(null);
  const [bulk, setBulk] = useState("");
  const [deleting, setDeleting] = useState(false);
  const tasks = filterTasks(state.tasks, { hub: state.hub, search, status, assignee, flow, date });
  const visibleSelection = selected.filter(id => tasks.some(task => task.id === id));
  const active = state.tasks.find(task => task.id === detail);
  const assign = (ids: string[], driver: string) => update(current => ({ ...current, tasks: current.tasks.map(task => ids.includes(task.id) ? { ...task, assignee: driver, status: driver ? "assigned" : "unassigned" } : task) }), `${ids.length} task assignment(s) updated locally.`);
  return <><div className="toolbar"><Field label={t("search")}><input placeholder="Task, ID or address" value={search} onChange={event => setSearch(event.target.value)}/></Field><Field label={t("status")}><select value={status} onChange={event => setStatus(event.target.value)}><option value="">{t("all")}</option>{Object.keys(STATUS_LABELS).map(key => <option key={key} value={key}>{t(key)}</option>)}</select></Field><Field label={t("driver")}><select value={assignee} onChange={event => setAssignee(event.target.value)}><option value="">{t("all")}</option>{rowAssignees.map(driver => <option key={driver}>{driver}</option>)}</select></Field><Field label={t("flow")}><select value={flow} onChange={event => setFlow(event.target.value)}><option value="">{t("all")}</option>{FLOWS.map(item => <option key={item}>{item}</option>)}</select></Field><Field label="Date"><input type="date" value={date} onChange={event => setDate(event.target.value)}/></Field><button onClick={() => { setSearch(""); setStatus(""); setAssignee(""); setFlow(""); setDate(""); }}>{t("reset")}</button></div><div className="section-toolbar"><div><h2>Today's dispatch board <span className="count">{tasks.length}</span></h2><p>Keep the next move clear for everyone.</p></div><div className="button-row"><button onClick={() => downloadText("altius-tasks-demo.csv", tasksCsv(tasks))}><Icon name="download"/>{t("export")}</button><button className="primary" onClick={() => setEditor("new")}><Icon name="plus"/>{t("create")} task</button></div></div>{visibleSelection.length > 0 && <div className="selection-bar"><strong>{visibleSelection.length} selected</strong><select aria-label="Bulk assignee" value={bulk} onChange={event => setBulk(event.target.value)}><option value="">Unassigned</option>{rowAssignees.map(driver => <option key={driver}>{driver}</option>)}</select><button onClick={() => { assign(visibleSelection, bulk); setSelected([]); }}>Apply assignment</button><button className="danger" onClick={() => setDeleting(true)}>Delete selected</button></div>}<Card>{tasks.length ? <Table count={tasks.length} headings={[<input key="select" type="checkbox" aria-label="Select all visible tasks" checked={tasks.length > 0 && visibleSelection.length === tasks.length} onChange={event => setSelected(event.target.checked ? tasks.map(task => task.id) : [])}/>, "Task / destination", t("flow"), "Start time", t("status"), t("driver"), "Actions"]}>{tasks.map(task => <tr key={task.id}><td><input type="checkbox" aria-label={`Select ${task.title}`} checked={selected.includes(task.id)} onChange={event => setSelected(event.target.checked ? [...selected, task.id] : selected.filter(id => id !== task.id))}/></td><td><button className="text-button" onClick={() => setDetail(task.id)}>{task.title}</button><small className="cell-sub">{task.id} · {task.address}</small></td><td><Badge>{task.flow}</Badge>{task.priority === "High" && <small className="cell-sub warning-text">High priority</small>}</td><td>{task.time}<small className="cell-sub">{task.date}</small></td><td><StatusBadge status={task.status}/></td><td><select aria-label={`Assign ${task.title}`} value={task.assignee} onChange={event => assign([task.id], event.target.value)}><option value="">{t("unassigned")}</option>{rowAssignees.map(driver => <option key={driver}>{driver}</option>)}</select></td><td><button onClick={() => setEditor(task)}>Edit</button></td></tr>)}</Table> : <Empty title={t("empty")}/>}</Card>{editor && <TaskEditor task={editor === "new" ? undefined : editor} onClose={() => setEditor(null)}/>} {active && <Modal title={active.title} onClose={() => setDetail(null)}><div className="stack"><StatusBadge status={active.status}/><p>{active.address}</p><p>{active.notes || "No instructions added."}</p><div className="info-box">Arrival: {active.arrival ?? "Not reported"}<br/>Departure: {active.departure ?? "Not reported"}<br/>Recorded against this task in the shared database.</div><div className="button-row">{([['Report arrival', 'in-progress'], ['Complete task', 'completed']] as const).map(([label, next]) => <button key={label} disabled={!active.assignee || active.status === "completed" || (next === "completed" && !active.arrival)} onClick={() => { const now = new Date().toISOString(); update(current => ({ ...current, tasks: current.tasks.map(task => task.id === active.id ? { ...task, status: next as DemoTaskStatus, ...(next === "completed" ? { departure: now } : { arrival: now }) } : task) })); notify("Task activity saved."); }}>{label}</button>)}</div></div></Modal>}{deleting && <Modal title="Delete selected tasks?" onClose={() => setDeleting(false)}><p>This permanently deletes {visibleSelection.length} task(s) from the shared database for everyone in your organization. It cannot be undone.</p><div className="modal-actions"><button onClick={() => setDeleting(false)}>{t("cancel")}</button><button className="danger" onClick={() => { update(current => ({ ...current, tasks: current.tasks.filter(task => !visibleSelection.includes(task.id)), selectedVisits: current.selectedVisits.filter(id => !visibleSelection.includes(id)), routeGenerated: false }), "Tasks deleted."); setSelected([]); setDeleting(false); }}>Delete permanently</button></div></Modal>}</>;
}
