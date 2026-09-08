"use client";

import { apiBase } from "./api-adapter";
import { accessToken, authConfig } from "@/lib/auth";

export interface Waypoint { lat: number; lng: number }

export interface OptimizedRoute {
  /** Indices into the waypoints array, in visit order. */
  order: number[];
  legSeconds: number[];
  totalMeters: number;
  /** "live" = Google Directions, "nearest" = offline greedy fallback. */
  source: "live" | "nearest";
  polyline?: string;
}

const request = async (path: string, body: unknown): Promise<Response> => {
  const base = apiBase();
  const cfg = authConfig();
  if (!base || !cfg) throw new Error("Route planning needs the Altius API and Keycloak configured.");
  const token = await accessToken(cfg);
  if (!token) throw new Error("Your session expired. Sign in again to plan a route.");
  const res = await fetch(`${base}${path}`, {
    method: "POST",
    headers: { authorization: `Bearer ${token}`, "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!res.ok) throw new Error(`Route service unavailable (${res.status}).`);
  return res;
};

export async function optimizeRoute(origin: Waypoint, waypoints: Waypoint[]): Promise<OptimizedRoute> {
  const res = await request("/api/v3/route/optimize", { origin, waypoints });
  const { data } = (await res.json()) as { data: Record<string, unknown> };
  return {
    order: Array.isArray(data.order) ? (data.order as number[]) : [],
    legSeconds: Array.isArray(data.leg_seconds) ? (data.leg_seconds as number[]) : [],
    totalMeters: Number(data.total_meters ?? 0),
    source: data.source === "live" ? "live" : "nearest",
    polyline: typeof data.polyline === "string" ? data.polyline : undefined,
  };
}

/**
 * Fetch the route as a PNG from our own origin. The Maps key stays on the
 * server — the browser never sees a Google URL carrying the credential.
 * Returns an object URL the caller must revoke.
 */
export async function staticMapUrl(
  markers: Waypoint[],
  polyline: string | undefined,
  width = 640,
  height = 360,
): Promise<string> {
  const res = await request("/api/v3/route/static-map", { markers, polyline, width, height });
  return URL.createObjectURL(await res.blob());
}
