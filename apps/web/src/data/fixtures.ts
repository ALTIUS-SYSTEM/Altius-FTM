import { DEMO_DATE, DRIVERS } from "./model";
import type { DemoState, DemoRecord, DemoTaskStatus } from "./model";

const customers = ["Nusantara Market", "Gambir Fresh", "Menteng Corner", "Cikini Grocer", "Senayan Supply", "Kota Warehouse", "Kemang Pantry", "Tebet Central", "Sudirman Office", "Pacific Depot", "Bandung Mart", "Surabaya Fresh"];
const statuses: DemoTaskStatus[] = ["completed", "completed", "in-progress", "assigned", "unassigned", "in-progress", "failed", "assigned", "completed", "unassigned", "assigned", "in-progress"];
const record = (id: string, kind: string, name: string, detail: string, extra: string, hub = "Jakarta", status = "Active"): DemoRecord => ({ id, kind, name, detail, extra, hub, status, archived: false });

export const createFixtures = (): DemoState => ({
  version: 1, hub: "Jakarta", role: "Admin", session: false, locale: "en",
  tasks: customers.map((title, i) => ({ id: `ALT-${1041 + i}`, title, address: `${12 + i * 7} Jalan ${["Merdeka", "Veteran", "Sudirman", "Cikini"][i % 4]}, ${i === 10 ? "Bandung" : i === 11 ? "Surabaya" : "Jakarta"}`, hub: i === 10 ? "Bandung" : i === 11 ? "Surabaya" : "Jakarta", flow: i % 4 === 3 ? "Pickup" : "Delivery", assignee: statuses[i] === "unassigned" ? "" : DRIVERS[i % 4], status: statuses[i], date: DEMO_DATE, time: `${String(8 + Math.floor(i / 2)).padStart(2, "0")}:${i % 2 ? "30" : "00"}`, priority: i === 6 || i === 4 ? "High" : "Normal", notes: "Synthetic delivery fixture. Verify package count and capture proof at the receiving desk.", ...(statuses[i] === "completed" ? { arrival: `${DEMO_DATE}T08:12:00+07:00`, departure: `${DEMO_DATE}T08:26:00+07:00` } : {}) })),
  records: [
    ...DRIVERS.map((name, i) => record(`user-${i}`, "user", name, `${name.toLowerCase().replaceAll(" ", ".")}@example.test`, i < 2 ? "Driver · North delivery" : "Driver · Central delivery")),
    record("user-admin", "user", "Alex Morgan", "alex@example.test", "Admin · Operations"),
    record("team-1", "team", "North delivery", "Morning shift · 08:00–16:00", "Adi Pratama, Nadia Putri"),
    record("team-2", "team", "Central delivery", "Afternoon shift · 12:00–20:00", "Rizky Saputra, Sari Wibowo"),
    ...["Jakarta", "Bandung", "Surabaya"].map((hub, i) => record(`hub-${i}`, "hub", hub, `${18 + i} Logistics Avenue, ${hub}`, "Indonesia · Asia/Jakarta", hub)),
    ...customers.slice(0, 10).map((name, i) => record(`customer-${i}`, "customer", name, `${12 + i * 7} Jalan Merdeka, Jakarta`, `C-${String(i + 1).padStart(3, "0")} · -6.175, 106.827`)),
    record("type-1", "datatype", "Customer", "Name, code, address, coordinates", "4 fields · Shared"),
    record("type-2", "datatype", "Product", "SKU, name, quantity", "3 fields · Shared"),
    record("schedule-1", "schedule", "Morning replenishment", "Weekdays at 08:00", "Delivery · Adi Pratama"),
    record("automation-1", "automation", "Flag failed delivery", "When task fails → Add to review queue", "Local preview only", "Jakarta", "Draft"),
    record("workflow-1", "workflow", "Delivery with proof", "Arrival → Package check → Proof → Complete", "4 steps", "Jakarta", "Published demo"),
    record("module-1", "module", "Vehicle inspection", "Daily safety checklist for field teams", "Checklist · 5 fields", "Jakarta", "Draft"),
    record("trash-1", "customer", "Archived demo customer", "Synthetic sample archived on 06 Sep 2026", "Restorable locally"),
    record("invoice-1", "invoice", "DEMO-2026-009", "01 Sep 2026 · 30 Sep 2026", "IDR 0", "Jakarta", "Sample"),
  ].map(item => item.id === "trash-1" ? { ...item, archived: true } : item),
  routeConfig: { speed: 25, service: 15, capacity: 500, returnHub: true, avoidTolls: false, vehicles: ["B 2041 ALT", "B 2042 ALT"] },
  selectedVisits: ["ALT-1043", "ALT-1044", "ALT-1045", "ALT-1046", "ALT-1048", "ALT-1050"], routeGenerated: false,
  reviews: {}, permissions: {}, organization: { name: "Altius Logistics · Demo", currency: "IDR", passwordDays: "90" },
  workflow: [{ id: "step-1", label: "Report arrival", type: "Location", required: true }, { id: "step-2", label: "Package count", type: "Number", required: true }, { id: "step-3", label: "Proof of delivery", type: "Photo", required: true }, { id: "step-4", label: "Recipient name", type: "Text", required: true }]
});
