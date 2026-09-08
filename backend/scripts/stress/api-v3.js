/**
 * Altius FTM — HTTP API v3 stress / load harness (k6).
 *
 * Targets local compose by default (`http://127.0.0.1:8080`).
 * Never point this at production without an explicit base URL and a
 * pre-minted token; default write scenarios are off.
 *
 * Env (all optional unless noted):
 *   ALTIUS_STRESS_BASE_URL   API origin (default http://127.0.0.1:8080)
 *   ALTIUS_STRESS_TOKEN      Bearer JWT (skips login)
 *   ALTIUS_STRESS_USER       Password-grant username (local admin/changeme)
 *   ALTIUS_STRESS_PASSWORD   Password-grant password
 *   ALTIUS_STRESS_VUS        Concurrent VUs for read scenario (default 10)
 *   ALTIUS_STRESS_DURATION   Read scenario duration (default 30s)
 *   ALTIUS_STRESS_HEALTH_RPS Target RPS for unauthenticated health burst (default 50)
 *   ALTIUS_STRESS_HEALTH_DUR Health burst duration (default 15s)
 *   ALTIUS_STRESS_WRITES     "1" to enable local-only task-create (default off)
 *   ALTIUS_STRESS_SCENARIO   all | health | reads | writes (default all)
 */

import http from "k6/http";
import { check, group, sleep } from "k6";
import { Counter, Rate, Trend } from "k6/metrics";
import { textSummary } from "https://jslib.k6.io/k6-summary/0.0.4/index.js";

const BASE = (__ENV.ALTIUS_STRESS_BASE_URL || "http://127.0.0.1:8080").replace(
  /\/$/,
  "",
);
const API = `${BASE}/api/v3`;
const TOKEN = (__ENV.ALTIUS_STRESS_TOKEN || "").trim();
const USER = (__ENV.ALTIUS_STRESS_USER || "").trim();
const PASS = (__ENV.ALTIUS_STRESS_PASSWORD || "").trim();
const VUS = Number(__ENV.ALTIUS_STRESS_VUS || 10);
const DURATION = __ENV.ALTIUS_STRESS_DURATION || "30s";
const HEALTH_RPS = Number(__ENV.ALTIUS_STRESS_HEALTH_RPS || 50);
const HEALTH_DUR = __ENV.ALTIUS_STRESS_HEALTH_DUR || "15s";
const WRITES = __ENV.ALTIUS_STRESS_WRITES === "1";
const SCENARIO = (__ENV.ALTIUS_STRESS_SCENARIO || "all").toLowerCase();

const statusBreakdown = new Counter("altius_http_status");
const authFailures = new Counter("altius_auth_failures");
const errorRate = new Rate("altius_errors");
const latency = new Trend("altius_latency_ms", true);

function want(name) {
  if (SCENARIO === "all") return true;
  return SCENARIO === name;
}

/** Parse k6 duration strings like `30s`, `1m`, `90` (seconds). */
function durationSeconds(raw) {
  const s = String(raw || "0").trim();
  const m = /^(\d+(?:\.\d+)?)(ms|s|m|h)?$/i.exec(s);
  if (!m) return 0;
  const n = Number(m[1]);
  const unit = (m[2] || "s").toLowerCase();
  if (unit === "ms") return n / 1000;
  if (unit === "m") return n * 60;
  if (unit === "h") return n * 3600;
  return n;
}

function scenarios() {
  const out = {};
  let offsetSec = 0;

  if (want("health")) {
    out.health_burst = {
      executor: "constant-arrival-rate",
      rate: HEALTH_RPS,
      timeUnit: "1s",
      duration: HEALTH_DUR,
      preAllocatedVUs: Math.min(Math.max(HEALTH_RPS, 5), 50),
      maxVUs: Math.min(HEALTH_RPS * 2, 100),
      exec: "healthBurst",
      tags: { scenario: "health" },
    };
    offsetSec += durationSeconds(HEALTH_DUR);
  }

  if (want("reads")) {
    out.authenticated_reads = {
      executor: "constant-vus",
      vus: VUS,
      duration: DURATION,
      exec: "authenticatedReads",
      startTime: `${offsetSec}s`,
      tags: { scenario: "reads" },
    };
    offsetSec += durationSeconds(DURATION);
  }

  if (want("writes") && WRITES) {
    out.local_writes = {
      executor: "constant-vus",
      vus: Math.min(VUS, 5),
      duration: "20s",
      exec: "localWrites",
      startTime: `${offsetSec}s`,
      tags: { scenario: "writes" },
    };
  }

  return out;
}

export const options = {
  scenarios: scenarios(),
  thresholds: {
    http_req_failed: ["rate<0.05"],
    http_req_duration: ["p(95)<2000", "p(99)<5000"],
    altius_errors: ["rate<0.05"],
  },
  summaryTrendStats: ["avg", "min", "med", "p(90)", "p(95)", "p(99)", "max"],
};

function record(res) {
  const code = String(res.status || 0);
  statusBreakdown.add(1, { status: code });
  latency.add(res.timings.duration);
  const failed = res.status < 200 || res.status >= 400;
  errorRate.add(failed);
  return !failed;
}

