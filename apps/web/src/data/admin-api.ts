"use client";

import { call, asString, asNumber, pick } from "./api-client";

export interface Hub { id: string; name: string; lat: number; lng: number }
export interface Team { id: string; name: string; shift: string; hubId: string; members: string[] }

export const listHubs = async (): Promise<Hub[]> => {
  const rows = await call<Record<string, unknown>[]>("/api/v3/hubs");
  return (rows ?? []).map(r => {
    const hub = (r.hub ?? r) as Record<string, unknown>;
    return {
      id: asString(pick(hub, "id", "hub-id", "hub_id")),
      name: asString(pick(hub, "name", "display-name", "display_name")),
      lat: asNumber(pick(hub, "lat", "latitude")),
      lng: asNumber(pick(hub, "lng", "longitude")),
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
      id: asString(pick(team, "id", "team-id", "team_id")),
      name: asString(pick(team, "name", "display-name", "display_name")),
      shift: asString(team.shift),
      hubId: asString(r.hub),
      members: members
        .map(m => {
          const u = (m.user ?? m) as Record<string, unknown>;
          return asString(pick(u, "sub", "user-sub", "user_sub"));
        })
        .filter(Boolean),
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

export interface User {
  id: string;
  name: string;
  roles: string[];
}

export const listUsers = async (): Promise<User[]> => {
  const rows = await call<Record<string, unknown>[]>('/api/v3/users');
  return (rows ?? []).map(r => {
    const user = (r.user ?? r) as Record<string, unknown>;
    const role = pick(user, 'role_name', 'role-name', 'roles');
    const roles = Array.isArray(role)
      ? role.map(String)
      : typeof role === 'string'
        ? [role]
        : [];
    return {
      id: asString(pick(user, 'sub', 'user-sub', 'user_sub')),
      name: asString(pick(user, 'display_name', 'display-name', 'name', 'email', 'sub', 'user-sub')),
      roles,
    };
  });
};

/**
 * Drivers a task can actually be assigned to.
 *
 * The API resolves the assignee against org membership and rejects anything
 * else with "assign task", so a hard-coded roster produces a 400 on save. Falls
 * back to `display-name` because that is what the task rows carry as assignee.
 */
export const listDrivers = async (): Promise<string[]> => {
  const users = await listUsers();
  return users
    .filter(u => u.roles.some(r => r.toLowerCase() === "driver"))
    .map(u => u.name)
    .filter(Boolean)
    .sort((a, b) => a.localeCompare(b));
};

export const ROLES = ["super-admin", "admin", "supervisor", "lead", "driver"] as const;

export const setUserRoles = (subject: string, roles: readonly string[]) =>
  call<{ subject: string; roles: string[] }>(`/api/v3/users/${encodeURIComponent(subject)}/roles`, {
    method: "PUT",
    body: JSON.stringify({ roles }),
  });
