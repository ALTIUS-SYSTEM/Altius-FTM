"use client";

import { call, asString, asNumber } from "./api-client";

/**
 * Driver daily reports (LHS), operating costs, and vehicle checks.
 *
 * All three are org-scoped server-side and share one shape: `{ data: [row] }`
 * where each row wraps the record under a single key — `report`, `entry` or
 * `check`. Staff see the whole organization; a driver sees only their own,
 * decided by the API from the token, not by anything sent from here.
 */

export interface DailyReport {
  driver: string;
  driverName: string;
  day: string;
  hub: string;
  vehicle: string;
  odometerStart: number;
  odometerEnd: number;
  notes: string;
  visited: number;
  completed: number;
  status: string;
  revision: number;
}

export interface CostEntry {
  id: string;
  driver: string;
  category: string;
  /** Minor units (sen). The API refuses floats for money. */
  amountMinor: number;
  currency: string;
  note: string;
  day: string;
  hub: string;
}

export interface VehicleCheck {
  id: string;
  driver: string;
  driverName: string;
  day: string;
  plate: string;
  vehicleType: string;
  kmStart: number;
  kmEnd: number;
  condition: string;
  notes: string;
  serviceDate: string;
  kirDate: string;
  stnkDate: string;
}

/** `visited_task_ids` / `completed_stop_ids` are jsonb arrays; count them. */
const countOf = (v: unknown): number => (Array.isArray(v) ? v.length : 0);

const unwrap = (row: Record<string, unknown>, key: string): Record<string, unknown> =>
  ((row[key] ?? row) as Record<string, unknown>) ?? {};

export const listReports = async (): Promise<DailyReport[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/reports");
  return (rows ?? []).map(row => {
    const r = unwrap(row, "report");
    return {
      driver: asString(r.driver_sub),
      driverName: asString(r.driver_name) || asString(r.driver_sub),
      day: asString(r.day),
      hub: asString(r.hub_id),
      vehicle: asString(r.vehicle_number),
      odometerStart: asNumber(r.odometer_start),
      odometerEnd: asNumber(r.odometer_end),
      notes: asString(r.notes),
      visited: countOf(r.visited_task_ids),
      completed: countOf(r.completed_stop_ids),
      status: asString(r.status, "submitted"),
      revision: asNumber(r.revision),
    };
  });
};

/** Staff review of one report: `approved` or `revision_requested`. */
export const reviewReport = (driver: string, day: string, decision: "approved" | "revision_requested", note?: string) =>
  call<{ reviewed: boolean }>(
    `/api/v3/reports/${encodeURIComponent(driver)}/${encodeURIComponent(day)}`,
    { method: "PATCH", body: JSON.stringify({ decision, note: note ?? "" }) },
  );

export const listCosts = async (): Promise<CostEntry[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/costs");
  return (rows ?? []).map(row => {
    const c = unwrap(row, "entry");
    return {
      id: asString(c.id),
      driver: asString(c.driver_sub),
      category: asString(c.category),
      amountMinor: asNumber(c.amount_minor),
      currency: asString(c.currency, "IDR"),
      note: asString(c.note),
      day: asString(c.day),
      hub: asString(c.hub_id),
    };
  });
};

export const listVehicleChecks = async (): Promise<VehicleCheck[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/vehicle-checks");
  return (rows ?? []).map(row => {
    const v = unwrap(row, "check");
    return {
      id: asString(v.id),
      driver: asString(v.driver_sub),
      driverName: asString(v.driver_name) || asString(v.driver_sub),
      day: asString(v.day),
      plate: asString(v.license_plate),
      vehicleType: asString(v.vehicle_type),
      kmStart: asNumber(v.km_start),
      kmEnd: asNumber(v.km_end),
      condition: asString(v.condition),
      notes: asString(v.notes),
      serviceDate: asString(v.service_date),
      kirDate: asString(v.kir_date),
      stnkDate: asString(v.stnk_date),
    };
  });
};

/** Money for display. Amounts are minor units, so never divide in the template. */
export const formatMoney = (minor: number, currency = "IDR"): string => {
  try {
    return new Intl.NumberFormat("id-ID", { style: "currency", currency, maximumFractionDigits: 0 })
      .format(minor / 100);
  } catch {
    return `${currency} ${(minor / 100).toLocaleString("id-ID")}`;
  }
};
