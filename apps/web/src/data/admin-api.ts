"use client";

import { apiBase } from "./api-adapter";
import { accessToken, authConfig } from "@/lib/auth";

export interface Hub { id: string; name: string; lat: number; lng: number }
export interface Team { id: string; name: string; shift: string; hubId: string; members: string[] }

/**
 * Authenticated call to the admin surface. Organization scope is resolved
 * server-side from the token — nothing here sends a tenant id, because a
 * client-supplied one would be a tenant selector the caller controls.
 */
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

export const listHubs = async (): Promise<Hub[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/hubs");
  return (rows ?? []).map(r => {
    const hub = (r.hub ?? r) as Record<string, unknown>;
    return {
      id: asString(hub["hub-id"]),
      name: asString(hub["display-name"]),
      lat: asNumber(hub.latitude),
      lng: asNumber(hub.longitude),
    };
  });
};

export const createHub = (h: Omit<Hub, "id"> & { id?: string }) =>
  call<{ hubId: string }>("/api/v3/hubs-create", { method: "POST", body: JSON.stringify(h) });

export const updateHub = (id: string, h: Omit<Hub, "id">) =>
  call<{ ok: true }>(`/api/v3/hubs/${encodeURIComponent(id)}`, { method: "PATCH", body: JSON.stringify(h) });

export const deleteHub = (id: string) =>
  call<{ ok: true }>(`/api/v3/hubs/${encodeURIComponent(id)}`, { method: "DELETE" });

export const renameOrganization = (name: string) =>
  call<{ ok: true }>("/api/v3/organization", { method: "PATCH", body: JSON.stringify({ name }) });

export const listTeams = async (): Promise<Team[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/teams");
  return (rows ?? []).map(r => {
    const team = (r.team ?? {}) as Record<string, unknown>;
    const members = Array.isArray(r.members) ? (r.members as Record<string, unknown>[]) : [];
    return {
      id: asString(team["team-id"]),
      name: asString(team["display-name"]),
      shift: asString(team.shift),
      hubId: asString(r.hub),
      members: members.map(m => asString(((m.user ?? {}) as Record<string, unknown>)["user-sub"])).filter(Boolean),
    };
  });
};

export const createTeam = (t: { id?: string; name: string; shift: string; hub_id?: string }) =>
  call<{ teamId: string }>("/api/v3/teams", { method: "POST", body: JSON.stringify(t) });

export const updateTeam = (id: string, t: { name: string; shift: string }) =>
  call<{ ok: true }>(`/api/v3/teams/${encodeURIComponent(id)}`, { method: "PATCH", body: JSON.stringify(t) });

export const deleteTeam = (id: string) =>
  call<{ ok: true }>(`/api/v3/teams/${encodeURIComponent(id)}`, { method: "DELETE" });

export const addTeamMember = (id: string, subject: string) =>
  call<{ ok: true }>(`/api/v3/teams/${encodeURIComponent(id)}/members`, { method: "POST", body: JSON.stringify({ subject }) });

export const removeTeamMember = (id: string, subject: string) =>
  call<{ ok: true }>(`/api/v3/teams/${encodeURIComponent(id)}/members`, { method: "DELETE", body: JSON.stringify({ subject }) });

export const ROLES = ["admin", "supervisor", "lead", "driver"] as const;

export const setUserRoles = (subject: string, roles: readonly string[]) =>
  call<{ subject: string; roles: string[] }>(`/api/v3/users/${encodeURIComponent(subject)}/roles`, {
    method: "PUT",
    body: JSON.stringify({ roles }),
  });
