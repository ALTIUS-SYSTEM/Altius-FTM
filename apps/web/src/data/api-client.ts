"use client";

import { apiBase } from "./api-adapter";
import { accessToken, authConfig } from "@/lib/auth";

/**
 * Authenticated call to the Altius API.
 *
 * Organization scope is resolved server-side from the token — nothing here
 * sends a tenant id, because a client-supplied one would be a tenant selector
 * the caller controls. Unwraps the `{ data }` envelope every v3 route returns
 * and turns `{ error: { message } }` into a thrown Error.
 */
export async function call<T>(path: string, init?: RequestInit): Promise<T> {
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

/** API rows are another trust boundary: coerce rather than assume. */
/**
 * First usable value among several spellings of the same field.
 *
 * The two stores name their columns differently and the API passes each
 * through untouched: Postgres serialises its rows (`hub_id`, `display_name`),
 * TypeDB its attributes (`hub-id`, `display-name`). Readers that knew only one
 * spelling returned empty strings against the other backend, which is how the
 * admin screens came to list rows with blank names against the default
 * Postgres deployment.
 */
export const pick = (row: Record<string, unknown>, ...keys: string[]): unknown => {
  for (const key of keys) {
    const value = row[key];
    if (value !== undefined && value !== null && value !== "") return value;
  }
  return undefined;
};

export const asString = (v: unknown, fallback = ""): string => (typeof v === "string" ? v : fallback);
export const asNumber = (v: unknown): number => {
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : 0;
};