function authHeaders(token) {
  return {
    Authorization: `Bearer ${token}`,
    Accept: "application/json",
  };
}

function loginViaApi() {
  if (!USER || !PASS) return "";
  const res = http.post(
    `${API}/auth/login`,
    JSON.stringify({ username: USER, password: PASS }),
    { headers: { "Content-Type": "application/json", Accept: "application/json" } },
  );
  if (res.status !== 200) {
    console.warn(
      `password grant via API failed: status=${res.status} body=${String(res.body).slice(0, 200)}`,
    );
    authFailures.add(1);
    return "";
  }
  let body;
  try {
    body = res.json();
  } catch (_) {
    authFailures.add(1);
    return "";
  }
  return body.access_token || body.accessToken || "";
}

export function setup() {
  const health = http.get(`${API}/health`);
  const ready = http.get(`${API}/ready`);
  if (health.status !== 200 || ready.status !== 200) {
    throw new Error(
      `API not ready at ${API} (health=${health.status} ready=${ready.status}). ` +
        `Start compose api + postgres, or set ALTIUS_STRESS_BASE_URL.`,
    );
  }

  let token = TOKEN;
  if (!token && (want("reads") || (want("writes") && WRITES))) {
    token = loginViaApi();
    if (!token) {
      console.warn(
        "No ALTIUS_STRESS_TOKEN and password grant failed/disabled. " +
          "Authenticated scenarios will record 401s. Mint a local JWT or enable " +
          "ALLOW_PASSWORD_GRANT + Keycloak direct access grants for stress only.",
      );
    }
  }

  let taskIds = [];
  if (token) {
    const list = http.get(`${API}/tasks`, { headers: authHeaders(token) });
    if (list.status === 200) {
      try {
        const data = list.json().data || [];
        taskIds = data
          .map((row) => {
            if (typeof row === "string") return row;
            if (row && row.id) return row.id;
            if (row && row.task && row.task.id) return row.task.id;
            return null;
          })
          .filter(Boolean)
          .slice(0, 20);
      } catch (_) {
        /* ignore parse errors; reads still exercise list */
      }
    }
  }

  return {
    token,
    taskIds,
    base: API,
    writes: WRITES,
  };
}

export function healthBurst() {
  group("unauthenticated probes", () => {
    const h = http.get(`${API}/health`);
    record(h);
    check(h, { "health 200": (r) => r.status === 200 });

    const r = http.get(`${API}/ready`);
    record(r);
    check(r, { "ready 200": (r) => r.status === 200 });
  });
}

export function authenticatedReads(data) {
  const token = data.token;
  if (!token) {
    authFailures.add(1);
    errorRate.add(true);
    sleep(0.2);
    return;
  }
  const headers = authHeaders(token);

  group("authenticated GETs", () => {
    const me = http.get(`${API}/auth/me`, { headers });
    record(me);
    check(me, { "auth/me 200": (r) => r.status === 200 });

    const tasks = http.get(`${API}/tasks`, { headers });
    record(tasks);
    check(tasks, { "tasks 200": (r) => r.status === 200 });

    const hubs = http.get(`${API}/hubs`, { headers });
    record(hubs);
    check(hubs, { "hubs 200": (r) => r.status === 200 });

    const drivers = http.get(`${API}/drivers`, { headers });
    record(drivers);
    check(drivers, { "drivers 200": (r) => r.status === 200 });

    const ids = data.taskIds || [];
    if (ids.length > 0) {
      const id = ids[Math.floor(Math.random() * ids.length)];
      const one = http.get(`${API}/task/${encodeURIComponent(id)}`, { headers });
      record(one);
      check(one, {
        "task/{id} 200|404": (r) => r.status === 200 || r.status === 404,
      });
    }
  });

  sleep(0.05);
}

export function localWrites(data) {
  if (!data.writes) return;
  const token = data.token;
  if (!token) {
    authFailures.add(1);
    return;
  }

  const id = `stress-${__VU}-${__ITER}-${Date.now()}`;
  const now = new Date().toISOString();
  const body = {
    id,
    tenant_id: "altius",
    hub_id: "jakarta",
    title: `k6 stress ${id}`,
    status: "assigned",
    assignee_id: null,
    created_at: now,
    stops: [
      {
        id: `${id}-s0`,
        sequence: 0,
        name: "Stress stop",
        address: "Jakarta (k6 local only)",
        location: { lat: -6.2, lng: 106.8 },
        status: "pending",
        service_seconds: 60,
      },
    ],
  };

  group("local task-create", () => {
    const res = http.post(`${API}/task-create`, JSON.stringify(body), {
      headers: {
        ...authHeaders(token),
        "Content-Type": "application/json",
      },
    });
    record(res);
    check(res, {
      "task-create 200|403": (r) => r.status === 200 || r.status === 403,
    });
  });

  sleep(0.1);
}

export function handleSummary(data) {
  const path = __ENV.ALTIUS_STRESS_SUMMARY_JSON || "";
  const out = {
    stdout: textSummary(data, { indent: " ", enableColors: true }),
  };
  if (path) {
    out[path] = JSON.stringify(data, null, 2);
  }
  return out;
}
