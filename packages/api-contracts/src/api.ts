import { z } from 'zod';
import { IdSchema, RevisionSchema, UtcTimestampSchema } from './primitives';
import { DeviceEventSchema, EventReceiptSchema, TaskSchema } from './tasks';
import { LhsReportSchema } from './lhs';
import { RoutePlanSchema } from './routes';

export const ApiErrorSchema = z.object({ code: z.enum(['invalid_request', 'unauthenticated', 'forbidden', 'not_found', 'conflict', 'rate_limited', 'unavailable']), message: z.string().min(1).max(2000), fieldErrors: z.record(z.array(z.string())).optional(), retryable: z.boolean() }).strict().readonly();
export const ResponseMetaSchema = z.object({ requestId: IdSchema, serverTimeUtc: UtcTimestampSchema, mode: z.enum(['demo', 'live']) }).strict().readonly();
export function apiResponseSchema<T extends z.ZodTypeAny>(data: T) {
  return z.discriminatedUnion('ok', [
    z.object({ ok: z.literal(true), data, meta: ResponseMetaSchema }).strict(),
    z.object({ ok: z.literal(false), error: ApiErrorSchema, meta: ResponseMetaSchema }).strict()
  ]).readonly();
}
export function pageSchema<T extends z.ZodTypeAny>(item: T) {
  return z.object({ items: z.array(item).readonly(), nextCursor: z.string().min(1).max(500).nullable() }).strict().readonly();
}
export const EventBatchRequestSchema = z.object({ tenantId: IdSchema, hubId: IdSchema, events: z.array(DeviceEventSchema).min(1).max(500).readonly() }).strict().superRefine((value, context) => {
  if (value.events.some(event => event.tenantId !== value.tenantId || event.hubId !== value.hubId)) context.addIssue({ code: 'custom', message: 'Event scope mismatch' });
  if (new Set(value.events.map(event => event.eventId)).size !== value.events.length || new Set(value.events.map(event => event.idempotencyKey)).size !== value.events.length) context.addIssue({ code: 'custom', message: 'Duplicate event or idempotency key in batch' });
}).readonly();
export const EventBatchResponseSchema = apiResponseSchema(z.array(EventReceiptSchema).readonly());
export const TaskListResponseSchema = apiResponseSchema(pageSchema(TaskSchema));
export const RoutePlanResponseSchema = apiResponseSchema(RoutePlanSchema);
export const LhsReportResponseSchema = apiResponseSchema(LhsReportSchema);
export const EntityDataSchema = z.object({ id: IdSchema, tenantId: IdSchema, hubId: IdSchema, typeId: IdSchema, revision: RevisionSchema, values: z.record(z.union([z.string(), z.number().finite(), z.boolean(), z.null()])).readonly() }).strict().readonly();
export type ApiError = z.infer<typeof ApiErrorSchema>;
export type ResponseMeta = z.infer<typeof ResponseMetaSchema>;
export type EventBatchRequest = z.infer<typeof EventBatchRequestSchema>;
export type EventBatchResponse = z.infer<typeof EventBatchResponseSchema>;
export type TaskListResponse = z.infer<typeof TaskListResponseSchema>;
export type RoutePlanResponse = z.infer<typeof RoutePlanResponseSchema>;
export type LhsReportResponse = z.infer<typeof LhsReportResponseSchema>;
export type EntityData = z.infer<typeof EntityDataSchema>;
export type ApiResponse<T> = Readonly<{ ok: true; data: T; meta: ResponseMeta } | { ok: false; error: ApiError; meta: ResponseMeta }>;
