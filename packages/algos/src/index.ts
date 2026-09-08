export interface GeoPoint { lat: number; lng: number }
export interface WeightedEdge { to: string; weight: number }
export type DemoGraph = Record<string, readonly WeightedEdge[]>;

/**
 * Great-circle distance in metres. Returns `Infinity` for any non-finite
 * input rather than NaN: these distances feed fraud controls, and NaN is
 * sticky through `Math.max` and false in every comparison, so one unusable
 * coordinate would silently clear an entire batch.
 */
export const haversineMeters = (a: GeoPoint, b: GeoPoint): number => {
  if (![a?.lat, a?.lng, b?.lat, b?.lng].every(Number.isFinite)) return Infinity;
  const rad = Math.PI / 180;
  const dLat = (b.lat - a.lat) * rad, dLng = (b.lng - a.lng) * rad;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(dLng / 2) ** 2;
  return 6371000 * 2 * Math.asin(Math.min(1, Math.sqrt(h)));
};

export function dijkstra(graph: DemoGraph, start: string, goal: string): { path: string[]; cost: number } | null {
  // Node names are client-supplied stop ids. Bracket access on a plain object
  // reaches Object.prototype, so a node called "constructor" or "toString"
  // resolves to an inherited function: the existence check passes and the
  // adjacency loop then throws on a non-iterable.
  const adj = new Map(Object.entries(graph).filter(([, e]) => Array.isArray(e)));
  // A NaN weight passes `< 0` and then loses every comparison, silently
  // removing the edge from consideration instead of failing. Validate the whole
  // graph once rather than only the edges reached before the goal.
  for (const edges of adj.values()) {
    for (const e of edges) {
      if (e.weight < 0) throw new Error("Negative edge weights are unsupported");
      // NaN/Infinity pass `< 0` and then lose every comparison, silently
      // dropping the edge instead of failing. No route is the honest answer.
      if (!Number.isFinite(e.weight)) return null;
    }
  }
  if (!adj.has(start)) return null;
  const dist = new Map<string, number>([[start, 0]]);
  const prev = new Map<string, string>();
  const open = new Set(adj.keys());
  for (const edges of adj.values()) for (const e of edges) if (!dist.has(e.to)) { open.add(e.to); dist.set(e.to, Infinity); }
  while (open.size) {
    let node = "", best = Infinity;
    for (const n of open) if ((dist.get(n) ?? Infinity) < best) { best = dist.get(n)!; node = n; }
    if (!node || node === goal) break;
    open.delete(node);
    for (const e of adj.get(node) ?? []) {
      const alt = best + e.weight;
      if (alt < (dist.get(e.to) ?? Infinity)) { dist.set(e.to, alt); prev.set(e.to, node); }
    }
  }
  if (!dist.has(goal) || dist.get(goal) === Infinity) return null;
  const path = [goal];
  while (path[0] !== start) { const head = path[0]; if (!head) return null; const p = prev.get(head); if (!p) return null; path.unshift(p); }
  return { path, cost: dist.get(goal)! };
}

const METERS_PER_KILOMETER = 1000;
const MINUTES_PER_HOUR = 60;

export const etaMinutes = (distanceMeters: number, speedKmh: number): number | null =>
  speedKmh <= 0 || distanceMeters < 0 ? null : (distanceMeters / METERS_PER_KILOMETER) / speedKmh * MINUTES_PER_HOUR;

const toXY = (origin: GeoPoint, p: GeoPoint) => ({ x: haversineMeters(origin, { lat: origin.lat, lng: p.lng }), y: haversineMeters(origin, { lat: p.lat, lng: origin.lng }) });

export function distanceToCorridorMeters(point: GeoPoint, corridor: readonly GeoPoint[]): number {
  const origin = corridor[0];
  if (!origin) return Infinity;
  if (corridor.length === 1) return haversineMeters(point, origin);
  const p = toXY(origin, point);
  let min = Infinity;
  for (let i = 0; i < corridor.length - 1; i++) {
    const a = toXY(origin, corridor[i]!), b = toXY(origin, corridor[i + 1]!);
    const dx = b.x - a.x, dy = b.y - a.y;
    const len2 = dx * dx + dy * dy;
    const t = len2 === 0 ? 0 : Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2));
    min = Math.min(min, Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy)));
  }
  return min;
}

export interface GeoSample extends GeoPoint { at: number; accuracyMeters?: number }
export interface CorridorResult { offRoute: boolean; distanceMeters: number; reason: "inside" | "outside" | "inaccurate" | "insufficient-corridor" }

