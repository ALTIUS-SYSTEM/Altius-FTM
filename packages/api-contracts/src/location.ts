import { z } from 'zod';
import { CoordinateSchema, DeviceTimeSchema, IdSchema, UtcTimestampSchema } from './primitives';

export const GpsSourceSchema = z.enum(['app_gps', 'vehicle_gps', 'manual']);
export const GpsQualitySchema = z.enum(['accurate', 'degraded', 'unavailable']);
export const GpsObservationSchema = z.object({
  id: IdSchema,
  tenantId: IdSchema,
  hubId: IdSchema,
  driverId: IdSchema,
  vehicleId: IdSchema.nullable(),
  deviceId: IdSchema,
  time: DeviceTimeSchema,
  source: GpsSourceSchema,
  quality: GpsQualitySchema,
  position: CoordinateSchema.nullable(),
  accuracyMeters: z.number().finite().nonnegative().nullable(),
  speedMetersPerSecond: z.number().finite().nonnegative().nullable(),
  mockLocationReported: z.boolean().nullable()
}).strict().superRefine((value, context) => {
  if (value.quality === 'unavailable' && (value.position !== null || value.accuracyMeters !== null)) context.addIssue({ code: 'custom', message: 'Unavailable observation must not contain a fix' });
  if (value.quality !== 'unavailable' && (value.position === null || value.accuracyMeters === null)) context.addIssue({ code: 'custom', message: 'GPS fix requires position and accuracy' });
}).readonly();
export const LocationHistorySchema = z.object({ tenantId: IdSchema, hubId: IdSchema, driverId: IdSchema, observations: z.array(GpsObservationSchema).readonly() }).strict().superRefine((value, context) => {
  if (value.observations.some(item => item.tenantId !== value.tenantId || item.hubId !== value.hubId || item.driverId !== value.driverId)) context.addIssue({ code: 'custom', message: 'Observation scope mismatch' });
  if (new Set(value.observations.map(item => item.id)).size !== value.observations.length) context.addIssue({ code: 'custom', message: 'Duplicate observation' });
}).readonly();
export const ObservationReceiptSchema = z.object({ observationId: IdSchema, receivedAtUtc: UtcTimestampSchema }).strict().readonly();
export type GpsSource = z.infer<typeof GpsSourceSchema>;
export type GpsQuality = z.infer<typeof GpsQualitySchema>;
export type GpsObservation = z.infer<typeof GpsObservationSchema>;
export type LocationHistory = z.infer<typeof LocationHistorySchema>;
export type ObservationReceipt = z.infer<typeof ObservationReceiptSchema>;
