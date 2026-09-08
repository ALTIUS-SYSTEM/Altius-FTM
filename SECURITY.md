# Security Policy — Altius FTM

**Product:** Altius Field Task Manager (Fleet & Transport Management)  
**Owner:** PT Antero Daemon Technologies  
**Last updated:** 2026-09-08

## Scope

This policy covers the Altius-FTM monorepo: `apps/web`, `apps/landing`, `apps/developer-portal`, `apps/mobile`, `packages/*`, and `backend/`.

## Reporting a vulnerability

Email security reports to the maintainers via the private channel used for this repository (do not open a public issue with exploit details).

Include:

- Affected component and version / commit
- Steps to reproduce
- Impact (confidentiality, integrity, availability)
- Whether a fix is already known

We aim to acknowledge within 5 business days.

## Security model (summary)

| Layer | Control |
|---|---|
| Identity | Keycloak OIDC (PKCE for `altius-web` / `altius-mobile`); API validates Bearer JWTs (RS256, `iss` / `aud` / `exp` / `nbf`) |
| Tenant isolation | Server resolves `org_id` / hub / role from JWT `sub` + membership tables — never from client-asserted tenant alone |
| Events | Append-only `device_events`; idempotent `request_key` scoped by `(org_id, driver_sub)` |
| Mobile tokens | `FlutterSecureStorage` / Keychain — not SQLite preferences |
| Maps / LLM keys | Server-side only; never shipped in mobile or web bundles |
| Postgres | Prefer `DATABASE_URL` with `sslmode=require` outside private networks |
| Edge | Caddy TLS + rate limits on VPS; CORS allowlist (`CORS_ORIGINS`) for Vercel origins |
| CI supply chain | GitHub Actions pinned to commit SHAs; pnpm `trustPolicy` / `blockExoticSubdeps` |

See also: [`Docs/ALTIUS_BACKEND_ARCHITECTURE.md`](./Docs/ALTIUS_BACKEND_ARCHITECTURE.md), [`VULN-FINDINGS.md`](./VULN-FINDINGS.md), [`AGENTS.md`](./AGENTS.md).

## Secrets

- Never commit `.env`, keystores, or realm admin passwords
- Rotate any credential that appears in chat, tickets, or screenshots
- Prefer SSH keys over password root login on VPS hosts

## Supported versions

Only the `main` (or designated production) branch of this repository is supported for security fixes. Deployed images should be rebuilt from that branch after a fix lands.

## Out of scope

- Third-party IdP / hosting misconfiguration outside this repo
- Physical device theft without disk encryption (mobile SQLite holds operational PII; SQLCipher is a planned hardening — see F-06-05)
- Denial-of-service against public Keycloak or Maps quotas
