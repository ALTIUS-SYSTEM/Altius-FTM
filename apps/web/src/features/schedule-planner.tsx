"use client";

import { useMemo, useState } from "react";
import { useDemo } from "@/components/demo-provider";
import { Badge, Card, Icon, Table } from "@/components/ui";
import { today, DRIVERS } from "@/data/model";
import { csvCell, downloadText } from "@/data/adapter";

interface PlanRow {
  id: string;
  date: string;
  activity: string;
  customer: string;
  fleet: string;
  staff: string;
  origin: string;
  destination: string;
  quantity: string;
  notes: string;
  tone: string;
  category: "fleet" | "packing";
}

const ACTIVITIES = [
  { key: "FCL", tone: "failed", fleet: "Tronton 1x40HC", unit: "FCL", category: "fleet" as const },
  { key: "LTL", tone: "assigned", fleet: "CDD Box Std", unit: "LTL", category: "fleet" as const },
  { key: "PICKUP", tone: "completed", fleet: "Traga Box", unit: "koli", category: "fleet" as const },
  { key: "DROPPING", tone: "in-progress", fleet: "CDD Box Std", unit: "koli", category: "fleet" as const },
  { key: "SHUTTLE", tone: "unassigned", fleet: "Traga Box", unit: "FCL", category: "fleet" as const },
  { key: "PACKING KAYU", tone: "failed", unit: "koli", category: "packing" as const },
  { key: "UKUR TIMBANAN", tone: "completed", unit: "koli", category: "packing" as const },
  { key: "WRAPPING ONLY", tone: "assigned", unit: "koli", category: "packing" as const },
];

const hash = (s: string) => {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h;
};

const quantityFor = (unit: string, h: number) => {
  if (unit === "FCL" || unit === "LTL") return `1 ${unit}`;
  if (unit === "koli") {
    const n = (h % 20) + 1;
    return `${n} koli`;
  }
  return "1";
};

const notesFor = (activity: string, destination: string) => {
  const map: Record<string, string> = {
    FCL: "Tujuan stuffing luar",
    LTL: "Respon 11.00, masuk dalam perencanaan armada",
    PICKUP: "Titip kembali DO / resi",
    DROPPING: "Konfirmasi penerima sebelum sampai",
    SHUTTLE: " Antar barang ke pool berikutnya",
    "PACKING KAYU": "Kemas kayu + fasten di truk",
    "UKUR TIMBANAN": "Timbang bersama petugas jembatan timbang",
    "WRAPPING ONLY": "Lapisi stretch wrap, tidak perlu packing kayu",
  };
  return map[activity] ?? `Operasional ke ${destination}`;
};

const usePlanRows = (hub: string): PlanRow[] => {
  const { state } = useDemo();
  return useMemo(() => {
    const rows: PlanRow[] = [];
    let index = 0;
    for (const task of state.tasks.filter((t) => t.hub === state.hub)) {
      const h = hash(task.id + String(index));
      const activity = ACTIVITIES[h % ACTIVITIES.length];
      const driverIndex = h % DRIVERS.length;
      const staff = task.assignee || DRIVERS[driverIndex];
      const fleet = activity.fleet ?? "-";
      const quantity = quantityFor(activity.unit, h);
      const notes = task.notes || notesFor(activity.key, task.address);
      rows.push({
        id: `${task.id}-${index}`,
        date: task.date || today(),
        activity: activity.key,
        customer: task.title,
        fleet,
        staff,
        origin: activity.category === "fleet" ? hub : "-",
        destination: task.address || task.hub,
        quantity,
        notes,
        tone: activity.tone,
        category: activity.category,
      });
      index++;
    }
    return rows.sort((a, b) => a.activity.localeCompare(b.activity));
  }, [state.tasks, state.hub, hub]);
};

export function Schedule() {
  const { state } = useDemo();
  const rows = usePlanRows(state.hub);
  const [search, setSearch] = useState("");
  const [mode, setMode] = useState<"all" | "fleet" | "packing">("all");
  const [activityFilter, setActivityFilter] = useState<string>("all");

  const allActivities = useMemo(
    () => Array.from(new Set(rows.map((r) => r.activity))).sort((a, b) => a.localeCompare(b)),
    [rows],
  );

  const filtered = useMemo(() => {
    return rows.filter((r) => {
      if (mode !== "all" && r.category !== mode) return false;
      if (activityFilter !== "all" && r.activity !== activityFilter) return false;
      if (!search) return true;
      const q = search.toLowerCase();
      return (
        r.customer.toLowerCase().includes(q) ||
        r.destination.toLowerCase().includes(q) ||
        r.activity.toLowerCase().includes(q) ||
        r.staff.toLowerCase().includes(q)
      );
    });
  }, [rows, mode, activityFilter, search]);

  const fleetMode = mode === "all" || mode === "fleet";

  const headings = [
    "Tanggal",
    "Aktivitas",
    "Customer",
    ...(fleetMode ? ["Armada", "Staff", "Dari"] : []),
    "Tujuan",
    "Jumlah",
    "Catatan",
  ];

  // Every cell goes through csvCell: customer/destination/notes carry
  // backend-supplied text, so an unquoted join lets a task title inject
  // spreadsheet formulas or forge extra rows with an embedded comma/newline.
  const csv = () => {
    const rows = [
      ["Date", "Activity", "Customer", "Fleet", "Staff", "Origin", "Destination", "Quantity", "Notes"],
      ...filtered.map((r) => [
        r.date,
        r.activity,
        r.customer,
        r.fleet,
        r.staff,
        r.origin,
        r.destination,
        r.quantity,
        r.notes,
      ]),
    ];
    return rows.map((row) => row.map(csvCell).join(",")).join("\r\n");
  };

  return (
    <>
      <div className="toolbar">
        <label className="field">
          <span>Tipe jadwal</span>
          <select value={mode} onChange={(e) => setMode(e.target.value as typeof mode)}>
            <option value="all">Semua</option>
            <option value="fleet">Fleet</option>
            <option value="packing">Packing</option>
          </select>
        </label>
        <label className="field">
          <span>Aktivitas</span>
          <select value={activityFilter} onChange={(e) => setActivityFilter(e.target.value)}>
            <option value="all">Semua</option>
            {allActivities.map((a) => (
              <option key={a} value={a}>
                {a}
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
                <td>
                  <Badge tone={r.tone}>{r.activity}</Badge>
                </td>
                <td>{r.customer}</td>
                {fleetMode && (
                  <>
                    <td>{r.fleet}</td>
                    <td>{r.staff}</td>
                    <td>{r.origin}</td>
                  </>
                )}
                <td>{r.destination}</td>
                <td>{r.quantity}</td>
                <td>{r.notes}</td>
              </tr>
            ))}
          </Table>
        ) : (
          <div className="empty">
            <div className="empty-symbol">
              <Icon name="tasks" size={32} />
            </div>
            <h3>Jadwal tidak ditemukan</h3>
            <p>Coba ubah filter pencarian atau mode tampilan.</p>
          </div>
        )}
      </Card>
      <div className="info-box">
        Tabel ini menyatukan rencana fleet dan packing harian. Data berasal dari daftar tugas demo yang dipetakan
        otomatis ke aktivitas operasional.
      </div>
    </>
  );
}
