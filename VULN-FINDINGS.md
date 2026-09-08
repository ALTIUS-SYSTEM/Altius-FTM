# Altius FTM — Static Vulnerability Review

62 findings across 7 focus areas, 74 source files.
Severity: 16 high / 25 medium / 21 low.
Remediation: **55 fixed**, 3 partial, 4 open.

Static review — no code was executed. Fix status reflects verified source changes;
the whole workspace passes typecheck, lint, clippy `-D warnings`, and all tests.

| ID | Sev | Status | Category | Location | Title |
|---|---|---|---|---|---|
| F-07-11 | MEDIUM | open | fail-open | `packages/algos/src/index.ts:89` | aggregateDaily drops entries whose day string does not match exactly |
| F-07-19 | MEDIUM | open | input-validation | `packages/api-contracts/src/primitives.ts:16` | Rust mirror accepts negative money, unknown fields and an unconstrained payload |
| F-06-05 | LOW | open | information-disclosure | `apps/mobile/lib/core/data/work_store.dart:108` | Operational PII stored in an unencrypted on-device database |
| F-05-08 | LOW | open | input-validation | `apps/web/src/features/tasks.tsx:33` | validateTask applied only on the editor submit path |
| F-03-04 | MEDIUM | partial | authz-bypass | `backend/crates/altius-api/src/agent.rs:32` | Tool executors carry no caller identity |
| F-07-10 | MEDIUM | partial | numeric-handling | `packages/algos/src/index.ts:93` | aggregateDaily summed currency as floats across mixed currencies |
| F-07-06 | MEDIUM | partial | privilege-escalation | `packages/api-contracts/src/tasks.ts:20` | Tenant/hub/actor carried in request bodies, cross-checked only against each other |
| F-06-02 | HIGH | fixed | cleartext-transport | `apps/mobile/lib/core/data/work_store.dart:163` | Server URL never validated as HTTPS |
| F-06-01 | HIGH | fixed | credential-storage | `apps/mobile/lib/core/data/work_store.dart:192` | Access and refresh tokens persisted in plaintext SQLite |
| F-05-01 | HIGH | fixed | csv-injection | `apps/web/src/features/schedule-planner.tsx:136` | Schedule CSV built by raw concatenation with no quoting or formula guard |
| F-04-01 | HIGH | fixed | csrf | `apps/web/src/lib/auth.ts:57` | OAuth state generated but never stored or verified |
| F-03-02 | HIGH | fixed | authz-bypass | `backend/crates/altius-api/src/agent.rs:121` | agent::resume trusted client conversation state and executed a client-chosen tool call |
| F-03-01 | HIGH | fixed | information-disclosure | `backend/crates/altius-api/src/maps.rs:181` | GOOGLE_MAPS_API_KEY returned in 503 bodies via reqwest error Display |
| F-02-03 | HIGH | fixed | authz-bypass | `backend/crates/altius-api/src/routes.rs:185` | Tenant fell back to an unverified organization_id token claim |
| F-02-01 | HIGH | fixed | authz-bypass | `backend/crates/altius-api/src/routes.rs:218` | create_task took tenant from the request body behind a Driver deny-list |
| F-02-02 | HIGH | fixed | idor | `backend/crates/altius-api/src/routes.rs:249` | sync_events scope check was self-referential |
| F-01-02 | HIGH | fixed | authz-bypass | `backend/crates/altius-api/src/store.rs:114` | create_task bound $o but never used it; body hub-id planted tasks in other tenants |
| F-01-01 | HIGH | fixed | idor | `backend/crates/altius-api/src/store.rs:235` | record_event mutated tasks/stops by raw id with no org constraint |
| F-07-01 | HIGH | fixed | fail-open | `packages/algos/src/index.ts:62` | evaluateCorridor returned a clean 'inside' for an empty sample set |
| F-07-02 | HIGH | fixed | fail-open | `packages/algos/src/index.ts:66` | Device-declared accuracy silently suppressed every sample |
| F-07-03 | HIGH | fixed | fail-open | `packages/algos/src/index.ts:84` | compareGpsStreams degraded to insufficient-data under attacker timestamps |
| F-07-05 | HIGH | fixed | privilege-escalation | `packages/api-contracts/src/identity.ts:6` | Membership permissions were client-assertable and unbound from role |
| F-07-04 | HIGH | fixed | privilege-escalation | `packages/api-contracts/src/lhs.ts:10` | LhsReport let the client assert its own approved status and reviewer identity |
| F-06-07 | MEDIUM | fixed | authorization | `apps/mobile/lib/core/data/work_store.dart:350` | Client-asserted tenant/hub/driver and local-only stage rules |
| F-06-03 | MEDIUM | fixed | data-loss | `apps/mobile/lib/core/data/work_store.dart:387` | Sync receipts matched to local events by array position |
| F-06-04 | MEDIUM | fixed | data-loss | `apps/mobile/lib/core/data/work_store.dart:389` | Any non-'accepted' status became a permanent 'rejected' drop |
| F-04-05 | MEDIUM | fixed | auth-bypass | `apps/web/src/app/[[...path]]/view.tsx:33` | Route guard trusted a persisted session boolean |
| F-05-02 | MEDIUM | fixed | csv-injection | `apps/web/src/data/adapter.ts:41` | csvCell formula regex anchored at position 0; a leading space bypassed it |
| F-05-03 | MEDIUM | fixed | input-validation | `apps/web/src/data/api-adapter.ts:111` | API-sourced tasks bypassed Zod entirely |
| F-05-05 | MEDIUM | fixed | data-loss | `apps/web/src/data/api-adapter.ts:118` | Live adapter silently discarded every task mutation while reporting success |
| F-04-04 | MEDIUM | fixed | token-leak | `apps/web/src/lib/auth.ts:95` | Refresh token for a public client persisted in sessionStorage |
| F-03-03 | MEDIUM | fixed | prompt-injection | `backend/crates/altius-api/src/agent.rs:251` | Tool arguments never validated against the declared schema |
| F-02-04 | MEDIUM | fixed | authz-bypass | `backend/crates/altius-api/src/routes.rs:317` | Roster routes had no role gate; a token with no known role was a full principal |
| F-01-05 | MEDIUM | fixed | sql-injection | `backend/crates/altius-api/src/store.rs:23` | esc() escaped only backslash and quote; C0 control characters passed through |
| F-01-03 | MEDIUM | fixed | toctou | `backend/crates/altius-api/src/store.rs:177` | Idempotency check-then-insert race |
| F-01-04 | MEDIUM | fixed | authz-bypass | `backend/crates/altius-api/src/store.rs:178` | Idempotency keys shared one global namespace across tenants |
| F-07-12 | MEDIUM | fixed | numeric-handling | `packages/algos/src/index.ts:5` | NaN coordinates propagated into comparisons that fail open |
| F-07-13 | MEDIUM | fixed | input-validation | `packages/algos/src/index.ts:13` | dijkstra indexed the graph by untrusted node name, reaching Object.prototype |
| F-07-08 | MEDIUM | fixed | algorithmic-complexity | `packages/algos/src/index.ts:81` | compareGpsStreams was O(n*m) over two unbounded arrays |
| F-07-15 | MEDIUM | fixed | input-validation | `packages/api-contracts/src/api.ts:26` | z.record escape hatches accepted unbounded keys and values |
| F-07-07 | MEDIUM | fixed | fail-open | `packages/api-contracts/src/location.ts:14` | GPS source self-declared, so one device could forge both corroborating streams |
| F-07-09 | MEDIUM | fixed | input-validation | `packages/api-contracts/src/location.ts:24` | Collection fields had no .max() before reaching storage |
| F-07-16 | MEDIUM | fixed | input-validation | `packages/api-contracts/src/primitives.ts:16` | Money amounts capped only at MAX_SAFE_INTEGER |
| F-06-06 | LOW | fixed | information-disclosure | `apps/mobile/lib/core/data/work_store.dart:205` | Logout left the previous driver's email and server URL on the device |
| F-05-06 | LOW | fixed | state-corruption | `apps/web/src/components/demo-provider.tsx:35` | Adapter write and setError ran inside a setState updater |
| F-05-04 | LOW | fixed | unbounded-recursion | `apps/web/src/data/api-adapter.ts:22` | leaf() recursed without a depth bound |
| F-05-07 | LOW | fixed | data-loss | `apps/web/src/data/api-adapter.ts:115` | A single malformed localStorage value silently discarded all persisted preferences |
| F-04-06 | LOW | fixed | auth-bypass | `apps/web/src/features/callback.tsx:41` | Callback ignored the OAuth error param and left code in the URL |
| F-04-02 | LOW | fixed | weak-randomness | `apps/web/src/lib/auth.ts:36` | randomString lost ~a third of its entropy and emitted a skewed 36-symbol alphabet |
| F-04-03 | LOW | fixed | auth-bypass | `apps/web/src/lib/auth.ts:83` | PKCE verifier cleared only on the success path |
| F-04-07 | LOW | fixed | race-condition | `apps/web/src/lib/auth.ts:100` | Concurrent refresh could consume a rotated token twice and wipe live credentials |
| F-03-06 | LOW | fixed | missing-loop-bound | `backend/crates/altius-api/src/agent.rs:225` | Per-step tool fan-out unbounded; resume reset the step budget |
| F-02-05 | LOW | fixed | information-disclosure | `backend/crates/altius-api/src/auth.rs:63` | Upstream reqwest error text returned to clients |
| F-02-07 | LOW | fixed | amplification | `backend/crates/altius-api/src/auth.rs:77` | Unknown kid forced an uncached JWKS refetch every request |
| F-02-06 | LOW | fixed | auth-bypass | `backend/crates/altius-api/src/auth.rs:98` | nbf never validated |
| F-03-05 | LOW | fixed | improper-validation | `backend/crates/altius-api/src/maps.rs:275` | waypoint_order returned to clients as indices without range validation |
| F-02-09 | LOW | fixed | information-disclosure | `backend/crates/altius-api/src/routes.rs:69` | Unauthenticated /health disclosed integration posture |
| F-02-08 | LOW | fixed | authz-bypass | `backend/crates/altius-api/src/routes.rs:368` | record_report let the submitter set its own approval status |
| F-07-14 | LOW | fixed | numeric-handling | `packages/algos/src/index.ts:24` | NaN edge weights bypassed the negative guard and vanished from the path |
| F-07-20 | LOW | fixed | input-validation | `packages/api-contracts/src/identity.ts:9` | User email and pagination cursor had no length cap |
| F-07-17 | LOW | fixed | fail-open | `packages/api-contracts/src/location.ts:19` | mockLocationReported was a nullable self-assertion |
| F-07-18 | LOW | fixed | input-validation | `packages/api-contracts/src/tasks.ts:19` | Client-chosen idempotency keys were unscoped, allowing cross-actor squatting |

