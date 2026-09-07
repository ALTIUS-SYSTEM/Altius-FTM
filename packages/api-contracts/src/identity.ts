import { z } from 'zod';
import { CoordinateSchema, IdSchema, TimeZoneSchema } from './primitives';

export const RoleSchema = z.enum(['admin', 'supervisor', 'lead', 'driver']);
export const PermissionSchema = z.enum(['tasks.read', 'tasks.manage', 'tasks.execute', 'routes.read', 'routes.manage', 'lhs.read', 'lhs.submit', 'lhs.review', 'gps.review', 'users.manage', 'settings.manage']);
export const MembershipSchema = z.object({ tenantId: IdSchema, role: RoleSchema, hubIds: z.array(IdSchema).min(1).refine(ids => new Set(ids).size === ids.length, 'Duplicate hubs').readonly(), permissions: z.array(PermissionSchema).readonly() }).strict().readonly();
export const OrganizationSchema = z.object({ id: IdSchema, name: z.string().min(1).max(200), timeZone: TimeZoneSchema }).strict().readonly();
export const HubSchema = z.object({ id: IdSchema, tenantId: IdSchema, name: z.string().min(1).max(200), timeZone: TimeZoneSchema, location: CoordinateSchema }).strict().readonly();
export const UserSchema = z.object({ id: IdSchema, displayName: z.string().min(1).max(200), email: z.string().email(), memberships: z.array(MembershipSchema).readonly(), active: z.boolean() }).strict().readonly();
export const CurrencySchema = z.object({ code: z.string().regex(/^[A-Z]{3}$/), minorUnitDigits: z.number().int().min(0).max(4), name: z.string().min(1).max(100) }).strict().readonly();
export type Role = z.infer<typeof RoleSchema>;
export type Permission = z.infer<typeof PermissionSchema>;
export type Membership = z.infer<typeof MembershipSchema>;
export type Organization = z.infer<typeof OrganizationSchema>;
export type Hub = z.infer<typeof HubSchema>;
export type User = z.infer<typeof UserSchema>;
export type Currency = z.infer<typeof CurrencySchema>;
