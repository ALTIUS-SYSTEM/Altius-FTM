"use client";

import { useMemo, useState } from "react";
import { useDemo } from "@/components/demo-provider";
import { Badge, Card, Icon, Table } from "@/components/ui";
import { today, STATUS_LABELS } from "@/data/model";
import { csvCell, downloadText } from "@/data/adapter";

/**
 * The day's schedule, read straight off the tasks.
 *
 * Every column here is a field somebody typed into the task editor and the API
 * stored. This screen used to derive an "activity", a vehicle, a quantity and
 * a set of Indonesian operating notes from `hash(task.id)` — stable per task,
 * plausible on screen, and entirely invented. It was exportable to CSV, so the
 * fiction travelled into spreadsheets that people plan real days from.
 */

interface PlanRow {
  id: string;
  date: string;
  time: string;
  flow: string;
  customer: string;
  staff: string;
  origin: string;
  destination: string;
  priority: string;
  notes: string;
  status: keyof typeof STATUS_LABELS;
}

/** Blank cells read better than empty ones, and say "not set" out loud. */
const orDash = (value: string) => (value.trim() ? value : "—");

const usePlanRows = (): PlanRow[] => {
  const { state } = useDemo();
  return useMemo(
    () =>
      state.tasks
        .filter((task) => task.hub === state.hub)
        .map((task) => ({
          id: task.id,
          date: task.date || today(),
          time: task.time,
          flow: task.flow,
          customer: task.title,
          staff: task.assignee,
          origin: task.hub,
          destination: task.address,
          priority: task.priority,
          notes: task.notes,
          status: task.status,
        }))
        // A schedule reads in the order the day happens.
        .sort((a, b) => `${a.date} ${a.time}`.localeCompare(`${b.date} ${b.time}`)),
    [state.tasks, state.hub],
  );
};

export function Schedule() {
  const rows = usePlanRows();
  const [search, setSearch] = useState("");
  const [flowFilter, setFlowFilter] = useState<string>("all");

  // Only the workflows actually present, so the filter can never offer one
  // that would empty the table.
  const flows = useMemo(
    () => Array.from(new Set(rows.map((r) => r.flow).filter(Boolean))).sort((a, b) => a.localeCompare(b)),
    [rows],
  );

  const filtered = useMemo(
    () =>
      rows.filter((r) => {
        if (flowFilter !== "all" && r.flow !== flowFilter) return false;
        if (!search) return true;
        const q = search.toLowerCase();
        // Every field here is optional in the model, so match defensively:
        // an unassigned task has no staff, and reading it as a string threw.
        return [r.customer, r.destination, r.staff, r.flow].some((field) =>
          (field ?? "").toLowerCase().includes(q),
        );
      }),
    [rows, flowFilter, search],
  );

  const headings = ["Tanggal", "Jam", "Alur kerja", "Customer", "Staff", "Dari", "Tujuan", "Prioritas", "Catatan"];

  // Every cell goes through csvCell: customer/destination/notes carry
  // backend-supplied text, so an unquoted join lets a task title inject
  // spreadsheet formulas or forge extra rows with an embedded comma/newline.
  const csv = () => {
    const rows = [
      ["Date", "Time", "Workflow", "Customer", "Staff", "Origin", "Destination", "Priority", "Status", "Notes"],
      ...filtered.map((r) => [
        r.date,
        r.time,
        r.flow,
        r.customer,
        r.staff,
        r.origin,
        r.destination,
        r.priority,
        STATUS_LABELS[r.status],
        r.notes,
      ]),
    ];
    return rows.map((row) => row.map(csvCell).join(",")).join("\r\n");
  };

  return (
    <>
      <div className="toolbar">
        <label className="field">
          <span>Alur kerja</span>
          <select value={flowFilter} onChange={(e) => setFlowFilter(e.target.value)}>
            <option value="all">Semua</option>
            {flows.map((flow) => (
              <option key={flow} value={flow}>
                {flow}
              </option>
            ))}
          </select>
        </label>
        <div className="search-field">
          <Icon name="search" size={16} />
          <input
            aria-label="Search schedule"
            placeholder="Customer / tujuan / staff"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
        <div className="grow" />
        <button className="primary" onClick={() => downloadText(`altius-schedule-${today()}.csv`, csv())}>
          <Icon name="download" />
          Ekspor CSV
        </button>
      </div>
      <Card>
        {filtered.length ? (
          <Table count={filtered.length} headings={headings}>
            {filtered.map((r) => (
              <tr key={r.id}>
                <td>{r.date}</td>
                <td>{orDash(r.time)}</td>
                <td>
                  <Badge tone={r.status}>{orDash(r.flow)}</Badge>
                </td>
                <td>{r.customer}</td>
                <td>{orDash(r.staff)}</td>
                <td>{r.origin}</td>
                <td>{r.destination}</td>
                <td>{r.priority}</td>
                <td>{orDash(r.notes)}</td>
              </tr>
            ))}
          </Table>
        ) : (
          <div className="empty">
            <div className="empty-symbol">
              <Icon name="tasks" size={32} />
            </div>
            <h3>Jadwal tidak ditemukan</h3>
            <p>
              {rows.length
                ? "Coba ubah filter pencarian."
                : "Belum ada tugas di hub ini. Buat tugas di papan tugas untuk mengisi jadwal."}
            </p>
          </div>
        )}
      </Card>
      <div className="info-box">
        Jadwal ini dibaca langsung dari daftar tugas hub yang sedang dipilih. Warna lencana mengikuti status tugas.
      </div>
    </>
  );
}