export function evaluateCorridor(samples: readonly GeoSample[], corridor: readonly GeoPoint[], opts: { radiusMeters: number; consecutiveBreach: number; maxAccuracyMeters: number }): CorridorResult {
  if (corridor.length < 2) return { offRoute: false, distanceMeters: 0, reason: "insufficient-corridor" };
  let streak = 0, last = 0, evaluated = 0;
  for (const s of samples) {
    if (s.accuracyMeters !== undefined && s.accuracyMeters > opts.maxAccuracyMeters) continue;
    const d = distanceToCorridorMeters(s, corridor);
    // An unmeasurable sample must not reset the breach streak — otherwise one
    // position-less reading interleaved between breaches hides the detour.
    if (!Number.isFinite(d)) continue;
    evaluated++;
    last = d;
    streak = last > opts.radiusMeters ? streak + 1 : 0;
    if (streak >= opts.consecutiveBreach) return { offRoute: true, distanceMeters: last, reason: "outside" };
  }
  // "inside" is an affirmative claim. With nothing measurable — an empty batch,
  // or every sample self-reporting an accuracy above the limit — we have no
  // evidence either way, and saying "inside" would let a device opt out of the
  // check by inflating accuracyMeters.
  if (evaluated === 0) return { offRoute: false, distanceMeters: 0, reason: "inaccurate" };
  return { offRoute: false, distanceMeters: last, reason: "inside" };
}

export interface StreamPair { app: readonly GeoSample[]; vehicle: readonly GeoSample[] }
export interface AnomalyResult { varianceMeters: number; matched: number; flag: "none" | "review" | "insufficient-data" }

export function compareGpsStreams({ app, vehicle }: StreamPair, opts: { maxTimeGapMs: number; thresholdMeters: number }): AnomalyResult {
  let matched = 0, worst = 0, unmeasurable = 0;
  // Sort once and advance a single pointer: the nested scan was O(n*m) over two
  // arrays whose size the uploading device chooses, so one accepted upload —
  // not a flood — determined the cost.
  const byTime = [...vehicle].filter(v => Number.isFinite(v.at)).sort((x, y) => x.at - y.at);
  let cursor = 0;
  for (const a of [...app].sort((x, y) => x.at - y.at)) {
    while (cursor + 1 < byTime.length && Math.abs(byTime[cursor + 1]!.at - a.at) <= Math.abs(byTime[cursor]!.at - a.at)) cursor++;
    const best: GeoSample | null = byTime[cursor] ?? null;
    const bestDelta = best ? Math.abs(a.at - best.at) : Infinity;
    if (best && bestDelta <= opts.maxTimeGapMs) {
      matched++;
      const d = haversineMeters(a, best);
      if (Number.isFinite(d)) worst = Math.max(worst, d); else unmeasurable++;
    }
  }
  // Only a genuinely empty comparison is "insufficient-data". If the app
  // reported positions and the vehicle corroborated none of them, that is the
  // signal — a spoofing device would otherwise defeat the check simply by
  // withholding the vehicle stream or time-shifting it past the match window.
  if (app.length === 0) return { varianceMeters: 0, matched: 0, flag: "insufficient-data" };
  if (matched === 0) return { varianceMeters: 0, matched: 0, flag: "review" };
  if (unmeasurable > 0) return { varianceMeters: Math.round(worst), matched, flag: "review" };
  return { varianceMeters: Math.round(worst), matched, flag: worst > opts.thresholdMeters ? "review" : "none" };
}

export function aggregateDaily(events: readonly { kind: string; entity: string; day: string }[], costs: readonly { amount: number; day: string }[], day: string) {
  const today = events.filter(e => e.day === day);
  return {
    completedStops: new Set(today.filter(e => e.kind === "done").map(e => e.entity)).size,
    visitedStops: new Set(today.filter(e => e.kind === "arrived").map(e => e.entity)).size,
    // Amounts are integer minor units. A non-finite or negative entry used to
    // poison or quietly reduce the whole day's total; count it instead so a
    // non-zero residual can be reconciled rather than silently absorbed.
    ...sumCosts(costs, day),
  };
}

function sumCosts(costs: readonly { amount: number; day: string }[], day: string) {
  let totalCost = 0, rejectedCosts = 0;
  for (const c of costs) {
    if (c.day !== day) continue;
    if (!Number.isSafeInteger(c.amount) || c.amount < 0) { rejectedCosts++; continue; }
    totalCost += c.amount;
  }
  return { totalCost, rejectedCosts };
}
