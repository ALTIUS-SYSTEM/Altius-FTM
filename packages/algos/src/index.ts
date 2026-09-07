export interface GeoPoint { lat: number; lng: number }
export interface WeightedEdge { to: string; weight: number }
export type DemoGraph = Record<string, readonly WeightedEdge[]>;

export const haversineMeters = (a: GeoPoint, b: GeoPoint): number => {
  const rad = Math.PI / 180;
  const dLat = (b.lat - a.lat) * rad, dLng = (b.lng - a.lng) * rad;
  const h = Math.sin(dLat / 2) ** 2 + Math.cos(a.lat * rad) * Math.cos(b.lat * rad) * Math.sin(dLng / 2) ** 2;
  return 6371000 * 2 * Math.asin(Math.min(1, Math.sqrt(h)));
};

export function dijkstra(graph: DemoGraph, start: string, goal: string): { path: string[]; cost: number } | null {
  if (!graph[start]) return null;
  const dist = new Map<string, number>([[start, 0]]);
  const prev = new Map<string, string>();
  const open = new Set(Object.keys(graph));
  for (const edges of Object.values(graph)) for (const e of edges) if (!dist.has(e.to)) { open.add(e.to); dist.set(e.to, Infinity); }
  while (open.size) {
    let node = "", best = Infinity;
    for (const n of open) if ((dist.get(n) ?? Infinity) < best) { best = dist.get(n)!; node = n; }
    if (!node || node === goal) break;
    open.delete(node);
    for (const e of graph[node] ?? []) {
      if (e.weight < 0) throw new Error("Negative edge weights are unsupported");
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
  let streak = 0, last = 0;
  for (const s of samples) {
    if (s.accuracyMeters !== undefined && s.accuracyMeters > opts.maxAccuracyMeters) continue;
    last = distanceToCorridorMeters(s, corridor);
    streak = last > opts.radiusMeters ? streak + 1 : 0;
    if (streak >= opts.consecutiveBreach) return { offRoute: true, distanceMeters: last, reason: "outside" };
  }
  return { offRoute: false, distanceMeters: last, reason: "inside" };
}

export interface StreamPair { app: readonly GeoSample[]; vehicle: readonly GeoSample[] }
export interface AnomalyResult { varianceMeters: number; matched: number; flag: "none" | "review" | "insufficient-data" }

export function compareGpsStreams({ app, vehicle }: StreamPair, opts: { maxTimeGapMs: number; thresholdMeters: number }): AnomalyResult {
  let matched = 0, worst = 0;
  for (const a of app) {
    let bestDelta = Infinity, best: GeoSample | null = null;
    for (const v of vehicle) { const d = Math.abs(a.at - v.at); if (d < bestDelta) { bestDelta = d; best = v; } }
    if (best && bestDelta <= opts.maxTimeGapMs) { matched++; worst = Math.max(worst, haversineMeters(a, best)); }
  }
  if (matched === 0) return { varianceMeters: 0, matched: 0, flag: "insufficient-data" };
  return { varianceMeters: Math.round(worst), matched, flag: worst > opts.thresholdMeters ? "review" : "none" };
}

export function aggregateDaily(events: readonly { kind: string; entity: string; day: string }[], costs: readonly { amount: number; day: string }[], day: string) {
  const today = events.filter(e => e.day === day);
  return {
    completedStops: new Set(today.filter(e => e.kind === "done").map(e => e.entity)).size,
    visitedStops: new Set(today.filter(e => e.kind === "arrived").map(e => e.entity)).size,
    totalCost: costs.filter(c => c.day === day).reduce((sum, c) => sum + c.amount, 0),
  };
}
