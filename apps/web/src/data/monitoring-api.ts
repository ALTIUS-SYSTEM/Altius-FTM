"use client";

import { call, asString, asNumber } from "./api-client";

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
