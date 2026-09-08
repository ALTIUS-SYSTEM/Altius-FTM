import { z } from 'zod';
import { CoordinateSchema, DeviceTimeSchema, IdSchema, NonnegativeIntegerSchema, RevisionSchema, UtcTimestampSchema } from './primitives';

const MS_PER_SECOND = 1000;

export const EtaEstimateSchema = z.object({ estimatedAtUtc: UtcTimestampSchema, earliestAtUtc: UtcTimestampSchema, latestAtUtc: UtcTimestampSchema, calculatedAtUtc: UtcTimestampSchema, source: z.enum(['demo_graph', 'directions_provider', 'manual']), uncertaintySeconds: NonnegativeIntegerSchema }).strict().superRefine((value, context) => {
  const earliest = Date.parse(value.earliestAtUtc), estimated = Date.parse(value.estimatedAtUtc), latest = Date.parse(value.latestAtUtc);
  if (earliest > estimated || estimated > latest || earliest < Date.parse(value.calculatedAtUtc)) context.addIssue({ code: 'custom', message: 'Invalid ETA interval' });
  if (Math.max(estimated - earliest, latest - estimated) > value.uncertaintySeconds * MS_PER_SECOND) context.addIssue({ code: 'custom', message: 'ETA interval exceeds declared uncertainty' });
}).readonly();
export const ActualArrivalSchema = z.object({ eventId: IdSchema, time: DeviceTimeSchema, source: z.literal('driver_report') }).strict().readonly();
export const StopTimingSchema = z.object({ stopId: IdSchema, plannedEta: EtaEstimateSchema.nullable(), liveEta: EtaEstimateSchema.nullable(), actualArrival: ActualArrivalSchema.nullable() }).strict().readonly();
export const RoutePlanSchema = z.object({
  id: IdSchema, tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, version: RevisionSchema, createdAtUtc: UtcTimestampSchema,
  source: z.enum(['demo_graph', 'directions_provider', 'manual']),
  orderedStopIds: z.array(IdSchema).min(1).max(500).refine(ids => new Set(ids).size === ids.length, 'Duplicate route stops').readonly(),
  polyline: z.array(CoordinateSchema).min(2).max(10000).readonly(),
  distanceMeters: z.number().finite().nonnegative(),
  travelSeconds: NonnegativeIntegerSchema,
  timings: z.array(StopTimingSchema).max(500).readonly()
}).strict().superRefine((value, context) => {
  if (new Set(value.timings.map(timing => timing.stopId)).size !== value.timings.length || value.timings.some(timing => !value.orderedStopIds.includes(timing.stopId))) context.addIssue({ code: 'custom', message: 'Unknown or duplicate stop timing' });
}).readonly();
export const RouteComparisonSchema = z.object({ routeId: IdSchema, routeVersion: RevisionSchema, tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, observationIds: z.array(IdSchema).max(5000).readonly(), recordedDistanceMeters: z.number().finite().nonnegative().nullable(), coverage: z.enum(['complete', 'partial', 'missing']), assessedAtUtc: UtcTimestampSchema }).strict().readonly();
export const GpsReviewSchema = z.object({ id: IdSchema, tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, appObservationId: IdSchema, vehicleObservationId: IdSchema.nullable(), classification: z.enum(['consistent', 'review_required', 'insufficient_data']), separationMeters: z.number().finite().nonnegative().nullable(), timeDeltaSeconds: z.number().finite().nonnegative().nullable(), reason: z.enum(['within_tolerance', 'separation', 'missing_pair', 'poor_accuracy', 'stale', 'scope_mismatch']), reviewedBy: IdSchema.nullable() }).strict().readonly();
export type EtaEstimate = z.infer<typeof EtaEstimateSchema>;
export type ActualArrival = z.infer<typeof ActualArrivalSchema>;
export type StopTiming = z.infer<typeof StopTimingSchema>;
export type RoutePlan = z.infer<typeof RoutePlanSchema>;
export type RouteComparison = z.infer<typeof RouteComparisonSchema>;
export type GpsReview = z.infer<typeof GpsReviewSchema>;