## Detail

### F-07-11 — aggregateDaily drops entries whose day string does not match exactly

- **Severity** MEDIUM  ·  **Status** open  ·  **Category** fail-open
- **Location** `packages/algos/src/index.ts:89`
- **Remediation** Day is still a bare client-supplied string; deriving it from a timestamp plus the report time zone is not done.

### F-07-19 — Rust mirror accepts negative money, unknown fields and an unconstrained payload

- **Severity** MEDIUM  ·  **Status** open  ·  **Category** input-validation
- **Location** `packages/api-contracts/src/primitives.ts:16`
- **Remediation** amount_minor is still i64, structs lack deny_unknown_fields, ExpenseCategory has 4 variants against the contract's 6, and DeviceEvent.payload has no Zod counterpart.

### F-06-05 — Operational PII stored in an unencrypted on-device database

- **Severity** LOW  ·  **Status** open  ·  **Category** information-disclosure
- **Location** `apps/mobile/lib/core/data/work_store.dart:108`
- **Remediation** Needs SQLCipher plus a keystore-held key and a retention window; not a code-only change.

### F-05-08 — validateTask applied only on the editor submit path

- **Severity** LOW  ·  **Status** open  ·  **Category** input-validation
- **Location** `apps/web/src/features/tasks.tsx:33`
- **Remediation** Assignment/arrival/completion paths still bypass it. Now lower risk because API tasks are Zod-validated on load (F-05-03), but the chokepoint fix — validating inside update() — is not done.

