import { test } from "node:test";
import assert from "node:assert/strict";
import { haversineMeters, dijkstra, etaMinutes, distanceToCorridorMeters, evaluateCorridor, compareGpsStreams, aggregateDaily } from "./index.ts";

const KM_PER_DEGREE_METERS = 111_000;
const HALF_KM_TOLERANCE = 500;
const THIRTY_SECONDS_MS = 30_000;
const FIVE_SECONDS_MS = 5_000;
const TEN_SECONDS_MS = 10_000;
const FAR_FUTURE_MS = 999_999;

test("haversine ~111km per degree lat", () => {
  const d = haversineMeters({ lat: 0, lng: 0 }, { lat: 1, lng: 0 });
  assert.ok(d > KM_PER_DEGREE_METERS - HALF_KM_TOLERANCE && d < KM_PER_DEGREE_METERS + HALF_KM_TOLERANCE);
});
test("dijkstra shortest path and unreachable", () => {
  const g = { A: [{ to: "B", weight: 2 }, { to: "C", weight: 5 }], B: [{ to: "C", weight: 1 }], C: [], D: [] };
  assert.deepEqual(dijkstra(g, "A", "C"), { path: ["A", "B", "C"], cost: 3 });
  assert.equal(dijkstra(g, "A", "D"), null);
  assert.equal(dijkstra(g, "X", "C"), null);
  assert.throws(() => dijkstra({ A: [{ to: "B", weight: -1 }] }, "A", "B"));
});
test("eta validation", () => {
  assert.equal(etaMinutes(10000, 50), 12);
  assert.equal(etaMinutes(100, 0), null);
});
test("corridor distance and hysteresis", () => {
  const corridor = [{ lat: 0, lng: 0 }, { lat: 0, lng: 0.01 }];
  const near = { lat: 0.0002, lng: 0.005, at: 1, accuracyMeters: 5 };
  const far = { lat: 0.005, lng: 0.005, at: 2, accuracyMeters: 5 };
  assert.ok(distanceToCorridorMeters(near, corridor) < 30);
  assert.ok(distanceToCorridorMeters(far, corridor) > 400);
  assert.equal(evaluateCorridor([near, far], corridor, { radiusMeters: 100, consecutiveBreach: 2, maxAccuracyMeters: 50 }).offRoute, false);
  assert.equal(evaluateCorridor([far, { ...far, at: 3 }], corridor, { radiusMeters: 100, consecutiveBreach: 2, maxAccuracyMeters: 50 }).offRoute, true);
  assert.equal(evaluateCorridor([{ ...far, accuracyMeters: 500 }], corridor, { radiusMeters: 100, consecutiveBreach: 1, maxAccuracyMeters: 50 }).reason, "inaccurate");
  assert.equal(evaluateCorridor([near], [{ lat: 0, lng: 0 }], { radiusMeters: 100, consecutiveBreach: 1, maxAccuracyMeters: 50 }).reason, "insufficient-corridor");
});
test("gps stream comparison", () => {
  const t0 = Date.now();
  const app = [{ lat: 0, lng: 0, at: t0 }, { lat: 0.001, lng: 0.001, at: t0 + THIRTY_SECONDS_MS }];
  const same = compareGpsStreams({ app, vehicle: app.map(s => ({ ...s, at: s.at + FIVE_SECONDS_MS })) }, { maxTimeGapMs: TEN_SECONDS_MS, thresholdMeters: 150 });
  assert.equal(same.flag, "none");
  const diverged = compareGpsStreams({ app, vehicle: [{ lat: 0.01, lng: 0.01, at: t0 }] }, { maxTimeGapMs: 10000, thresholdMeters: 150 });
  assert.equal(diverged.flag, "review");
  const missing = compareGpsStreams({ app, vehicle: [{ lat: 0, lng: 0, at: t0 + FAR_FUTURE_MS }] }, { maxTimeGapMs: TEN_SECONDS_MS, thresholdMeters: 150 });
  assert.equal(missing.flag, "review");
  assert.equal(compareGpsStreams({ app: [], vehicle: [] }, { maxTimeGapMs: TEN_SECONDS_MS, thresholdMeters: 150 }).flag, "insufficient-data");
  // One unusable coordinate must not clear the batch: NaN is sticky through
  // Math.max and false in every comparison, so it would otherwise read "none".
  const poisoned = compareGpsStreams(
    { app: [{ lat: Number.NaN, lng: 0, at: t0 }], vehicle: [{ lat: 0, lng: 0, at: t0 }] },
    { maxTimeGapMs: TEN_SECONDS_MS, thresholdMeters: 150 },
  );
  assert.equal(poisoned.flag, "review");
  // An unmeasurable sample must not reset the corridor breach streak.
  const corridor = [{ lat: 0, lng: 0 }, { lat: 0, lng: 0.01 }];
  const off = { lat: 0.005, lng: 0.005, at: 1, accuracyMeters: 5 };
  assert.equal(
    evaluateCorridor([off, { lat: Number.NaN, lng: 0, at: 2 }, { ...off, at: 3 }], corridor,
      { radiusMeters: 100, consecutiveBreach: 2, maxAccuracyMeters: 50 }).offRoute,
    true,
  );
});
test("daily aggregation", () => {
  const r = aggregateDaily(
    [{ kind: "arrived", entity: "t1", day: "2026-09-07" }, { kind: "done", entity: "t1", day: "2026-09-07" }, { kind: "arrived", entity: "t2", day: "2026-09-07" }],
    [{ amount: 50000, day: "2026-09-07" }, { amount: 1000, day: "2026-09-06" }], "2026-09-07");
  assert.deepEqual(r, { completedStops: 1, visitedStops: 2, totalCost: 50000 });
});
