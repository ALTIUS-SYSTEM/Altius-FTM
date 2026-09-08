import { z } from 'zod';

export const IdSchema = z.string().min(1).max(128).regex(/^[A-Za-z0-9][A-Za-z0-9_.:-]*$/);
export const UtcTimestampSchema = z.string().datetime({ offset: false }).refine(value => Number.isFinite(Date.parse(value)), 'Invalid UTC instant');
export const DeviceTimeSchema = z.object({ occurredAtUtc: UtcTimestampSchema, utcOffsetMinutes: z.number().int().min(-840).max(840) }).strict().readonly();
export const DateOnlySchema = z.string().regex(/^\d{4}-\d{2}-\d{2}$/).refine(value => {
  const instant = Date.parse(`${value}T00:00:00Z`);
  return Number.isFinite(instant) && new Date(instant).toISOString().slice(0, 10) === value;
}, 'Invalid calendar date');
export const TimeZoneSchema = z.string().min(1).max(100).refine(value => {
  try { new Intl.DateTimeFormat('en', { timeZone: value }); return true; } catch { return false; }
}, 'Expected an IANA time zone');
export const CoordinateSchema = z.object({ latitude: z.number().finite().min(-90).max(90), longitude: z.number().finite().min(-180).max(180) }).strict().readonly();
export const NonnegativeIntegerSchema = z.number().int().nonnegative().max(Number.MAX_SAFE_INTEGER);
export const RevisionSchema = NonnegativeIntegerSchema;
/** 10^12 minor units is far above any real single line item and far below
 *  MAX_SAFE_INTEGER, so sums stay exact. */
export const MAX_AMOUNT_MINOR = 1_000_000_000_000;
export const MoneySchema = z.object({ amountMinor: NonnegativeIntegerSchema.max(MAX_AMOUNT_MINOR), currency: z.string().regex(/^[A-Z]{3}$/) }).strict().readonly();
export const TenantScopeSchema = z.object({ tenantId: IdSchema, hubId: IdSchema }).strict().readonly();
export type DeviceTime = z.infer<typeof DeviceTimeSchema>;
export type Coordinate = z.infer<typeof CoordinateSchema>;
export type Money = z.infer<typeof MoneySchema>;
export type TenantScope = z.infer<typeof TenantScopeSchema>;
