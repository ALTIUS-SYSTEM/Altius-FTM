import test from 'node:test';
import assert from 'node:assert/strict';
import { z } from 'zod';
import { apiResponseSchema, DateOnlySchema, DeviceEventSchema, DeviceTimeSchema, EtaEstimateSchema, EventBatchRequestSchema, EventReceiptSchema, GpsObservationSchema, LhsReportSchema, MoneySchema, TaskSchema, TimeZoneSchema, UtcTimestampSchema } from './index';

const time = { occurredAtUtc: '2026-09-07T01:00:00.000Z', utcOffsetMinutes: 420 };
const event = { schemaVersion: 1, eventId: 'evt-1', idempotencyKey: 'idem-1', tenantId: 'tenant-1', hubId: 'hub-1', actorId: 'driver-1', deviceId: 'phone-1', deviceSequence: 1, taskId: 'task-1', stopId: 'stop-1', expectedTaskRevision: 0, action: 'arrive', time, observationId: null, reason: null };
const expense = { id: 'expense-1', tenantId: 'tenant-1', hubId: 'hub-1', driverId: 'driver-1', revision: 0, time, category: 'fuel', money: { amountMinor: 15000, currency: 'IDR' }, description: 'Fuel', receiptId: null };
const report = { id: 'lhs-1', tenantId: 'tenant-1', hubId: 'hub-1', driverId: 'driver-1', day: '2026-09-07', timeZone: 'Asia/Jakarta', revision: 0, status: 'draft', visitedTaskIds: [], completedStopIds: [], expenses: [expense], totals: [expense.money], routeVersions: [], reviews: [] };

