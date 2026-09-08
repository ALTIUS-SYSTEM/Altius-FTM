# `@altius/api-contracts`

TypeScript Zod schemas for Altius FTM domain types. Field names are **camelCase** (and a few contract-specific names such as `actorId`, `occurredAtUtc`, `latitude`).

## Wire JSON vs this package

HTTP JSON on `/api/v3` is **snake_case** on the wire (canonical serialize shape from `altius-core`). The Rust types accept camelCase / contract aliases via `serde(alias)` so a body shaped like these Zod objects can deserialize without an adapter rename pass. Prefer snake_case in new clients; treat aliases as dual-accept for contract parity, not a second official response dialect.