### F-03-04 — Tool executors carry no caller identity

- **Severity** MEDIUM  ·  **Status** partial  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/agent.rs:32`
- **Remediation** Resume is now bound to the approving subject and gated-only, but Exec is still Fn(Value): a future tool would still run with ambient backend credentials. Threading &Principal through Exec remains open.

### F-07-10 — aggregateDaily summed currency as floats across mixed currencies

- **Severity** MEDIUM  ·  **Status** partial  ·  **Category** numeric-handling
- **Location** `packages/algos/src/index.ts:93`
- **Remediation** Non-finite/negative/non-safe-integer amounts are now rejected and counted (rejectedCosts). Per-currency separation still not modelled — the function keeps a single scalar total.

### F-07-06 — Tenant/hub/actor carried in request bodies, cross-checked only against each other

- **Severity** MEDIUM  ·  **Status** partial  ·  **Category** privilege-escalation
- **Location** `packages/api-contracts/src/tasks.ts:20`
- **Remediation** No longer exploitable — the API derives scope from the token and ignores the body — but the contracts still carry the fields and still invite the mistake.

### F-06-02 — Server URL never validated as HTTPS

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** cleartext-transport
- **Location** `apps/mobile/lib/core/data/work_store.dart:163`
- **Remediation** parseServerUrl rejects any non-https scheme or empty host.

### F-06-01 — Access and refresh tokens persisted in plaintext SQLite

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** credential-storage
- **Location** `apps/mobile/lib/core/data/work_store.dart:192`
- **Remediation** Routed through SecureTokenStore (Keychain/Keystore); preferences hold no secrets.

### F-05-01 — Schedule CSV built by raw concatenation with no quoting or formula guard

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** csv-injection
- **Location** `apps/web/src/features/schedule-planner.tsx:136`
- **Remediation** Routed through csvCell with CRLF joins.

### F-04-01 — OAuth state generated but never stored or verified

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** csrf
- **Location** `apps/web/src/lib/auth.ts:57`
- **Remediation** Stored at startLogin, compared in finishLogin, cleared on every exit.

### F-03-02 — agent::resume trusted client conversation state and executed a client-chosen tool call

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/agent.rs:121`
- **Remediation** Paused runs are sealed as an HS256 JWT bound to the subject with a TTL and step budget; resume executes the sealed call only, and refuses a non-gated tool.

