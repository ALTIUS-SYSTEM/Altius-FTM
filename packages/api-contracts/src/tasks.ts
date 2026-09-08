import { z } from 'zod';
import { CoordinateSchema, DeviceTimeSchema, IdSchema, NonnegativeIntegerSchema, RevisionSchema, UtcTimestampSchema } from './primitives';

export const TaskStatusSchema = z.enum(['unassigned', 'assigned', 'in_progress', 'completed', 'cancelled']);
export const StopStatusSchema = z.enum(['pending', 'arrived', 'working', 'completed', 'departed', 'skipped']);
export const StopActionSchema = z.enum(['arrive', 'start_activity', 'complete_activity', 'depart', 'skip']);
export const StopSchema = z.object({ id: IdSchema, sequence: NonnegativeIntegerSchema, name: z.string().min(1).max(200), address: z.string().min(1).max(1000), location: CoordinateSchema, status: StopStatusSchema, serviceSeconds: NonnegativeIntegerSchema }).strict().readonly();
export const TaskSchema = z.object({
  id: IdSchema, tenantId: IdSchema, hubId: IdSchema, title: z.string().min(1).max(200), driverId: IdSchema.nullable(), flowId: IdSchema.nullable(), status: TaskStatusSchema, revision: RevisionSchema, scheduledAtUtc: UtcTimestampSchema, stops: z.array(StopSchema).min(1).readonly()
}).strict().superRefine((value, context) => {
  if (new Set(value.stops.map(stop => stop.id)).size !== value.stops.length || new Set(value.stops.map(stop => stop.sequence)).size !== value.stops.length) context.addIssue({ code: 'custom', message: 'Duplicate stop ID or sequence' });
  if (['assigned', 'in_progress', 'completed'].includes(value.status) && value.driverId === null) context.addIssue({ code: 'custom', message: 'Assigned task requires a driver' });
  if (value.status === 'unassigned' && value.driverId !== null) context.addIssue({ code: 'custom', message: 'Unassigned task cannot have a driver' });
  if (value.status === 'completed' && value.stops.some(stop => !['completed', 'departed', 'skipped'].includes(stop.status))) context.addIssue({ code: 'custom', message: 'Completed task has unfinished stops' });
}).readonly();
export const DeviceEventSchema = z.object({
  schemaVersion: z.literal(1),
  eventId: IdSchema,
  idempotencyKey: IdSchema,
  tenantId: IdSchema,
  hubId: IdSchema,
  actorId: IdSchema,
  deviceId: IdSchema,
  deviceSequence: NonnegativeIntegerSchema,
  taskId: IdSchema,
  stopId: IdSchema,
  expectedTaskRevision: RevisionSchema,
  action: StopActionSchema,
  time: DeviceTimeSchema,
  // `.nullish()`, not `.nullable()`: nullable still requires the key to be
  // present, so every event from a device that predates this field — i.e. every
  // app already in the field — would fail `.strict()` and take its whole batch
  // down with it. A field added to a contract that shipped devices already
  // speak has to tolerate absence, not just null.
  location: CoordinateSchema.nullish(),
  accuracyMeters: z.number().finite().nonnegative().nullish(),
  observationId: IdSchema.nullable(),
  reason: z.string().min(1).max(1000).nullable()
}).strict().superRefine((event, context) => {
  if (event.action === 'skip' && event.reason === null) context.addIssue({ code: 'custom', message: 'Skipping requires a reason' });
}).readonly();
export const SyncStatusSchema = z.enum(['queued', 'sending', 'accepted', 'conflict', 'rejected']);
export const EventReceiptSchema = z.discriminatedUnion('status', [
  z.object({ eventId: IdSchema, idempotencyKey: IdSchema, receivedAtUtc: UtcTimestampSchema, status: z.literal('accepted'), taskRevision: RevisionSchema, duplicate: z.boolean() }).strict(),
  z.object({ eventId: IdSchema, idempotencyKey: IdSchema, receivedAtUtc: UtcTimestampSchema, status: z.literal('conflict'), currentTaskRevision: RevisionSchema, code: z.enum(['revision_conflict', 'idempotency_conflict', 'invalid_transition']) }).strict(),
  z.object({ eventId: IdSchema, idempotencyKey: IdSchema, receivedAtUtc: UtcTimestampSchema, status: z.literal('rejected'), code: z.enum(['forbidden', 'invalid_event', 'not_found']) }).strict()
]).readonly();
export const CheckInOutSchema = z.object({ eventId: IdSchema, idempotencyKey: IdSchema, tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, deviceId: IdSchema, action: z.enum(['check_in', 'check_out']), time: DeviceTimeSchema, observationId: IdSchema.nullable() }).strict().readonly();
export const ComponentTypeSchema = z.enum(['photo', 'video', 'input', 'select', 'list', 'bill', 'scan_display', 'otp', 'capture', 'voice', 'print', 'view', 'subpage']);
export const FlowSchema = z.object({ id: IdSchema, tenantId: IdSchema, name: z.string().min(1).max(200), revision: RevisionSchema, components: z.array(z.object({ id: IdSchema, type: ComponentTypeSchema, label: z.string().min(1).max(200), required: z.boolean() }).strict().readonly()).readonly() }).strict().readonly();
export type TaskStatus = z.infer<typeof TaskStatusSchema>;
export type StopStatus = z.infer<typeof StopStatusSchema>;
export type StopAction = z.infer<typeof StopActionSchema>;
export type Stop = z.infer<typeof StopSchema>;
export type Task = z.infer<typeof TaskSchema>;
export type DeviceEvent = z.infer<typeof DeviceEventSchema>;
export type EventReceipt = z.infer<typeof EventReceiptSchema>;
export type SyncStatus = z.infer<typeof SyncStatusSchema>;
export type CheckInOut = z.infer<typeof CheckInOutSchema>;
export type ComponentType = z.infer<typeof ComponentTypeSchema>;
export type Flow = z.infer<typeof FlowSchema>;