test('device time requires UTC plus bounded offset, rejects impossible dates', () => {
  assert.ok(DeviceTimeSchema.safeParse(time).success);
  for (const instant of ['2026-09-07T08:00:00+07:00', '2026-09-07T01:00:00', '2026-02-30T00:00:00Z']) assert.equal(UtcTimestampSchema.safeParse(instant).success, false);
  assert.equal(DeviceTimeSchema.safeParse({ ...time, utcOffsetMinutes: 841 }).success, false);
  assert.equal(DateOnlySchema.safeParse('2026-02-29').success, false);
  assert.ok(DateOnlySchema.safeParse('2024-02-29').success);
  assert.equal(TimeZoneSchema.safeParse('Invalid/Zone').success, false);
});
test('immutable JSON device events keep server receipt separate', () => {
  const parsed = DeviceEventSchema.parse(JSON.parse(JSON.stringify(event)));
  assert.ok(Object.isFrozen(parsed));
  assert.ok(Object.isFrozen(parsed.time));
  assert.equal(DeviceEventSchema.safeParse({ ...event, receivedAtUtc: time.occurredAtUtc }).success, false);
  assert.equal(DeviceEventSchema.safeParse({ ...event, action: 'skip' }).success, false);
  assert.equal(DeviceEventSchema.safeParse({ ...event, expectedTaskRevision: -1 }).success, false);
  assert.ok(EventReceiptSchema.safeParse({ eventId: 'evt-1', idempotencyKey: 'idem-1', receivedAtUtc: time.occurredAtUtc, status: 'accepted', taskRevision: 1, duplicate: false }).success);
});
test('batch validates tenant/hub isolation and duplicate idempotency keys', () => {
  assert.ok(EventBatchRequestSchema.safeParse({ tenantId: 'tenant-1', hubId: 'hub-1', events: [event] }).success);
  assert.equal(EventBatchRequestSchema.safeParse({ tenantId: 'other', hubId: 'hub-1', events: [event] }).success, false);
  assert.equal(EventBatchRequestSchema.safeParse({ tenantId: 'tenant-1', hubId: 'hub-1', events: [event, { ...event, eventId: 'evt-2' }] }).success, false);
});
test('tasks reject duplicate stops and inconsistent status', () => {
  const stop = { id: 'stop-1', sequence: 0, name: 'Depot', address: 'Demo road', location: { latitude: -6.2, longitude: 106.8 }, status: 'pending', serviceSeconds: 300 };
  const task = { id: 'task-1', tenantId: 'tenant-1', hubId: 'hub-1', title: 'Delivery', driverId: 'driver-1', flowId: null, status: 'assigned', revision: 0, scheduledAtUtc: time.occurredAtUtc, stops: [stop] };
  assert.ok(TaskSchema.safeParse(task).success);
  assert.equal(TaskSchema.safeParse({ ...task, stops: [stop, stop] }).success, false);
  assert.equal(TaskSchema.safeParse({ ...task, status: 'completed' }).success, false);
  assert.equal(TaskSchema.safeParse({ ...task, driverId: null }).success, false);
});
test('GPS missing fixes are explicit and coordinates/accuracy validated', () => {
  const gps = { id: 'gps-1', tenantId: 'tenant-1', hubId: 'hub-1', driverId: 'driver-1', vehicleId: null, deviceId: 'phone-1', time, source: 'app_gps', quality: 'unavailable', position: null, accuracyMeters: null, speedMetersPerSecond: null, mockLocationReported: null };
  assert.ok(GpsObservationSchema.safeParse(gps).success);
  assert.equal(GpsObservationSchema.safeParse({ ...gps, quality: 'accurate' }).success, false);
  assert.equal(GpsObservationSchema.safeParse({ ...gps, quality: 'accurate', position: { latitude: 91, longitude: 0 }, accuracyMeters: 5 }).success, false);
  assert.equal(GpsObservationSchema.safeParse({ ...gps, position: { latitude: 0, longitude: 0 } }).success, false);
});
test('ETA uncertainty interval must surround estimate after calculation', () => {
  const eta = { calculatedAtUtc: time.occurredAtUtc, earliestAtUtc: '2026-09-07T01:09:00Z', estimatedAtUtc: '2026-09-07T01:10:00Z', latestAtUtc: '2026-09-07T01:11:00Z', source: 'demo_graph', uncertaintySeconds: 60 };
  assert.ok(EtaEstimateSchema.safeParse(eta).success);
  assert.equal(EtaEstimateSchema.safeParse({ ...eta, uncertaintySeconds: 1 }).success, false);
  assert.equal(EtaEstimateSchema.safeParse({ ...eta, latestAtUtc: time.occurredAtUtc }).success, false);
});
test('safe integer money, currency totals and review revisions are enforced', () => {
  for (const amountMinor of [-1, 0.1, Infinity, Number.MAX_SAFE_INTEGER + 1]) assert.equal(MoneySchema.safeParse({ amountMinor, currency: 'IDR' }).success, false);
  assert.ok(LhsReportSchema.safeParse(report).success);
  assert.equal(LhsReportSchema.safeParse({ ...report, totals: [] }).success, false);
  assert.equal(LhsReportSchema.safeParse({ ...report, expenses: [expense, expense] }).success, false);
  assert.equal(LhsReportSchema.safeParse({ ...report, status: 'approved' }).success, false);
  const reviews = [{ revision: 1, action: 'submit', actorId: 'driver-1', atUtc: time.occurredAtUtc, note: null }, { revision: 2, action: 'request_revision', actorId: 'lead-1', atUtc: time.occurredAtUtc, note: 'Attach receipt' }, { revision: 3, action: 'resubmit', actorId: 'driver-1', atUtc: time.occurredAtUtc, note: null }, { revision: 4, action: 'approve', actorId: 'lead-1', atUtc: time.occurredAtUtc, note: null }];
  assert.ok(LhsReportSchema.safeParse({ ...report, revision: 4, status: 'approved', reviews }).success);
  assert.equal(LhsReportSchema.safeParse({ ...report, revision: 3, status: 'approved', reviews }).success, false);
});
test('REST envelopes require demo/live metadata and a single success/error branch', () => {
  const schema = apiResponseSchema(z.string());
  const value = { ok: true, data: 'hello', meta: { requestId: 'req-1', serverTimeUtc: time.occurredAtUtc, mode: 'demo' } };
  assert.ok(schema.safeParse(value).success);
  assert.equal(schema.safeParse({ ...value, error: { code: 'unavailable', message: 'Unavailable', retryable: true } }).success, false);
  assert.equal(schema.safeParse({ ok: true, data: 'hello' }).success, false);
});