### F-03-01 — GOOGLE_MAPS_API_KEY returned in 503 bodies via reqwest error Display

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** information-disclosure
- **Location** `backend/crates/altius-api/src/maps.rs:181`
- **Remediation** Routed through ApiError::upstream.

### F-02-03 — Tenant fell back to an unverified organization_id token claim

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/routes.rs:185`
- **Remediation** Fallback removed; no membership row = Forbidden.

### F-02-01 — create_task took tenant from the request body behind a Driver deny-list

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/routes.rs:218`
- **Remediation** require_staff allow-list; org resolved from membership.

### F-02-02 — sync_events scope check was self-referential

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** idor
- **Location** `backend/crates/altius-api/src/routes.rs:249`
- **Remediation** Closed in the store so every caller inherits it.

### F-01-02 — create_task bound $o but never used it; body hub-id planted tasks in other tenants

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/store.rs:114`
- **Remediation** allocation (org,hub) clause added, plus a pre-insert allocation check so an empty match can no longer report success.

### F-01-01 — record_event mutated tasks/stops by raw id with no org constraint

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** idor
- **Location** `backend/crates/altius-api/src/store.rs:235`
- **Remediation** Every task/stop lookup now joins organization → allocation → located → contains; an out-of-scope stop returns Conflict instead of being ignored.

### F-07-01 — evaluateCorridor returned a clean 'inside' for an empty sample set

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** fail-open
- **Location** `packages/algos/src/index.ts:62`
- **Remediation** Nothing measurable now returns 'inaccurate', not a pass.

### F-07-02 — Device-declared accuracy silently suppressed every sample

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** fail-open
- **Location** `packages/algos/src/index.ts:66`
- **Remediation** Skipped samples are counted; an all-skipped batch is 'inaccurate'.

### F-07-03 — compareGpsStreams degraded to insufficient-data under attacker timestamps

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** fail-open
- **Location** `packages/algos/src/index.ts:84`
- **Remediation** matched === 0 with a non-empty app stream is now 'review'.

### F-07-05 — Membership permissions were client-assertable and unbound from role

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** privilege-escalation
- **Location** `packages/api-contracts/src/identity.ts:6`
- **Remediation** MembershipRequestSchema omits permissions; array capped and deduped.

### F-07-04 — LhsReport let the client assert its own approved status and reviewer identity

- **Severity** HIGH  ·  **Status** fixed  ·  **Category** privilege-escalation
- **Location** `packages/api-contracts/src/lhs.ts:10`
- **Remediation** LhsReportRequestSchema omits status, revision and reviews; server-owned.

### F-06-07 — Client-asserted tenant/hub/driver and local-only stage rules

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** authorization
- **Location** `apps/mobile/lib/core/data/work_store.dart:350`
- **Remediation** Neutralised server-side: the API derives org from membership and re-validates transitions (F-01-01, F-02-02).

### F-06-03 — Sync receipts matched to local events by array position

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** data-loss
- **Location** `apps/mobile/lib/core/data/work_store.dart:387`
- **Remediation** Matched by event_id; unknown ids ignored; a shortfall is reported as a partial sync instead of success.

### F-06-04 — Any non-'accepted' status became a permanent 'rejected' drop

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** data-loss
- **Location** `apps/mobile/lib/core/data/work_store.dart:389`
- **Remediation** Only accepted/rejected are terminal; anything else stays pending and is retried.

### F-04-05 — Route guard trusted a persisted session boolean

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** auth-bypass
- **Location** `apps/web/src/app/[[...path]]/view.tsx:33`
- **Remediation** Guard now keys on hasSession() token possession when Keycloak is configured.

### F-05-02 — csvCell formula regex anchored at position 0; a leading space bypassed it

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** csv-injection
- **Location** `apps/web/src/data/adapter.ts:41`
- **Remediation** Leading whitespace and control chars now precede the trigger class.

### F-05-03 — API-sourced tasks bypassed Zod entirely

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `apps/web/src/data/api-adapter.ts:111`
- **Remediation** Each mapped task is safeParsed; malformed rows are dropped and warned.

### F-05-05 — Live adapter silently discarded every task mutation while reporting success

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** data-loss
- **Location** `apps/web/src/data/api-adapter.ts:118`
- **Remediation** save() throws when tasks diverge from the loaded set; the toast now fires only on a real persist.

### F-04-04 — Refresh token for a public client persisted in sessionStorage

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** token-leak
- **Location** `apps/web/src/lib/auth.ts:95`
- **Remediation** Tokens held in memory only; endSession() added to revoke the Keycloak SSO session.

### F-03-03 — Tool arguments never validated against the declared schema

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** prompt-injection
- **Location** `backend/crates/altius-api/src/agent.rs:251`
- **Remediation** validate_args enforces object shape and required keys; parse failures go back to the model instead of becoming {}.

### F-02-04 — Roster routes had no role gate; a token with no known role was a full principal

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/routes.rs:317`
- **Remediation** require_staff on /users,/hubs,/drivers; empty role set rejected at validate().

