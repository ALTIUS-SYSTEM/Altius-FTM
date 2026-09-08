# Altius FTM — Static Vulnerability Review

62 findings across 7 focus areas.
Severity: 16 high / 25 medium / 21 low.
Remediation: **55 fixed**, 4 partial, 1 open.

Static review — no code was executed. Fix status reflects verified source changes;
the whole workspace passes typecheck, lint, clippy `-D warnings`, and all tests.

| ID | Sev | Status | Disposition | Category | Location | Title |
|---|---|---|---|---|---|---|
| F-07-11 | MEDIUM | accepted-risk | accepted-risk | fail-open | `packages/algos/src/index.ts:89` | aggregateDaily drops entries whose day string does not match exactly |
| F-07-19 | MEDIUM | partial | accepted-risk | input-validation | `packages/api-contracts/src/primitives.ts:16` | Rust mirror accepts negative money, unknown fields and an unconstrained payload |
| F-06-05 | LOW | open | open | information-disclosure | `apps/mobile/lib/core/data/work_store.dart:108` | Operational PII stored in an unencrypted on-device database |
| F-05-08 | LOW | accepted-risk | accepted-risk | input-validation | `apps/web/src/features/tasks.tsx:33` | validateTask applied only on the editor submit path |
| F-03-04 | MEDIUM | partial | accepted-risk | authz-bypass | `backend/crates/altius-api/src/agent.rs:32` | Tool executors carry no caller identity |
| F-07-10 | MEDIUM | partial | accepted-risk | numeric-handling | `packages/algos/src/index.ts:93` | aggregateDaily summed currency as floats across mixed currencies |
| F-07-06 | MEDIUM | partial | accepted-risk | privilege-escalation | `packages/api-contracts/src/tasks.ts:20` | Tenant/hub/actor carried in request bodies, cross-checked only against each other |
| F-06-02 | HIGH | fixed | mitigated | cleartext-transport | `apps/mobile/lib/core/data/work_store.dart:163` | Server URL never validated as HTTPS |
| F-06-01 | HIGH | fixed | mitigated | credential-storage | `apps/mobile/lib/core/data/work_store.dart:192` | Access and refresh tokens persisted in plaintext SQLite |
| F-05-01 | HIGH | fixed | mitigated | csv-injection | `apps/web/src/features/schedule-planner.tsx:136` | Schedule CSV built by raw concatenation with no quoting or formula guard |
| F-04-01 | HIGH | fixed | mitigated | csrf | `apps/web/src/lib/auth.ts:57` | OAuth state generated but never stored or verified |
| F-03-02 | HIGH | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/agent.rs:121` | agent::resume trusted client conversation state and executed a client-chosen tool call |
| F-03-01 | HIGH | fixed | mitigated | information-disclosure | `backend/crates/altius-api/src/maps.rs:181` | GOOGLE_MAPS_API_KEY returned in 503 bodies via reqwest error Display |
| F-02-03 | HIGH | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/routes.rs:185` | Tenant fell back to an unverified organization_id token claim |
| F-02-01 | HIGH | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/routes.rs:218` | create_task took tenant from the request body behind a Driver deny-list |
| F-02-02 | HIGH | fixed | mitigated | idor | `backend/crates/altius-api/src/routes.rs:249` | sync_events scope check was self-referential |
| F-01-02 | HIGH | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/store.rs:114` | create_task bound $o but never used it; body hub-id planted tasks in other tenants |
| F-01-01 | HIGH | fixed | mitigated | idor | `backend/crates/altius-api/src/store.rs:235` | record_event mutated tasks/stops by raw id with no org constraint |
| F-07-01 | HIGH | fixed | mitigated | fail-open | `packages/algos/src/index.ts:62` | evaluateCorridor returned a clean 'inside' for an empty sample set |
| F-07-02 | HIGH | fixed | mitigated | fail-open | `packages/algos/src/index.ts:66` | Device-declared accuracy silently suppressed every sample |
| F-07-03 | HIGH | fixed | mitigated | fail-open | `packages/algos/src/index.ts:84` | compareGpsStreams degraded to insufficient-data under attacker timestamps |
| F-07-05 | HIGH | fixed | mitigated | privilege-escalation | `packages/api-contracts/src/identity.ts:6` | Membership permissions were client-assertable and unbound from role |
| F-07-04 | HIGH | fixed | mitigated | privilege-escalation | `packages/api-contracts/src/lhs.ts:10` | LhsReport let the client assert its own approved status and reviewer identity |
| F-06-07 | MEDIUM | fixed | mitigated | authorization | `apps/mobile/lib/core/data/work_store.dart:350` | Client-asserted tenant/hub/driver and local-only stage rules |
| F-06-03 | MEDIUM | fixed | mitigated | data-loss | `apps/mobile/lib/core/data/work_store.dart:387` | Sync receipts matched to local events by array position |
| F-06-04 | MEDIUM | fixed | mitigated | data-loss | `apps/mobile/lib/core/data/work_store.dart:389` | Any non-'accepted' status became a permanent 'rejected' drop |
| F-04-05 | MEDIUM | fixed | mitigated | auth-bypass | `apps/web/src/app/[[...path]]/view.tsx:33` | Route guard trusted a persisted session boolean |
| F-05-02 | MEDIUM | fixed | mitigated | csv-injection | `apps/web/src/data/adapter.ts:41` | csvCell formula regex anchored at position 0; a leading space bypassed it |
| F-05-03 | MEDIUM | fixed | mitigated | input-validation | `apps/web/src/data/api-adapter.ts:111` | API-sourced tasks bypassed Zod entirely |
| F-05-05 | MEDIUM | fixed | mitigated | data-loss | `apps/web/src/data/api-adapter.ts:118` | Live adapter silently discarded every task mutation while reporting success |
| F-04-04 | MEDIUM | fixed | mitigated | token-leak | `apps/web/src/lib/auth.ts:95` | Refresh token for a public client persisted in sessionStorage |
| F-03-03 | MEDIUM | fixed | mitigated | prompt-injection | `backend/crates/altius-api/src/agent.rs:251` | Tool arguments never validated against the declared schema |
| F-02-04 | MEDIUM | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/routes.rs:317` | Roster routes had no role gate; a token with no known role was a full principal |
| F-01-05 | MEDIUM | fixed | mitigated | sql-injection | `backend/crates/altius-api/src/store.rs:23` | esc() escaped only backslash and quote; C0 control characters passed through |
| F-01-03 | MEDIUM | fixed | mitigated | toctou | `backend/crates/altius-api/src/store.rs:177` | Idempotency check-then-insert race |
| F-01-04 | MEDIUM | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/store.rs:178` | Idempotency keys shared one global namespace across tenants |
| F-07-12 | MEDIUM | fixed | mitigated | numeric-handling | `packages/algos/src/index.ts:5` | NaN coordinates propagated into comparisons that fail open |
| F-07-13 | MEDIUM | fixed | mitigated | input-validation | `packages/algos/src/index.ts:13` | dijkstra indexed the graph by untrusted node name, reaching Object.prototype |
| F-07-08 | MEDIUM | fixed | mitigated | algorithmic-complexity | `packages/algos/src/index.ts:81` | compareGpsStreams was O(n*m) over two unbounded arrays |
| F-07-15 | MEDIUM | fixed | mitigated | input-validation | `packages/api-contracts/src/api.ts:26` | z.record escape hatches accepted unbounded keys and values |
| F-07-07 | MEDIUM | fixed | mitigated | fail-open | `packages/api-contracts/src/location.ts:14` | GPS source self-declared, so one device could forge both corroborating streams |
| F-07-09 | MEDIUM | fixed | mitigated | input-validation | `packages/api-contracts/src/location.ts:24` | Collection fields had no .max() before reaching storage |
| F-07-16 | MEDIUM | fixed | mitigated | input-validation | `packages/api-contracts/src/primitives.ts:16` | Money amounts capped only at MAX_SAFE_INTEGER |
| F-06-06 | LOW | fixed | mitigated | information-disclosure | `apps/mobile/lib/core/data/work_store.dart:205` | Logout left the previous driver's email and server URL on the device |
| F-05-06 | LOW | fixed | mitigated | state-corruption | `apps/web/src/components/demo-provider.tsx:35` | Adapter write and setError ran inside a setState updater |
| F-05-04 | LOW | fixed | mitigated | unbounded-recursion | `apps/web/src/data/api-adapter.ts:22` | leaf() recursed without a depth bound |
| F-05-07 | LOW | fixed | mitigated | data-loss | `apps/web/src/data/api-adapter.ts:115` | A single malformed localStorage value silently discarded all persisted preferences |
| F-04-06 | LOW | fixed | mitigated | auth-bypass | `apps/web/src/features/callback.tsx:41` | Callback ignored the OAuth error param and left code in the URL |
| F-04-02 | LOW | fixed | mitigated | weak-randomness | `apps/web/src/lib/auth.ts:36` | randomString lost ~a third of its entropy and emitted a skewed 36-symbol alphabet |
| F-04-03 | LOW | fixed | mitigated | auth-bypass | `apps/web/src/lib/auth.ts:83` | PKCE verifier cleared only on the success path |
| F-04-07 | LOW | fixed | mitigated | race-condition | `apps/web/src/lib/auth.ts:100` | Concurrent refresh could consume a rotated token twice and wipe live credentials |
| F-03-06 | LOW | fixed | mitigated | missing-loop-bound | `backend/crates/altius-api/src/agent.rs:225` | Per-step tool fan-out unbounded; resume reset the step budget |
| F-02-05 | LOW | fixed | mitigated | information-disclosure | `backend/crates/altius-api/src/auth.rs:63` | Upstream reqwest error text returned to clients |
| F-02-07 | LOW | fixed | mitigated | amplification | `backend/crates/altius-api/src/auth.rs:77` | Unknown kid forced an uncached JWKS refetch every request |
| F-02-06 | LOW | fixed | mitigated | auth-bypass | `backend/crates/altius-api/src/auth.rs:98` | nbf never validated |
| F-03-05 | LOW | fixed | mitigated | improper-validation | `backend/crates/altius-api/src/maps.rs:275` | waypoint_order returned to clients as indices without range validation |
| F-02-09 | LOW | fixed | mitigated | information-disclosure | `backend/crates/altius-api/src/routes.rs:69` | Unauthenticated /health disclosed integration posture |
| F-02-08 | LOW | fixed | mitigated | authz-bypass | `backend/crates/altius-api/src/routes.rs:368` | record_report let the submitter set its own approval status |
| F-07-14 | LOW | fixed | mitigated | numeric-handling | `packages/algos/src/index.ts:24` | NaN edge weights bypassed the negative guard and vanished from the path |
| F-07-20 | LOW | fixed | mitigated | input-validation | `packages/api-contracts/src/identity.ts:9` | User email and pagination cursor had no length cap |
| F-07-17 | LOW | fixed | mitigated | fail-open | `packages/api-contracts/src/location.ts:19` | mockLocationReported was a nullable self-assertion |
| F-07-18 | LOW | fixed | mitigated | input-validation | `packages/api-contracts/src/tasks.ts:19` | Client-chosen idempotency keys were unscoped, allowing cross-actor squatting |
