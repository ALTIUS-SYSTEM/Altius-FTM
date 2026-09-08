# Altius-FTM Developer Portal

Public documentation for **Altius-FTM** (customer IT and integrators): bilingual guides (EN/ID) and an interactive OpenAPI reference (Scalar).

Engineering deep-dives live under [`Docs/`](../../Docs/) (e.g. `ALTIUS_ARCHITECTURE.md`); this portal is the customer-facing surface.

## Run locally

```bash
# from repo root
pnpm install
pnpm dev:portal
# http://127.0.0.1:3200 → redirects to /en
```

Copy [`.env.example`](./.env.example) to `.env.local` if you need non-default dashboard or API URLs.

| Env | Purpose |
|---|---|
| `NEXT_PUBLIC_WEB_APP_URL` | Dashboard origin for “Open dashboard” |
| `NEXT_PUBLIC_API_BASE` | API origin hinted in Scalar servers |

## OpenAPI sync

The interactive reference loads [`public/openapi/altius-ftm-v3.openapi.yaml`](./public/openapi/altius-ftm-v3.openapi.yaml).

**Source of truth:** [`Docs/openapi/altius-ftm-v3.openapi.yaml`](../../Docs/openapi/altius-ftm-v3.openapi.yaml). Re-copy into `public/openapi/` when the source changes:

```bash
cp Docs/openapi/altius-ftm-v3.openapi.yaml apps/developer-portal/public/openapi/
```

## Auth note

This portal is public and does **not** run Keycloak. API “Try it” requires a Bearer JWT obtained from the dashboard / IdP (see guides → API authentication).