### F-01-05 — esc() escaped only backslash and quote; C0 control characters passed through

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** sql-injection
- **Location** `backend/crates/altius-api/src/store.rs:23`
- **Remediation** newline/CR/tab escaped, remaining C0 dropped.

### F-01-03 — Idempotency check-then-insert race

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** toctou
- **Location** `backend/crates/altius-api/src/store.rs:177`
- **Remediation** request-key @unique violation at commit is now absorbed as the replay it represents, instead of 500-ing the batch.

### F-01-04 — Idempotency keys shared one global namespace across tenants

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/store.rs:178`
- **Remediation** Keys namespaced by length-prefixed org+driver; replay path returns the stored event-id, not the client's key.

### F-07-12 — NaN coordinates propagated into comparisons that fail open

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** numeric-handling
- **Location** `packages/algos/src/index.ts:5`
- **Remediation** haversineMeters returns Infinity for non-finite input; unmeasurable samples no longer reset the breach streak.

### F-07-13 — dijkstra indexed the graph by untrusted node name, reaching Object.prototype

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `packages/algos/src/index.ts:13`
- **Remediation** Adjacency held in a Map built from Object.entries; non-array values filtered.

### F-07-08 — compareGpsStreams was O(n*m) over two unbounded arrays

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** algorithmic-complexity
- **Location** `packages/algos/src/index.ts:81`
- **Remediation** Sort-once two-pointer match, plus a 5000-observation contract cap.

### F-07-15 — z.record escape hatches accepted unbounded keys and values

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `packages/api-contracts/src/api.ts:26`
- **Remediation** Keys constrained to IdSchema minus __proto__/constructor/prototype; values capped; 200-key limit.

### F-07-07 — GPS source self-declared, so one device could forge both corroborating streams

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** fail-open
- **Location** `packages/api-contracts/src/location.ts:14`
- **Remediation** vehicle_gps now requires a non-null vehicleId. Binding deviceId to a registered telematics unit remains a server-side task.

### F-07-09 — Collection fields had no .max() before reaching storage

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `packages/api-contracts/src/location.ts:24`
- **Remediation** Caps added across location, lhs, routes and identity.

### F-07-16 — Money amounts capped only at MAX_SAFE_INTEGER

- **Severity** MEDIUM  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `packages/api-contracts/src/primitives.ts:16`
- **Remediation** MAX_AMOUNT_MINOR of 10^12 per line item.

### F-06-06 — Logout left the previous driver's email and server URL on the device

- **Severity** LOW  ·  **Status** fixed  ·  **Category** information-disclosure
- **Location** `apps/mobile/lib/core/data/work_store.dart:205`
- **Remediation** logout() deletes all draft: keys.

### F-05-06 — Adapter write and setError ran inside a setState updater

- **Severity** LOW  ·  **Status** fixed  ·  **Category** state-corruption
- **Location** `apps/web/src/components/demo-provider.tsx:35`
- **Remediation** Persistence moved to a microtask outside the render phase; adapter memoized.

### F-05-04 — leaf() recursed without a depth bound

- **Severity** LOW  ·  **Status** fixed  ·  **Category** unbounded-recursion
- **Location** `apps/web/src/data/api-adapter.ts:22`
- **Remediation** Depth capped at 16.

### F-05-07 — A single malformed localStorage value silently discarded all persisted preferences

- **Severity** LOW  ·  **Status** fixed  ·  **Category** data-loss
- **Location** `apps/web/src/data/api-adapter.ts:115`
- **Remediation** Rejected blob is preserved under a .corrupt key and warned rather than overwritten.

### F-04-06 — Callback ignored the OAuth error param and left code in the URL

- **Severity** LOW  ·  **Status** fixed  ·  **Category** auth-bypass
- **Location** `apps/web/src/features/callback.tsx:41`
- **Remediation** error/error_description surfaced; query stripped on entry.

### F-04-02 — randomString lost ~a third of its entropy and emitted a skewed 36-symbol alphabet

- **Severity** LOW  ·  **Status** fixed  ·  **Category** weak-randomness
- **Location** `apps/web/src/lib/auth.ts:36`
- **Remediation** Deleted; both state and verifier use the 32-byte base64url primitive.

### F-04-03 — PKCE verifier cleared only on the success path

- **Severity** LOW  ·  **Status** fixed  ·  **Category** auth-bypass
- **Location** `apps/web/src/lib/auth.ts:83`
- **Remediation** try/finally clears verifier and state on every path.

### F-04-07 — Concurrent refresh could consume a rotated token twice and wipe live credentials

- **Severity** LOW  ·  **Status** fixed  ·  **Category** race-condition
- **Location** `apps/web/src/lib/auth.ts:100`
- **Remediation** Single in-flight promise; a losing racer no longer clears a winner's tokens.

### F-03-06 — Per-step tool fan-out unbounded; resume reset the step budget

- **Severity** LOW  ·  **Status** fixed  ·  **Category** missing-loop-bound
- **Location** `backend/crates/altius-api/src/agent.rs:225`
- **Remediation** MAX_TOOL_CALLS_PER_STEP cap and a budget carried inside the seal across resumes.

### F-02-05 — Upstream reqwest error text returned to clients

- **Severity** LOW  ·  **Status** fixed  ·  **Category** information-disclosure
- **Location** `backend/crates/altius-api/src/auth.rs:63`
- **Remediation** ApiError::upstream logs the cause and returns a fixed message.

### F-02-07 — Unknown kid forced an uncached JWKS refetch every request

- **Severity** LOW  ·  **Status** fixed  ·  **Category** amplification
- **Location** `backend/crates/altius-api/src/auth.rs:77`
- **Remediation** 30s cooldown on unknown-kid refetches; TTL refresh unaffected.

### F-02-06 — nbf never validated

- **Severity** LOW  ·  **Status** fixed  ·  **Category** auth-bypass
- **Location** `backend/crates/altius-api/src/auth.rs:98`
- **Remediation** validate_nbf plus required spec claims.

### F-03-05 — waypoint_order returned to clients as indices without range validation

- **Severity** LOW  ·  **Status** fixed  ·  **Category** improper-validation
- **Location** `backend/crates/altius-api/src/maps.rs:275`
- **Remediation** Verified as a permutation of 0..waypoints.len(); falls back to input order otherwise.

### F-02-09 — Unauthenticated /health disclosed integration posture

- **Severity** LOW  ·  **Status** fixed  ·  **Category** information-disclosure
- **Location** `backend/crates/altius-api/src/routes.rs:69`
- **Remediation** Liveness only; posture moved to a debug log.

### F-02-08 — record_report let the submitter set its own approval status

- **Severity** LOW  ·  **Status** fixed  ·  **Category** authz-bypass
- **Location** `backend/crates/altius-api/src/routes.rs:368`
- **Remediation** status forced to Submitted, revision to 0.

### F-07-14 — NaN edge weights bypassed the negative guard and vanished from the path

- **Severity** LOW  ·  **Status** fixed  ·  **Category** numeric-handling
- **Location** `packages/algos/src/index.ts:24`
- **Remediation** Whole graph validated up front; non-finite weights return null, negative still throws (existing contract).

### F-07-20 — User email and pagination cursor had no length cap

- **Severity** LOW  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `packages/api-contracts/src/identity.ts:9`
- **Remediation** email capped at 254; memberships capped at 64.

### F-07-17 — mockLocationReported was a nullable self-assertion

- **Severity** LOW  ·  **Status** fixed  ·  **Category** fail-open
- **Location** `packages/api-contracts/src/location.ts:19`
- **Remediation** Required non-null for app_gps observations that carry a position.

### F-07-18 — Client-chosen idempotency keys were unscoped, allowing cross-actor squatting

- **Severity** LOW  ·  **Status** fixed  ·  **Category** input-validation
- **Location** `packages/api-contracts/src/tasks.ts:19`
- **Remediation** Server-side dedup is keyed on org+driver+key (F-01-04).
