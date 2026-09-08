# Altius FTM — API Endpoint Reference

> **Product:** Altius FTM (Fleet & Transport Management)  
> **Package:** `com.altius.altius_field`  
> **Base URL:** `{BASE_URL}/api/v3`  
> **Auth (production):** Keycloak OIDC Bearer (RS256) — see § Implemented surface  
> **Date:** 2026-09-08  

This document describes the Altius FTM HTTP API. **Section 0** lists routes
implemented in [`backend/`](../backend/). Later sections retain the broader
product surface (including flows still planned or client-local).

**Machine-readable spec:** [openapi/altius-ftm-v3.openapi.yaml](./openapi/altius-ftm-v3.openapi.yaml) (OpenAPI 3.0.3).  
**Integrator guide:** [ALTIUS_API_GUIDE.md](./ALTIUS_API_GUIDE.md).

---

## 0. Implemented surface (`backend/`)

Prefer these when integrating web, mobile, or Vercel → VPS.

| Method | Path | Auth | Notes |
|---|---|---|---|
| GET | `/health` | — | Liveness |
| GET | `/ready` | — | Readiness (503 if persistence down) |
| POST | `/auth/login` | — | Password grant only if explicitly enabled |
| POST | `/auth/refresh` | — | Refresh token exchange |
| GET | `/auth/me` | Bearer | Subject, roles, org/hub scope |
| GET | `/tasks` | Bearer | Org board for staff / `integration`; assigned-only for drivers |
| GET | `/task/{id}` | Bearer | Single task + stops (drivers: own assignee only) |
| PUT | `/task/{id}` | Bearer (staff) | Update task; org from JWT |
| DELETE | `/task/{id}` | Bearer (staff) | Delete task; 409 if `in_progress` |
| POST | `/task-create` | Bearer (staff) | Create task + stops under caller hub |
| POST | `/events` | Bearer (driver) | Idempotent outbox sync; receipts include `event_id`; skip needs `reason` |
| POST | `/route/optimize` | Bearer | Stop order optimization |
| POST | `/route/eta` | Bearer | Directions ETA or schematic fallback |
| POST | `/route/geocode` | Bearer | Address → coordinate |
| POST | `/route/static-map` | Bearer | Server-proxied static map |
| POST | `/places/autocomplete` | Bearer | Place suggestions |
| GET/POST | `/users`, `/drivers`, hubs, teams, … | Bearer | Roster / org admin |
| GET/POST | `/reports`, `/costs`, `/vehicle-checks` | Bearer | LHS / costs |
| PATCH | `/reports/{driver}/{day}` | Bearer (staff) | Review: `approved` \| `revision_requested` (+ note) |
| GET/POST/DELETE | `/integrations`, `/integrations/{sub}` | Bearer (admin) | Bind/list/remove M2M service-account subjects |
| GET/POST | `/monitoring/*` | Bearer | Vehicles, GPS reviews, McEasy hooks |
| POST | `/notify/*` | Bearer | SMS / WhatsApp / push (provider-gated) |
| POST | `/agent/dispatch-suggestion`, `/agent/resume` | Bearer (staff) | HITL agent loop |

**M2M:** realm role `integration` + client-credentials Bearer (OpenAPI `oauth2ClientCredentials`); org binding via `/integrations`. Read org tasks/reports; staff writes stay staff-only.  
**DeviceEvent:** snake_case wire + camelCase deserialize aliases; optional `schema_version`, `device_sequence`, `expected_task_revision`, `reason`, `observation_id` (V4 columns).  
**Retention:** GPS via `MCEASY_RETENTION_HOURS`; device events via `EVENTS_RETENTION_DAYS` (default 90, `0` off).  
**Mobile sync:** match receipts by `event_id` / `server_event_id`, never by array index.  
**Web:** set `NEXT_PUBLIC_API_BASE` + Keycloak public client `altius-web`.  
**OpenAPI:** [openapi/altius-ftm-v3.openapi.yaml](./openapi/altius-ftm-v3.openapi.yaml).

---

## Table of Contents

