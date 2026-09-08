# Altius FTM — Tenant Model

> **Product:** Altius FTM (Fleet & Transport Management)
> **Date:** 2026-09-07  

---

## Table of Contents

1. [Tenant Hierarchy](#1-tenant-hierarchy)
2. [Build-Time Flavors](#2-build-time-flavors)
3. [Runtime Organization](#3-runtime-organization)
4. [Hub / Warehouse Context](#4-hub--warehouse-context)
5. [Permission & Role System](#5-permission--role-system)
6. [User Session & Local Data](#6-user-session--local-data)
7. [Special Tenant Behavior](#7-special-tenant-behavior)
8. [Tenant Isolation Mechanisms](#8-tenant-isolation-mechanisms)
9. [Evidence](#9-evidence)

---

## 1. Tenant Hierarchy

```
Build Flavor / Environment
  └── Runtime Organization
       └── Hub / Warehouse
            └── User Permissions & Roles
                 └── User Session & Local Data
```

Each level scopes data, configuration, and access:

| Level | Scope | Isolation |
|---|---|---|
| Build Flavor | Infrastructure / API endpoint | Compile-time (separate APK builds) |
| Organization | Business tenant | Runtime (switch with data clearing) |
| Hub | Physical location / warehouse | Runtime (select, geofence-bound) |
| Permission | User authorization | Per org + hub + role |
| Session | User identity & token | Per login, stored locally |

---

## 2. Build-Time Flavors

### 2.1 Flavor Inventory (8 flavors)

| Flavor | API Base | Web Origin | App Name | Special |
|---|---|---|---|---|
| **Prod** | `api.altius.example` | `app.altius.example` | Altius Field | Standard production |
| **Dev** | `api-dev.altius.example` | `app-dev.altius.example` | Altius Field Dev | Development |
| **Beta** | `api-beta.altius.example` | `app-beta.altius.example` | Altius Field Beta | Beta testing |
| **Sandbox** | `api-sandbox.altius.example` | `app-sandbox.altius.example` | Altius Field Sandbox | Testing/staging |
| **Unilever** | `api-enterprise.altius.example` | `app.altius.example` | Altius Field | Unilever-dedicated API |
| **Unilever Sandbox** | `api-sandbox.altius.example` | `app.altius.example` | Altius Field Sandbox | Unilever testing |
| **Y3** | `api.altius.example` | `app.altius.example` | Altius Field | Y3 branding (shared prod API) |
| **Y3 Sandbox** | `api-sandbox.altius.example` | `app-sandbox.altius.example` | Altius Field Sandbox | Y3 testing |

### 2.2 Flavor Configuration

Each flavor is defined by a `dotEnv*` file in `flutter_assets/envs/`:

```
envs/
├── dotEnvProd
├── dotEnvDev
├── dotEnvBeta
├── dotEnvSandbox
├── dotEnvUnilever
├── dotEnvUnileversandbox
├── dotEnvY3
└── dotEnvY3sandbox
```

Each file contains:
- `BASE_URL` — API endpoint
- `WEB_ORIGIN` — Web app origin
- `REGION_BASE_ENDPOINT` — S3 region
- `HMS_URL` — HMS e-learning URL
- `UNILEVER_BASE_URL` — Unilever integration URL
- `IS_DEBUG` — Debug flag
- (Plus sensitive integration keys — redacted)

### 2.3 Flavor Selection

Flavor is selected at **compile time**. The Flutter build system produces separate APKs per flavor. The installed APK determines which API endpoint the app communicates with.

Key observations:
- **Unilever** has a dedicated API (`api-enterprise.altius.example`) but shares the production web app
- **Y3** shares the production API and web app but has distinct branding (`y3_logo_purple2.png`)
- **Sandbox** flavors point to sandbox APIs for testing

---

## 3. Runtime Organization

### 3.1 Organization Concept

An **organization** is the primary business tenant. A single user may belong to multiple organizations. The user can switch between organizations at runtime.

### 3.2 Organization Data Model

```
OrganizationEntity
  ├── id: string
  ├── name: string
  ├── configuration: OrgConfigurationEntity
  │   ├── flow configurations
  │   ├── entity data versions
  │   ├── currency settings
  │   ├── custom modules
  │   └── webhook configurations
  └── is_avian: boolean (special flag for Avian brand)
```

### 3.3 Organization Switching

**Use Cases:**
- `GetOrganizationsUseCase` — fetch list of user's organizations
- `SwitchOrganizationUseCase` — switch to selected org (calls `POST /organization/switch`)
- `GetOrgIdFromLocalUseCase` — get current org ID from local storage
- `GetOrgConfigurationUseCase` — get org-specific configuration

**UI Flow:**
```
Settings → Change Organization
  → GET /organizations (list)
  → User selects org
  → POST /organization/switch
  → Clear local data (tasks, flows, currency, OTP cache)
  → Save new org ID
  → Fetch new org configuration
  → Re-sync all data for new org
  → Navigate to Hub Chooser
```

### 3.4 Organization-Specific Data

Each organization has its own:
- Flow definitions (task templates)
- Entity data (master data)
- Currency settings
- Custom modules
- Webhook configurations
- Permission configurations
- User roles

When switching organizations, all local data from the previous org is cleared to prevent cross-tenant data leakage.

---

## 4. Hub / Warehouse Context

### 4.1 Hub Concept

A **hub** is a physical location or warehouse within an organization. Each organization can have multiple hubs. A user is assigned to one or more hubs and must select a working hub before starting operations.

### 4.2 Hub Data Model

```
HubEntity
  ├── id: string
  ├── org_id: string (parent organization)
  ├── name: string
  ├── geo_lock: GeoLockEntity
  │   ├── latitude: double
  │   ├── longitude: double
  │   └── radius: double (meters)
  └── permissions: PermissionsCollection

GeoLockEntity
  ├── latitude: double
  ├── longitude: double
  └── radius: double (geofence radius in meters)
```

### 4.3 Hub Selection

**Use Cases:**
- `GetAllHubUseCase` — fetch all hubs for current org
- `GetHubUseCase` — get specific hub
- `GetHubIdFromLocalUseCase` — get current hub from local
- `SaveCurrentHubUseCase` — save selected hub to local
- `SyncHubsUseCase` — sync hubs from remote

**UI:** `HubChooserPage` — user selects from available hubs

### 4.4 Hub Geofencing

Check-in requires the user to be physically within the hub's geofence:

```
distance = haversine(user_location, hub.geo_lock)
if distance > hub.geo_lock.radius:
    show_error("You are {distance} meters away from {hubName}")
    block_check_in()
else:
    allow_check_in()
```

### 4.5 Hub Permissions

Each hub has its own permission set:
- `CheckInCheckOutMeta` — check-in/out metadata
- `AddTaskMeta` — task creation permissions
- `SyncHubPermissionRemoteUseCase` — sync permissions from server

---

## 5. Permission & Role System

### 5.1 Permission Data Model

```
PermissionEntity
  ├── id: string
  ├── name: string
  └── scope: org_id + hub_id

PermissionDetailEntity
  ├── can_check_in: boolean
  ├── can_add_task: boolean
  ├── can_do_task: boolean
  ├── can_view_entity_data: boolean
  └── ... (additional permissions)

RoleEntity
  ├── id: string
  ├── name: string
  └── permissions: List<PermissionDetailEntity>
```

### 5.2 Permission Use Cases

- `GetPermissionUseCase` — get current user's permissions
- `GetPermissionCheckInUseCase` — check if user can check in
- `SyncHubPermissionRemoteUseCase` — sync permissions from server

### 5.3 Permission Enforcement

Permissions are checked before operations:

| Operation | Permission Check | Error if Denied |
|---|---|---|
| Create task | `can_add_task` | "You don't have permission to add task, contact your admin to request access." |
| Execute task | `can_do_task` | "You don't have permission to do task, contact your admin to request access." |
| View entity data | `can_view_entity_data` | "You don't have permission to view data, contact your admin to request access." |
| Check in | `can_check_in` | (check-in blocked) |

### 5.4 Permission Scope

Permissions are scoped to:
```
Organization + Hub + User Role → Permission Set
```

A user may have different permissions in different hubs within the same organization.

---

## 6. User Session & Local Data

### 6.1 Session Data

Stored locally per session:
- `token` — JWT bearer token (via `SaveTokenToLocalUseCase`)
- `user_data` — user profile (via `SaveUserDataToLocalUseCase`)
- `org_id` — current organization (via `GetOrgIdFromLocalUseCase`)
- `hub_id` — current hub (via `GetHubIdFromLocalUseCase`)
- `fcm_token` — push notification token
- `check_in_time` — current check-in timestamp
- `is_avian` — Avian brand flag

### 6.2 Local Data Clearing on Org Switch

When switching organizations, the following local data is cleared:
- Task data (`RemoveLocalTaskDataUseCase`)
- Flow data (`FailedToClearFlowData`)
- Currency data (`FailedToClearCurrencyData`)
- OTP cache (`FailedToClearOtpCache`)
- Entity data (inferred)

This ensures no cross-tenant data leakage.

### 6.3 Session Lifecycle

```
Login → Save token + user data
  → Select org → Save org ID → Sync org data
  → Select hub → Save hub ID → Sync hub permissions
  → Check in → Start session (location tracking)
  → ... perform tasks ...
  → Check out → End session
  → Logout or switch org → Clear local data
```

---

## 7. Special Tenant Behavior

### 7.1 Unilever

- **Dedicated API:** `api-enterprise.altius.example` (production)
- **Dedicated base URL:** `tenant-a.altius.example` / `tenant-a-dev.altius.example`
- **Shared web app:** Uses `app.altius.example` (same as standard prod)
- **Sandbox:** Uses standard sandbox API (`api-sandbox.altius.example`)
- **Purpose:** Unilever-specific integrations and data isolation

### 7.2 Y3

- **Shared API:** Uses standard production API (`api.altius.example`)
- **Shared web app:** Uses `app.altius.example`
- **Distinct branding:** `y3_logo_purple2.png`, purple color scheme
- **Sandbox:** Uses standard sandbox API and web
- **Purpose:** Y3-branded variant of the standard product

### 7.3 Avian

- **Flag:** `is_avian` boolean in organization entity
- **Use case:** `GetIsAvianFromLocalUseCase`
- **Purpose:** Avian brand-specific behavior (inferred: different branding or flow configuration)

### 7.4 HMS (Road Hazard Awareness)

- **URL:** `lms.altius.example` (prod) / `lms-sandbox.altius.example` (sandbox)
- **Feature:** `road_warning` — displays HMS e-learning content for road hazard awareness
- **Integration:** External HMS LMS system for driver safety training

---

## 8. Tenant Isolation Mechanisms

### 8.1 Isolation Layers

| Layer | Mechanism | Scope |
|---|---|---|
| **Build Flavor** | Separate APK builds with different `BASE_URL` | Infrastructure isolation |
| **API Token** | JWT token contains org_id, scoped to organization | Auth-level isolation |
| **Organization Switch** | `POST /organization/switch` returns new token | Tenant boundary |
| **Local Data Clearing** | All org-specific data cleared on switch | Local storage isolation |
| **Hub Geofence** | Physical location validation | Location-based isolation |
| **Permission Scoping** | Permissions per org + hub + role | Authorization isolation |
| **Flow Configuration** | Each org has own flow definitions | Business logic isolation |
| **Entity Data** | Each org has own master data | Data isolation |

### 8.2 Data Isolation Matrix

```
                    Org A          Org B
                   ┌──────┐       ┌──────┐
                   │ Hub 1│       │ Hub 3│
                   │ Hub 2│       │ Hub 4│
                   └──┬───┘       └──┬───┘
                      │              │
              ┌───────┴──────┐  ┌────┴───────┐
              │ User A (RW)  │  │ User C (RW)│
              │ User B (R)   │  │ User D (R) │
              └──────────────┘  └────────────┘

- User A cannot see Org B data
- User A can see Hub 1 + Hub 2 data (within Org A)
- User B has read-only access to Org A hubs
- Switching to Org B clears all Org A local data
```

### 8.3 Cross-Tenant Prevention

1. **Token scoping:** JWT token is issued for a specific org. API returns 403 if accessing other org's data.
2. **Local data clearing:** `RemoveLocalTaskDataUseCase` + flow/currency/OTP clearing on org switch
3. **Permission check:** Every operation validates user's permission for current org+hub
4. **Geofence validation:** Check-in requires physical presence at hub location

---

## 9. Evidence

### 9.1 Confirmed Evidence

| Evidence | Source |
|---|---|
| 8 build flavors | `envs/dotEnv*` files |
| Organization switching | `ChangeOrganizationPage`, `SwitchOrganizationUseCase`, `POST /organization/switch` |
| Hub selection | `HubChooserPage`, `GetAllHubUseCase`, `SaveCurrentHubUseCase` |
| Hub geofencing | `GeoLockEntity`, distance error messages |
| Permission system | `PermissionEntity`, `PermissionDetailEntity`, `RoleEntity` |
| Permission enforcement | `noAddTaskPermission`, `noDoTaskPermission`, `noViewEntityDataPermission` messages |
| Local data clearing | `RemoveLocalTaskDataUseCase`, `FailedToClearFlowData`, `FailedToClearCurrencyData` |
| Unilever flavor | `dotEnvUnilever`, `UNILEVER_BASE_URL`, `api-enterprise.altius.example` |
| Y3 flavor | `dotEnvY3`, `y3_logo_purple2.png` |
| Avian flag | `GetIsAvianFromLocalUseCase`, `is_avian` field |
| HMS integration | `HMS_URL` in env files, `road_warning` feature |

### 9.2 Inferred

| Inference | Basis |
|---|---|
| Organization-specific flows | Org configuration entity + flow version sync |
| Organization-specific entity data | Entity data sync per org |
| Hub-specific permissions | `SyncHubPermissionRemoteUseCase`, `CheckInCheckOutMeta` |
| Cross-tenant data isolation via token | Standard JWT practice + org switch returns new token |

---

*End of Tenant Model*
