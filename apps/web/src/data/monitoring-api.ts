"use client";

import { apiBase } from "./api-adapter";
import { accessToken, authConfig } from "@/lib/auth";

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const base = apiBase();
  const cfg = authConfig();
  if (!base || !cfg) throw new Error("The Altius API and Keycloak must be configured.");
  const token = await accessToken(cfg);
  if (!token) throw new Error("Your session expired. Sign in again.");
  const res = await fetch(`${base}${path}`, {
    ...init,
    headers: { authorization: `Bearer ${token}`, "content-type": "application/json", ...(init?.headers ?? {}) },
  });
  const body = (await res.json().catch(() => ({}))) as { data?: T; error?: { message?: string } };
  if (!res.ok) throw new Error(body.error?.message ?? `Request failed (${res.status}).`);
  return body.data as T;
}

const asString = (v: unknown, fallback = ""): string => (typeof v === "string" ? v : fallback);
const asNumber = (v: unknown): number => {
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : 0;
};

export interface GpsReview {
  id: string;
  driver: string;
  vehicle: string;
  classification: string;
  reason: string;
  variance: number;
  matched: number;
  reviewedBy?: string;
}

export interface MonitoringSnapshot {
  vehicles: Record<string, unknown>[];
  observations: Record<string, unknown>[];
  reviews: Record<string, unknown>[];
}

export interface MonitoringView {
  vehicles: Record<string, unknown>[];
  observations: Record<string, unknown>[];
  reviews: GpsReview[];
}

function reviewFromRow(r: Record<string, unknown>): GpsReview {
  const review = (r.review ?? r) as Record<string, unknown>;
  return {
    id: asString(review["review-id"]),
    driver: asString(review["user-sub"]),
    vehicle: asString(review["plate"]),
    classification: asString(review["classification"]),
    reason: asString(review["review-reason"]),
    variance: asNumber(review["separation-meters"]),
    matched: 0,
    reviewedBy: asString(review["reviewed-by"]),
  };
}

export const listReviews = async (): Promise<GpsReview[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/monitoring/reviews");
  return (rows ?? []).map(reviewFromRow);
};

export const snapshot = async (): Promise<MonitoringView> => {
  const data = await call<MonitoringSnapshot>("/api/v3/monitoring/vehicles");
  return {
    vehicles: Array.isArray(data?.vehicles) ? data.vehicles : [],
    observations: Array.isArray(data?.observations) ? data.observations : [],
    reviews: Array.isArray(data?.reviews) ? data.reviews.map(reviewFromRow) : [],
  };
};

export const markReviewed = (id: string) =>
  call<{ reviewed: boolean }>(`/api/v3/monitoring/reviews/${encodeURIComponent(id)}/reviewed`, { method: "POST" });