1. [Environment Endpoints](#1-environment-endpoints)
2. [Authentication API](#2-authentication-api)
3. [User API](#3-user-api)
4. [Organization API](#4-organization-api)
5. [OTP API](#5-otp-api)
6. [Cloud Authenticator API](#6-cloud-authenticator-api)
7. [Task API](#7-task-api)
8. [Flow API](#8-flow-api)
9. [Data Source API](#9-data-source-api)
10. [Currency API](#10-currency-api)
11. [Custom Modules API](#11-custom-modules-api)
12. [Location History API](#12-location-history-api)
13. [Device Token API](#13-device-token-api)
14. [Main Menu API](#14-main-menu-api)
15. [Media Upload API](#15-media-upload-api)
16. [Troubleshooting API](#16-troubleshooting-api)
17. [Privacy Policy API](#17-privacy-policy-api)
18. [Version API](#18-version-api)
19. [Keys API](#19-keys-api)
20. [Webhook API](#20-webhook-api)
21. [Maps API](#21-maps-api)
22. [WebView API](#22-webview-api)

---

## 1. Environment Endpoints

| Flavor | Base URL | Web Origin | S3 Region |
|---|---|---|---|
| Production | `https://api.altius.example/api/v3` | `https://app.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Development | `https://api-dev.altius.example/api/v3` | `https://app-dev.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Beta | `https://api-beta.altius.example/api/v3` | `https://app-beta.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Sandbox | `https://api-sandbox.altius.example/api/v3` | `https://app-sandbox.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Unilever | `https://api-enterprise.altius.example/api/v3` | `https://app.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Unilever Sandbox | `https://api-sandbox.altius.example/api/v3` | `https://app.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Y3 | `https://api.altius.example/api/v3` | `https://app.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |
| Y3 Sandbox | `https://api-sandbox.altius.example/api/v3` | `https://app-sandbox.altius.example/` | `s3-ap-southeast-1.amazonaws.com` |

### HMS Integration

| Environment | HMS URL |
|---|---|
| Production | `https://lms.altius.example/hms/elicense/road-hazard-awareness` |
| Sandbox/Dev/Beta | `https://lms-sandbox.altius.example/hms/elicense/road-hazard-awareness` |

### Unilever Integration

| Environment | Unilever Base URL |
|---|---|
| Production | `https://tenant-a.altius.example` |
| Dev/Sandbox | `https://tenant-a-dev.altius.example` |

---

## 2. Authentication API

### POST /auth
Login with email and password.

**Request:**
```json
{
  "email": "user@example.com",
  "password": "hashed_password"
}
```

**Response (success):**
```json
{
  "token": "jwt_bearer_token",
  "expires_in": 3600,
  "user": {
    "id": "user_id",
    "email": "user@example.com",
    "name": "User Name",
    "org_id": "org_id",
    "hub_id": "hub_id",
    "role": "field_worker"
  }
}
```

**Response (MFA required):**
```json
{
  "mfa_required": true,
  "mfa_type": "otp" | "totp",
  "mfa_token": "verification_token"
}
```

**Response (password expired):**
```json
{
  "password_expired": true,
  "change_password_token": "temp_token"
}
```

### POST /logout
Invalidate current session token.

**Headers:** `Authorization: Bearer {token}`

---

## 3. User API

### GET /user
Get current user profile.

### POST /user/forgot-password
Request password recovery.

**Request:**
```json
{
  "email": "user@example.com"
}
```

### POST /user/change-password
Change password (for expired password or voluntary change).

**Request:**
```json
{
  "old_password": "old_hashed",
  "new_password": "new_hashed"
}
```

---

## 4. Organization API

### GET /organizations
List all organizations available to the current user.

**Response:**
```json
{
  "organizations": [
    {
      "id": "org_id",
      "name": "Organization Name",
      "configuration": { ... },
      "is_avian": false
    }
  ]
}
```

### POST /organization/switch
Switch to a different organization. Clears previous org context.

**Request:**
```json
{
  "organization_id": "org_id"
}
```

**Response:**
```json
{
  "token": "new_jwt_token",
  "configuration": { ... }
}
```

---

## 5. OTP API

### POST /otp/send
Send OTP code to recipient (SMS or email).

**Request:**
```json
{
  "recipient": "phone_or_email",
  "service_sender": "sms" | "email"
}
```

### POST /otp/verify
Verify OTP code.

**Request:**
```json
{
  "code": "123456",
  "mfa_token": "verification_token"
}
```

**Response (success):**
```json
{
  "verified": true,
  "token": "jwt_bearer_token"
}
```

**Response (failure):**
```json
{
  "verified": false,
  "message": "Invalid code",
  "attempts_remaining": 2
}
```

---

## 6. Cloud Authenticator API

### POST /cloud-authenticator
Initialize TOTP authenticator setup.

### POST /cloud-authenticator/verify
Verify TOTP code from authenticator app.

**Request:**
```json
{
  "code": "123456"
}
```

---

## 7. Task API

### GET /tasks
Get task list for current user (filtered by org, hub, status).

**Query Parameters:**
- `status` — `ongoing` | `done` | `pending`
- `hub_id` — filter by hub
- `page` — pagination
- `limit` — page size

### GET /task/{id}
Get task detail by ID.

**Response:**
```json
{
  "id": "task_id",
  "title": "Task Title",
  "flow_id": "flow_id",
  "hub_id": "hub_id",
  "assignee_id": "user_id",
  "status": "ongoing",
  "data": { ... },
  "created_at": "2026-09-07T00:00:00Z",
  "updated_at": "2026-09-07T00:00:00Z"
}
```

### POST /task-create
Create a new task.

**Request:**
```json
{
  "flow_id": "flow_id",
  "hub_id": "hub_id",
  "title": "Task Title",
  "assignee_id": "user_id"
}
```

### POST /tasks/bulk
Bulk update tasks (status, data).

**Request:**
```json
{
  "tasks": [
    { "id": "task_id", "status": "done", "data": { ... } }
  ]
}
```

### POST /task/photo/
Upload photo for a task.

**Request:** `multipart/form-data`
- `task_id` — task ID
- `page_id` — page ID
- `component_id` — component ID
- `file` — image file
- `category` — photo category (optional)

### POST /task/video/
Upload video for a task.

**Request:** `multipart/form-data`
- `task_id` — task ID
- `page_id` — page ID
- `component_id` — component ID
- `file` — video file

### POST /voice/
Upload voice recording.

### POST /compressed/
Upload compressed media (compressed video/images).

---

## 8. Flow API

### GET /flow/{id}
Get flow definition by ID.

**Response:**
```json
{
  "id": "flow_id",
  "name": "Flow Name",
  "pages": [
    {
      "id": "page_id",
      "title": "Page Title",
      "components": [
        {
          "id": "component_id",
          "type": "photo" | "video" | "input" | "select" | "list" | "bill" | "scan_display" | "otp" | "capture" | "voice" | "print" | "view" | "subpage",
          "required": true,
          "configuration": { ... }
        }
      ]
    }
  ],
  "version": 3
}
```

### GET /flows/check-version
Check if flow data version has changed.

**Request:**
```json
{
  "current_version": 3
}
```

**Response:**
```json
{
  "latest_version": 4,
  "update_required": true
}
```

---

## 9. Data Source API

### GET /data-source-list
List all data sources for current org.

### GET /data-type-list
List data types for a data source.

### GET /data-types
Get entity data type definitions.

### GET /data
Get entity data records.

**Query Parameters:**
- `data_type_id` — filter by data type
- `search` — search query
- `page` — pagination
- `limit` — page size

### GET /data-version
Check data version for sync.

---

## 10. Currency API

### GET /currency
List all currencies for current org.

**Response:**
```json
{
  "currencies": [
    { "id": "cur_id", "code": "IDR", "name": "Indonesian Rupiah", "symbol": "Rp" }
  ]
}
```

---

## 11. Custom Modules API

### GET /custom-modules
List custom modules for current org.

### GET /custom-module/{id}
Get custom module detail.

**Response:**
```json
{
  "id": "module_id",
  "name": "Module Name",
  "url": "https://custom.altius.example/module",
  "icon": "icon_name"
}
```

---

## 12. Location History API

### POST /location-history/bulk
Bulk upload location history records.

**Request:**
```json
{
  "records": [
    {
      "lat": -6.123456,
      "lng": 106.789012,
      "accuracy": 5.0,
      "timestamp": "2026-09-07T08:00:00Z",
      "connection_state": "wifi"
    }
  ]
}
```

---

## 13. Device Token API

### POST /device-token
Register or update FCM token.

**Request:**
```json
{
  "token": "fcm_token_value",
  "platform": "android",
  "device_id": "device_identifier"
}
```

---

## 14. Main Menu API

### GET /main-menu
Get main menu configuration for current user.

### GET /main-menu/custom-module
Get custom modules for main menu.

### GET /main-menu/setting
Get settings menu items.

### GET /main-menu/task-create
Get task creation options (available flows).

### GET /main-menu/task-list
Get task list for main menu.

---

## 15. Media Upload API

### POST /media/images
Upload image file.

**Request:** `multipart/form-data`
- `file` — image file

### POST /media/files
Upload generic file.

### POST /media/images/troubleshooting/
Upload troubleshooting screenshot.

### POST /compressed/
Upload compressed media (video compression result).

---

## 16. Troubleshooting API

### POST /troubleshooting
Submit troubleshooting report.

**Request:**
```json
{
  "api_connection": true,
  "download_connection": true,
  "upload_connection": true,
  "app_version": "0.1.0",
  "min_version": "1.35.0",
  "device_info": { ... },
  "screenshot": "image_url"
}
```

---

## 17. Privacy Policy API

### GET /privacy-policy/
Get privacy policy content.

**Response:**
```json
{
  "title": "Privacy Policy",
  "content": "HTML or markdown content",
  "version": "1.0"
}
```

---

## 18. Version API

### GET /version/
Check app version requirements.

**Response:**
```json
{
  "latest_version": "1.41.0",
  "min_version": "1.35.0",
  "update_required": false,
  "update_available": true
}
```

---

## 19. Keys API

### GET /keys
Get key-value configuration pairs.

**Response:**
```json
{
  "keys": [
    { "key": "config_name", "value": "config_value" }
  ]
}
```

---

## 20. Webhook API

### POST /to/
Send webhook data to configured target URL.

**Request:**
```json
{
  "task_id": "task_id",
  "page_id": "page_id",
  "data": { ... }
}
```

Page webhooks are configured per flow page. When a task page is completed, the webhook queue processes:
1. `AddPageWebhookDataToQueueUseCase` — queue webhook data
2. `SyncPageWebhookUseCase` — process queue, send to target URL
3. On failure — generate sync failure report

---

## 21. Maps API

### GET /maps/dir/2
Get map directions between two points.

**Query Parameters:**
- `origin` — `lat,lng`
- `destination` — `lat,lng`

---

## 22. WebView API

### GET /web-view
Get content for embedded WebView display.

Returns web content URL or HTML for in-app WebView rendering.

---

## API Authentication (production)

Production clients use **Keycloak OIDC** (not password OTP) as the primary gate:

1. Browser / app → Keycloak authorization code + PKCE (`altius-web` / `altius-mobile`)
2. Exchange → access token (+ refresh on mobile; secure storage)
3. `Authorization: Bearer <access_token>` on `/api/v3/*`
4. Optional: `GET /api/v3/auth/me` to hydrate org/hub/role

```
Authorization: Bearer {access_token}
```

OTP / cloud-authenticator routes elsewhere in this document are optional tenant
policy features and are secondary to Keycloak in the reference deployment.

Token refresh: mobile uses AppAuth refresh; web may re-enter PKCE (memory-held access tokens).

---

## API Error Handling

The API returns standard HTTP status codes:

| Status | Message Key | Description |
|---|---|---|
| 400 | `Bad Request` | Bad request |
| 401 | (auth guard) | Unauthorized — redirect to login |
| 402 | `Payment Required` | Payment required |
| 403 | (permission) | Forbidden — no permission |
| 404 | `Not Found` | Resource not found |
| 405 | `Method Not Allowed` | Method not allowed |
| 406 | `Not Acceptable` | Not acceptable |
| 409 | (conflict) | Duplicate/conflict |
| 410 | (gone) | Task removed by admin |
| 412 | `Precondition Failed` | Precondition failed |
| 413 | (payload too large) | Media too large |
| 416 | `Length Required` | Length required |
| 417 | `Expectation Failed` | Expectation failed |
| 422 | (validation) | Data validation error |
| 429 | (rate limit) | Too many requests |
| 500 | `Internal Server Error` | Server error |
| 502 | (bad gateway) | Bad gateway |
| 503 | `Service Unavailable` | Service unavailable |
| 504 | (timeout) | `connectApiTimeoutMessage` — "Connect API request timed out" |

### Error Response Format

```json
{
  "error": true,
  "message": "Error description",
  "code": "ERROR_CODE",
  "details": { ... }
}
```

---

## API Sync Strategy

The mobile app uses an **offline-first** strategy (see `WorkStore` + `SyncService`):

```
1. READ:  GET /tasks → upsert local SQLite (incl. stop_id)
2. WRITE: Append immutable outbox events locally
3. SYNC:  POST /events → match receipts by event_id / server_event_id
4. PULL:  GET /tasks again so the board matches server stage
```

Sync triggers:
- After login / Keycloak hydrate
- Connectivity restored / WorkManager background
- Manual sync
- After push of pending outbox

---

*End of API Reference*
