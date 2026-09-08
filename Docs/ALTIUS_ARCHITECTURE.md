# Altius — Field Task Manager (FTM)
## Comprehensive Technical Architecture Documentation

> **Document Status:** Product architecture  
> **Product:** Altius FTM (Fleet & Transport Management)  
> **Package:** `com.altius.altius_field`  
> **Brand:** Altius  
> **Date:** 2026-09-08  
> **Backend:** [`ALTIUS_BACKEND_ARCHITECTURE.md`](./ALTIUS_BACKEND_ARCHITECTURE.md) · [`ALTIUS_DATABASE_DESIGN.md`](./ALTIUS_DATABASE_DESIGN.md)  

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [High-Level Architecture](#2-high-level-architecture)
3. [Backend Structure](#3-backend-structure)
4. [Low-Level Architecture](#4-low-level-architecture)
5. [Frontend Components](#5-frontend-components)
6. [Database Schema](#6-database-schema)
7. [API Endpoint Structure](#7-api-endpoint-structure)
8. [User Journey](#8-user-journey)
9. [User Flow](#9-user-flow)
10. [Business Logic](#10-business-logic)
11. [End-to-End Flow](#11-end-to-end-flow)
12. [Pseudocode](#12-pseudocode)
13. [Tenant Model](#13-tenant-model)
14. [Mobile & Web Connectivity](#14-mobile--web-connectivity)
15. [UI Breakdown](#15-ui-breakdown)
16. [Security & Permissions](#16-security--permissions)
17. [Environment Configuration](#17-environment-configuration)
18. [Evidence Classification](#18-evidence-classification)

---

## 1. Executive Summary

Altius FTM is a **Flutter-based field operations platform** developed by Altius (PT Antero Daemon Technologies). It enables organizations to manage fleet and transport field work: task assignment, check-in/check-out with geofencing, route planning, multi-step stop execution, offline-first synchronization, and GPS anomaly review.

The mobile client follows **Clean Architecture** (domain / data / presentation) with **Cubit (Bloc)** state management. The operations web app is Next.js; the API is Rust (Axum) with Keycloak and PostgreSQL.

| Attribute | Value |
|---|---|
| Package | `com.altius.altius_field` |
| Product line | Altius FTM — Fleet & Transport Management |
| Mobile | Flutter (offline-first SQLite + outbox) |
| Web | Next.js 15 (Vercel) |
| API | Rust / Axum `/api/v3` (VPS) |
| Identity | Keycloak OIDC (PKCE) |
| Primary store | PostgreSQL |
| Languages | 7 (en, id, th, ja, zh, fil-PH, vi) |
| Architecture Pattern | Clean Architecture + Cubit (mobile) |
| Local DB | SQLite (drift/sqflite) + Hive |
| Maps | Google Maps Flutter |
| Camera | Flutter Camera plugin |
| Barcode | ML Kit Barcode Scanning |
| Notifications | Firebase Cloud Messaging (FCM) |
| Crashlytics | Sentry + Firebase Crashlytics |
| Languages | 7 (en, id, th, ja, zh, fil-PH, vi) |

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        ALTius FTM ECOSYSTEM                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                     │
│  ┌──────────────────┐    QR Login    ┌──────────────────┐          │
│  │  Altius Mobile   │◄──────────────►│  Altius Web      │          │
│  │  (Flutter)       │                │  (app.altius.example)  │          │
│  │  Field Worker    │                │  Admin Dashboard │          │
│  └────────┬─────────┘                └────────┬─────────┘          │
│           │                                    │                    │
│           │     Shared REST API (/api/v3)      │                    │
│           │                                    │                    │
│           ▼                                    ▼                    │
│  ┌─────────────────────────────────────────────────────────┐       │
│  │              Backend API (api.altius.example)               │       │
│  │              Shared database, shared auth                │       │
│  └─────────────────────────────────────────────────────────┘       │
│           │                                                          │
│           ▼                                                          │
│  ┌─────────────────────────────────────────────────────────┐       │
│  │              AWS S3 (ap-southeast-1)                     │       │
│  │              Media storage (photos, videos)              │       │
│  └─────────────────────────────────────────────────────────┘       │
│                                                                     │
└─────────────────────────────────────────────────────────────────────┘
```

### System Components

```
┌──────────────────────────────────────────────────────────────────┐
│                    MOBILE APPLICATION                             │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Presentation Layer (Cubits + Pages + Widgets)             │  │
│  │  ├── Auth (Login, OTP, CloudAuth, ForgotPassword)          │  │
│  │  ├── Task (List, Detail, Doing, Create, Map, History)      │  │
│  │  ├── CheckIn/Out (Geofence, GeoLock)                       │  │
│  │  ├── Settings (Profile, QR, OrgSwitch, HubChooser)         │  │
│  │  ├── Components (13 types: photo, video, voice, etc.)      │  │
│  │  └── Support (Troubleshooting, RoadWarning, WebView)       │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │  Domain Layer (Entities + Use Cases + Repositories)        │  │
│  │  ├── 60+ domain entities                                    │  │
│  │  ├── 114 use cases                                          │  │
│  │  └── 24 repository contracts                                │  │
│  ├────────────────────────────────────────────────────────────┤  │
│  │  Data Layer (Local DS + Remote DS + Repository Impl)       │  │
│  │  ├── Local: SQLite, Hive, SharedPreferences                │  │
│  │  ├── Remote: REST API (Dio HTTP client)                    │  │
│  │  └── Models: Freezed + json_serializable                   │  │
│  └────────────────────────────────────────────────────────────┘  │
│  ┌────────────────────────────────────────────────────────────┐  │
│  │  Native Layer                                               │  │
│  │  ├── libapp.so (Dart AOT, ~16.8 MB)                        │  │
│  │  ├── libflutter.so (Flutter engine)                        │  │
│  │  ├── libtoolChecker.so (root/mock detection)               │  │
│  │  ├── libapi_util.so (API utility)                          │  │
│  │  ├── libbarhopper_v3.so (ML Kit barcode)                   │  │
│  │  └── libdatastore_shared_counter.so (DataStore)            │  │
│  └────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 3. Backend Structure

### 3.1 API Infrastructure

The backend is a RESTful API served at `/api/v3` with environment-specific hosts:

| Flavor | API Host | Web Origin |
|---|---|---|
| Production | `api.altius.example` | `app.altius.example` |
| Development | `api-dev.altius.example` | `app-dev.altius.example` |
| Beta | `api-beta.altius.example` | `app-beta.altius.example` |
| Sandbox | `api-sandbox.altius.example` | `app-sandbox.altius.example` |
| Unilever | `api-enterprise.altius.example` | `app.altius.example` |
| Unilever Sandbox | `api-sandbox.altius.example` | `app.altius.example` |
| Y3 | `api.altius.example` | `app.altius.example` |
| Y3 Sandbox | `api-sandbox.altius.example` | `app-sandbox.altius.example` |

### 3.2 Backend Service Areas

The backend (`backend/`) provides these service areas:

```
Backend API (/api/v3)
├── /auth                    — Authentication (login, MFA, token)
├── /user                    — User management
│   ├── /user/forgot-password
│   └── /user/change-password
├── /organizations           — Organization listing & switching
│   └── /organization/switch
├── /otp                     — OTP service
│   ├── /otp/send
│   └── /otp/verify
├── /cloud-authenticator     — TOTP authenticator
│   └── /cloud-authenticator/verify
├── /tasks                   — Task management
│   ├── /tasks/bulk
│   ├── /task/{id}
│   ├── /task/photo/
│   ├── /task/video/
│   └── /task-create
├── /flows                   — Flow configuration
│   ├── /flow/
│   └── /flows/check-version
├── /data                    — Data source & entity data
│   ├── /data-source-list
│   ├── /data-type-list
│   ├── /data-types
│   └── /data-version
├── /currency                — Currency management
├── /custom-modules          — Custom module management
├── /keys                    — Key-value configuration
├── /version/                — App version checking
├── /location-history        — Location tracking
│   └── /location-history/bulk
├── /device-token            — FCM token registration
├── /troubleshooting         — Diagnostic upload
│   └── /media/images/troubleshooting/
├── /main-menu               — Main menu configuration
│   ├── /main-menu/custom-module
│   ├── /main-menu/setting
│   ├── /main-menu/task-create
│   └── /main-menu/task-list
├── /privacy-policy/         — Privacy policy content
├── /maps/dir/2              — Map directions
├── /compressed/             — Compressed media upload
├── /to/                     — Webhook targets
└── /web-view                — WebView content
```

### 3.3 Media Storage

Media files (photos, videos) are uploaded to AWS S3 in the `ap-southeast-1` region. The `REGION_BASE_ENDPOINT` is `s3-ap-southeast-1.amazonaws.com`.

Upload flow:
1. Mobile captures photo/video
2. Video compression via `video_compression_service.dart`
3. Upload to S3 via pre-signed URL or direct upload
4. S3 URL stored in task data and synced to backend

### 3.4 Notification Infrastructure

- **FCM (Firebase Cloud Messaging):** Push notifications for task assignment, task status changes
- **Telegram webhook:** `post_notification_data_log_to_telegram_use_case`, `post_task_data_log_to_telegram_use_case` — diagnostic notifications sent to Telegram
- **Discord webhook:** Configured in environment files (URL redacted)
- **Foreground service:** `foreground_service.dart` — persistent notification for location tracking during check-in

### 3.5 Third-Party Integrations

| Service | Purpose |
|---|---|
| Sentry | Crash reporting |
| Firebase | FCM, Crashlytics |
| Luxand | Face recognition (liveness check) |
| HMS LMS | Road hazard awareness e-license |
| freeRASP | App security/tamper detection |
| Google Maps | Navigation, geofencing |
| ML Kit | Barcode scanning |

---

## 4. Low-Level Architecture

### 4.1 Clean Architecture Layer Structure

Each feature follows this structure:

```
features/{feature_name}/
├── data/
│   ├── local/
│   │   └── data_sources/
│   │       └── {feature}_local_data_source.dart
│   ├── remote/
│   │   ├── data_sources/
│   │   │   └── {feature}_remote_data_source.dart
│   │   └── models/
│   │       ├── {feature}_remote_model.dart
│   │       ├── {feature}_remote_model.freezed.dart
│   │       └── {feature}_remote_model.g.dart
│   └── repositories/
│       └── {feature}_repository_impl.dart
├── domain/
│   ├── entities/
│   │   └── {feature}_entity.dart
│   ├── repositories/
│   │   └── {feature}_repository.dart
│   └── use_cases/
│       └── {feature}_use_case.dart
└── presentation/
    ├── manager/
    │   ├── {feature}_cubit.dart
    │   ├── {feature}_cubit.freezed.dart
    │   └── {feature}_state.dart
    ├── pages/
    │   └── {feature}_page.dart
    └── widgets/
        └── {feature}_widget.dart
```

### 4.2 Feature Inventory (29 features)

| # | Feature | Description |
|---|---|---|
| 1 | `change_organization` | Runtime org switching |
| 2 | `change_password` | Password change (expired password) |
| 3 | `check_in_check_out` | Geofenced check-in/out (start/end trip) |
| 4 | `cloud_authenticator` | TOTP authenticator (MFA) |
| 5 | `currency` | Currency master data sync |
| 6 | `data_source_feature` | Data source & entity data management |
| 7 | `entity_data` | Entity data CRUD (shared) |
| 8 | `forget_password` | Password recovery |
| 9 | `history` | Sync failure history & reports |
| 10 | `home` | App version check & data version sync |
| 11 | `hub_chooser` | Hub/warehouse selection (shared) |
| 12 | `location_history_service` | GPS trail recording & sync |
| 13 | `login` | Authentication (email, MFA, org config) |
| 14 | `mock_gps_app_detected` | Fake GPS detection |
| 15 | `more_menu` | Custom modules menu |
| 16 | `otp` | OTP verification |
| 17 | `permissions` | Role-based permissions (shared) |
| 18 | `privacy_policy` | Privacy policy display |
| 19 | `road_warning` | HMS road hazard awareness |
| 20 | `route_optimization` | Visit reorder by nearest location |
| 21 | `settings` | Profile, QR login, check-out, settings |
| 22 | `shared` | Cross-cutting: auth, sync, task entities |
| 23 | `sync_data` | Data synchronization (shared) |
| 24 | `task_activity_workflow` | Task step order validation |
| 25 | `task_create` | Flow selection & task creation |
| 26 | `task_doing` | Multi-component task execution engine |
| 27 | `task_list` | Task list (ongoing/done) with sync |
| 28 | `task_map` | Google Maps task navigation |
| 29 | `troubleshooting` | Device health diagnostics |
| 30 | `web_view` | Embedded web content viewer |

### 4.3 Data Sources

**Local Data Sources (15):**

| Feature | Local DS | Storage |
|---|---|---|
| check_in_check_out | `check_in_out_local_data_source` | SQLite |
| history | `history_local_data_source` | SQLite |
| home | `home_local_data_source` | SQLite |
| location_history | `location_history_data_source_local` | SQLite |
| login | `login_local_data_source` | SQLite + SharedPreferences |
| otp | `otp_local_data_source` | SQLite (cache) |
| task_create | `task_create_data_source` | SQLite |
| task_doing | `task_doing_local_data_source` | SQLite |
| task_list | `task_list_local_data_source` | SQLite |
| troubleshooting | `troubleshooting_local_data_source` | SQLite |
| entity_data | `entity_data_local_data_source` | SQLite |
| shared | `shared_local_data_source` | SQLite + Hive |

**Remote Data Sources (9):**

| Feature | Remote DS |
|---|---|
| forget_password | `forget_password_remote_ds` |
| home | `home_remote_data_source` |
| location_history | `location_history_data_source_remote` |
| login | `login_remote_data_source` |
| otp | `otp_remote_data_source` |
| task_doing | `task_doing_remote_ds` |
| task_list | `task_list_remote_data_source` |
| troubleshooting | `troubleshooting_remote_data_source` |
| shared | `shared_remote_data_source` |

### 4.4 Repository Implementations (24)

Each repository implements its domain contract with local-first strategy:

```
RepositoryImpl
  ├── getFromLocal() → LocalDataSource
  ├── getFromRemote() → RemoteDataSource
  ├── saveToLocal() → LocalDataSource
  └── syncRemoteToLocal() → RemoteDataSource → LocalDataSource
```

### 4.5 Domain Entities (60+)

| Category | Entities |
|---|---|
| **Auth** | AdminUserEntity, UserDataEntity, LoginDataEntity, MfaResponseEntity, MfaVerificationParam, LogInParamEntity, CountryEntity |
| **Organization** | OrganizationEntity, OrgConfigurationEntity, RoleEntity, PermissionDetailEntity |
| **Hub** | HubEntity, GeoLockEntity, InLocationEntity |
| **Task** | TaskEntity, CurrentDestinationTaskEntity, TaskWorkflowEntity, FlowEntity, FlowConfigurationEntity, PageEntity, ComponentEntity, PageParamEntity |
| **CheckIn** | CheckInOutEntity, RemoteCheckInStatusEntity |
| **Location** | LocationHistoryEntity, ConnectionEntity, UserLocation, MapBounds |
| **Data** | EntityDataListEntity, DataTypeFieldEntity, EntityDataEntity, EntityDataTypeEntity, SimpleEntityDataEntity, CurrencyEntity |
| **Webhook** | PageWebhookConfigurationEntity, PageWebhookDataEntity, PageWebhookQueueDataEntity, PageWebhookSyncResultEntity |
| **Sync Failure** | BaseSyncFailureReportEntity, InitialSyncFailureReportEntity, ItemDetailSyncFailureReportEntity, OtherSyncFailureReportEntity, SyncFailureReportResultEntity, TaskStatusUpdatedReportEntity, TaskSyncFailedReportEntity |
| **Other** | VersionUpdateDataEntity, PrivacyPolicyEntity, RoadHazardEntity, CustomModuleEntity, OtpDataEntity, RecoveryPasswordEntity, ChangePasswordEntity, AssignedVehicleEntity, KeyEntity, ApiResponseEntity |

### 4.6 Use Cases (114)

Use cases follow naming convention `{Verb}{Noun}UseCase`:

**Auth & Session:**
- LoginUseCase, LoginWithTokenUseCase, LoginWithTokenThenSaveToLocalUseCase
- LogoutUseCase, ForceLogoutUseCase
- SaveTokenToLocalUseCase, GetTokenFromLocalUseCase, SyncKeyUseCase
- SaveUserDataToLocalUseCase, GetUserDataLocalUseCase
- GetUserEmailFromLocalUseCase, GetUserNameFromLocalUseCase
- RegisterDeviceTokenUseCase, SyncFcmTokenUseCase
- GetFcmTokenFromLocalUseCase, SaveFcmTokenToLocalUseCase

**Organization & Hub:**
- GetOrganizationsUseCase, SwitchOrganizationUseCase, GetOrgIdFromLocalUseCase
- GetOrgConfigurationUseCase, GetIsAvianFromLocalUseCase
- GetHubUseCase, GetAllHubUseCase, GetHubIdFromLocalUseCase, SaveCurrentHubUseCase
- SyncHubPermissionRemoteUseCase, SyncHubsUseCase

**Task:**
- GetTaskListLocalUseCase, GetTaskListRemoteUseCase, UpdateTaskBulkUseCase
- GetTaskDetailUseCase, GetDetailTask, GetDetailFlow
- SaveTaskData, RemoveLocalTaskDataUseCase, RemoveUnusedFilesUseCase
- UpdateLocalTaskFieldsUseCase, UpdateLastDoneOrderUseCase
- ValidateTaskStepOrderUseCase, ValidateFlowUseCase
- GetListFlowUseCase, GetLocalFlowDataByIdUseCase

**CheckIn/Out:**
- IsCheckedInUseCase, GetCheckInTimeFromLocalUseCase, SaveCheckinTimeToLocalUseCase
- SyncRemoteCheckInToLocalUseCase, GetPermissionCheckInUseCase
- SaveInLocationDataUseCase, SaveOutLocationDataUseCase
- CheckAlreadyStartTripUseCase, CheckEndTripNotifReminderUseCase

**Sync:**
- SyncVersionDataUseCase, SyncCurrencyUseCase, SyncEntityDataUseCase
- SyncPageWebhookUseCase, SyncFcmTokenUseCase, SyncHubsUseCase
- GenerateSyncFailureReportUseCase

**Home & Version:**
- IsRequireToUpdateAppUseCase, GetAppUpdateAvailableFlagUseCase
- GetLastMinVersionUpdateUseCase, SaveAppUpdateAvailableFlagUseCase
- SetVersionDataUseCase, IsPasswordExpiredUseCase

**Other:**
- RecoveryPasswordUseCase, GetCurrencyCodeUseCase
- GetDataSourceConfigUseCase, GetAllEntityLocalUseCase, GetAllEntityRemoteUseCase
- StreamGetEntityRemoteUseCase, GetCameraInformationModelUseCase
- GetPageWebhookConfigurationUseCase, AddPageWebhookDataToQueueUseCase
- GetResultFromApiUseCase, GetHubSettingsUseCase
- SendOtpUseCase, VerifyOtpUseCase
- CheckApiConnectionUseCase, CheckDownloadConnectionUseCase, CheckUploadConnectionUseCase
- IsUsingLatestAppVersionUseCase
- PostNotificationDataLogToTelegramUseCase, PostTaskDataLogToTelegramUseCase
- ClearKeyUseCase, GetKeyUseCase
- UpdateTrackingSettingUseCase

### 4.7 Native Libraries

| Library | Size | Purpose |
|---|---|---|
| `libapp.so` | ~16.8 MB | Dart AOT compiled application code |
| `libflutter.so` | ~6 MB | Flutter engine |
| `libtoolChecker.so` | small | Root/mock GPS detection |
| `libapi_util.so` | small | API utility (native HTTP/crypto) |
| `libbarhopper_v3.so` | ~2 MB | ML Kit barcode scanning |
| `libdatastore_shared_counter.so` | tiny | DataStore shared preferences counter |
| `libimage_processing_util_jni.so` | small | Image processing JNI |
| `libsurface_util_jni.so` | small | Surface rendering JNI |

---

## 5. Frontend Components

### 5.1 Pages (40+)

See [ALTIUS_UI_BREAKDOWN.md](./ALTIUS_UI_BREAKDOWN.md) for the complete UI breakdown.

### 5.2 Task Component Engine (13 component types)

The task doing engine renders pages composed of 13 component types:

| Component | Pages | Widgets |
|---|---|---|
| photo | CameraPage, GalleryPage, ComponentPhotoPage | PhotoGridView, CategoriesDropdown, FooterButton |
| video | VideoCameraPage, VideoGalleryPage, ComponentVideoPage | ControlsOverlayVideo |
| input | ComponentInput | InputTextCurrency, InputTextDate, InputTextEntityData, InputTextURL |
| select | (inline) | DropdownWidget, MultiSelectWidget, LabeledCheckBoxWidget, SearchWidget |
| list | (inline) | ListCaCubit, ListQtyCounterModel |
| bill | BillPage | BillBody, BillItemModel, BillSummary, BillList |
| scan_display | ComponentScanDisplay | ScanButtonWidget |
| otp | ComponentOtpPage | OTPInputFieldWidget |
| capture | ComponentCapturePage | CaptureWidget |
| voice | ComponentVoicePage | PlayRecord, SoundRecord |
| print | ComponentPrint | (print dialog) |
| view | ComponentViewPage, ImageFullScreen, PDFFullScreen, VideoFullScreen | ComponentViewDefault, ComponentViewCurrency, ViewUrl, ErrorPreviewWidget |
| subpage | SubpageFormPage, SubpageListPage | SubpageEntryCard, SubpageListContent |

### 5.3 Core Widgets (reusable)

| Widget | Path |
|---|---|
| ButtonDefault | `core/widgets/button_default.dart` |
| CustomButton | `core/widgets/custom_button.dart` |
| CustomButtonDialog | `core/widgets/custom_button_dialog.dart` |
| DefaultText | `core/widgets/default_text.dart` |
| DropdownDefault | `core/widgets/dropdown_default.dart` |
| ListDefault | `core/widgets/list_default.dart` |
| SearchBox | `core/widgets/search_box.dart` |
| ProgressIndicatorDefault | `core/widgets/progress_indicator_default.dart` |
| SeparatorDefault | `core/widgets/separator_default.dart` |
| TextFormFieldCustom | `core/widgets/text_field/text_form_field_custom.dart` |
| TextFormFieldDefault | `core/widgets/text_field/text_form_field_default.dart` |
| TextFormFieldPasswordDefault | `core/widgets/text_field/text_form_field_password_default.dart` |
| BottomSheetHelper | `core/widgets/sheet_helper/bottom_sheet_helper.dart` |
| LocationDisclosureDialogContent | `core/widgets/location_disclosure_dialog_content.dart` |
| LeadingEllipsisText | `core/widgets/leading_ellipsis_text.dart` |
| GutterScrollViewWidget | `core/widgets/gutter_scroll_view_widget.dart` |

### 5.4 State Management (30+ Cubits)

Each feature has a Cubit + State pair. See [ALTIUS_UI_BREAKDOWN.md](./ALTIUS_UI_BREAKDOWN.md) for the full list.

---

## 6. Database Schema

### 6.1 Local Database (SQLite)

The application uses a local SQLite database for offline-first operation. Based on entity analysis, the following tables are inferred:

```
┌─────────────────────────────────────────────────────────────┐
│                    LOCAL DATABASE (SQLite)                   │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  users                                                      │
│  ├── id (PK)                                                │
│  ├── email                                                  │
│  ├── name                                                   │
│  ├── org_id (FK)                                            │
│  ├── hub_id (FK)                                            │
│  ├── role_id (FK)                                           │
│  └── permissions (JSON)                                     │
│                                                             │
│  organizations                                              │
│  ├── id (PK)                                                │
│  ├── name                                                   │
│  ├── configuration (JSON)                                   │
│  └── is_avian (boolean)                                     │
│                                                             │
│  hubs                                                       │
│  ├── id (PK)                                                │
│  ├── org_id (FK)                                            │
│  ├── name                                                   │
│  ├── geo_lock_lat (real)                                    │
│  ├── geo_lock_lng (real)                                    │
│  ├── geo_lock_radius (real)                                 │
│  └── permissions (JSON)                                     │
│                                                             │
│  tasks                                                      │
│  ├── id (PK)                                                │
│  ├── remote_id                                              │
│  ├── flow_id (FK)                                           │
│  ├── hub_id (FK)                                            │
│  ├── assignee_id (FK)                                       │
│  ├── status (enum: pending, ongoing, done, synced)          │
│  ├── title                                                  │
│  ├── data (JSON: page results)                              │
│  ├── created_at                                             │
│  ├── updated_at                                             │
│  ├── done_order (int)                                       │
│  └── is_synced (boolean)                                    │
│                                                             │
│  flows                                                      │
│  ├── id (PK)                                                │
│  ├── name                                                   │
│  ├── pages (JSON: page definitions)                         │
│  ├── configuration (JSON)                                   │
│  └── version (int)                                          │
│                                                             │
│  flow_versions                                              │
│  ├── flow_id (FK)                                           │
│  └── version (int)                                          │
│                                                             │
│  check_in_out                                               │
│  ├── id (PK)                                                │
│  ├── hub_id (FK)                                            │
│  ├── user_id (FK)                                           │
│  ├── check_in_time (datetime)                               │
│  ├── check_out_time (datetime)                              │
│  ├── in_location (JSON)                                     │
│  ├── out_location (JSON)                                    │
│  └── info (text)                                            │
│                                                             │
│  location_history                                           │
│  ├── id (PK)                                                │
│  ├── lat (real)                                             │
│  ├── lng (real)                                             │
│  ├── accuracy (real)                                        │
│  ├── timestamp (datetime)                                   │
│  ├── connection_state (text)                                │
│  └── is_synced (boolean)                                    │
│                                                             │
│  entity_data                                                │
│  ├── id (PK)                                                │
│  ├── data_type_id (FK)                                      │
│  ├── name                                                   │
│  ├── fields (JSON)                                          │
│  └── is_synced (boolean)                                    │
│                                                             │
│  data_types                                                 │
│  ├── id (PK)                                                │
│  ├── name                                                   │
│  ├── fields (JSON: field definitions)                       │
│  └── version (int)                                          │
│                                                             │
│  currencies                                                 │
│  ├── id (PK)                                                │
│  ├── code (text)                                            │
│  ├── name (text)                                            │
│  └── symbol (text)                                          │
│                                                             │
│  keys                                                       │
│  ├── key (PK)                                               │
│  └── value (text)                                           │
│                                                             │
│  page_webhook_queue                                         │
│  ├── id (PK)                                                │
│  ├── task_id (FK)                                           │
│  ├── page_id (FK)                                           │
│  ├── webhook_url (text)                                     │
│  ├── data (JSON)                                            │
│  ├── status (enum: pending, processing, done, failed)       │
│  └── attempts (int)                                         │
│                                                             │
│  sync_failure_reports                                       │
│  ├── id (PK)                                                │
│  ├── type (enum: task_data, upload, page_webhook, ...)      │
│  ├── task_id (FK)                                           │
│  ├── detail (JSON)                                          │
│  └── created_at (datetime)                                  │
│                                                             │
│  custom_modules                                             │
│  ├── id (PK)                                                │
│  ├── org_id (FK)                                            │
│  ├── name                                                   │
│  └── url (text)                                             │
│                                                             │
│  assigned_vehicles                                          │
│  ├── id (PK)                                                │
│  ├── user_id (FK)                                           │
│  ├── vehicle_id                                             │
│  └── vehicle_name                                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### 6.2 Remote Database (Inferred)

The PostgreSQL schema (see ALTIUS_DATABASE_DESIGN.md) contains:

- `users` — global user accounts
- `organizations` — tenant organizations
- `hubs` — warehouses/locations per org
- `flows` — task templates per org
- `tasks` — task instances
- `task_results` — submitted task data
- `entity_data` — master data per org
- `data_types` — data type definitions per org
- `currencies` — currency master
- `permissions` — role-based permissions
- `roles` — user roles per org
- `location_history` — GPS trails
- `page_webhooks` — webhook configurations
- `custom_modules` — org-specific modules
- `media` — S3 media references
- `device_tokens` — FCM tokens
- `app_versions` — version metadata

---

## 7. API Endpoint Structure

See [ALTIUS_API_REFERENCE.md](./ALTIUS_API_REFERENCE.md) for the complete API endpoint reference.

---

## 8. User Journey

### 8.1 Field Worker Journey (Mobile)

```
┌─────────┐     ┌─────────┐     ┌─────────┐     ┌─────────┐
│ Install │────►│  Login  │────►│  OTP/   │────►│ Select  │
│  App    │     │ (email) │     │  MFA    │     │   Org   │
└─────────┘     └─────────┘     └─────────┘     └────┬────┘
                                                     │
                    ┌─────────┐     ┌─────────┐      │
                    │ Check   │◄────│ Select  │◄─────┘
                    │  In     │     │  Hub    │
                    │(geofence│     └─────────┘
                    │  check) │
                    └────┬────┘
                         │
              ┌──────────┴──────────┐
              │                     │
              ▼                     ▼
        ┌──────────┐         ┌──────────┐
        │  View    │         │  Create  │
        │Task List │         │  Task    │
        │(Ongoing) │         │(if perm) │
        └────┬─────┘         └──────────┘
             │
             ▼
        ┌──────────┐
        │  Execute │
        │  Task    │
        │(multi-   │
        │component)│
        └────┬─────┘
             │
             ▼
        ┌──────────┐
        │  Sync    │
        │  to      │
        │  Server  │
        └────┬─────┘
             │
             ▼
        ┌──────────┐
        │  Check   │
        │  Out     │
        │(end trip)│
        └──────────┘
```

### 8.2 Admin Journey (Web — Inferred)

```
Login → Dashboard → Manage Orgs → Manage Hubs → Manage Users
  → Create Flows → Assign Tasks → Monitor Field Workers
  → View Reports → Export Data
```

---

## 9. User Flow

### 9.1 Authentication Flow

```
1. User enters email + password
2. App calls POST /auth (login)
3. Backend returns:
   a. If MFA required → MfaResponseEntity (OTP or TOTP)
   b. If password expired → redirect to /change-password
   c. If success → token + user data
4. If MFA:
   a. OTP: POST /otp/send → user enters code → POST /otp/verify
   b. TOTP: POST /cloud-authenticator/verify
5. On success:
   a. Save token to local (SaveTokenToLocalUseCase)
   b. Save user data to local (SaveUserDataToLocalUseCase)
   c. Register FCM token (RegisterDeviceTokenUseCase)
   d. Fetch organizations (GetOrganizationsUseCase)
   e. Fetch org configuration (GetOrgConfigurationUseCase)
   f. Sync version data (SyncVersionDataUseCase)
6. Navigate to org selection or main menu
```

### 9.2 Check-In Flow

```
1. User taps "Start Trip" / Check-In
2. App checks GPS enabled + permission granted
3. App gets current location
4. App validates geofence:
   a. Get hub geo_lock (lat, lng, radius)
   b. Calculate distance from user to hub
   c. If distance > radius → show error "You are {distance} meters away from {hubName}"
   d. If distance <= radius → proceed
5. App saves check-in:
   a. SaveCheckinTimeToLocalUseCase
   b. SaveInLocationDataUseCase
   c. SyncRemoteCheckInToLocalUseCase
6. Start foreground service (location tracking)
7. Start location history recording
8. Navigate to task list
```

### 9.3 Task Execution Flow

```
1. User selects task from Ongoing list
2. App loads task detail (GetDetailTask, GetDetailFlow)
3. App validates task step order (ValidateTaskStepOrderUseCase)
4. App renders task pages sequentially:
   For each page in flow:
     a. Display page title (PageTitle widget)
     b. Render components for this page
     c. For each component:
        - photo: Camera/Gallery → compress → save
        - video: Record/Select → compress → save
        - input: Text/Currency/Date/URL/EntityData → validate → save
        - select: Dropdown/MultiSelect/Checkbox → save
        - list: Add items with qty → save
        - bill: Add items + costs + summary → save
        - scan_display: Scan QR/barcode → display result → save
        - otp: Send OTP to recipient → verify → save
        - capture: Signature/capture → save
        - voice: Record voice → save
        - print: Print receipt/label
        - view: Display read-only content (image/PDF/video/URL)
        - subpage: Nested form/list → save
     d. Validate required fields
     e. Save page data (SaveTaskData)
     f. Trigger page webhook if configured (AddPageWebhookDataToQueueUseCase)
     g. Navigate to next page
5. On all pages complete:
   a. Update task status to done
   b. Sync to server (sync task data + media)
   c. Update done order (UpdateLastDoneOrderUseCase)
6. Return to task list
```

### 9.4 Sync Flow

```
1. Triggered by:
   a. Pull-to-refresh on task list
   b. After task completion
   c. After check-in/check-out
   d. App foreground (LifecycleWatcherState)
   e. Manual sync from settings
2. Sync process:
   a. SyncVersionDataUseCase → check flow/data version
   b. If version changed:
      - SyncCurrencyUseCase
      - SyncEntityDataUseCase
      - SyncHubsUseCase
      - Download new flow definitions
   c. SyncFcmTokenUseCase
   d. Upload pending task data
   e. Upload pending media (photos, videos)
   f. Process page webhook queue
   g. Sync location history (bulk)
3. On failure:
   a. Generate sync failure report
   b. Store in sync_failure_reports
   c. Show sync failure sheet to user
   d. User can copy details and report to admin
```

### 9.5 Organization Switch Flow

```
1. User goes to Settings → Change Organization
2. App fetches organizations (GetOrganizationsUseCase)
3. User selects new organization
4. App calls POST /organization/switch
5. On success:
   a. Clear local data:
      - RemoveLocalTaskDataUseCase
      - Clear flow data
      - Clear currency data
      - Clear OTP cache
   b. Save new org ID to local
   c. Fetch new org configuration
   d. Re-sync all data for new org
   e. Navigate to hub chooser
6. User selects new hub
7. App saves hub (SaveCurrentHubUseCase)
8. Navigate to main menu
```

---

## 10. Business Logic

### 10.1 Task Step Order Validation

Tasks may have dependencies (step ordering). `ValidateTaskStepOrderUseCase` checks:
- Previous task activity must be completed before starting next
- If invalid: "Cannot do this task. Finish the previous task activity first."

### 10.2 Geofence Validation

Check-in requires the user to be within the hub's geofence:
- `GeoLockEntity` contains lat, lng, radius
- Distance calculated using Haversine formula
- If outside: "You are {distance} meters away from {hubName}"

### 10.3 Permission System

Permissions are scoped to organization + hub + user:

```
Permission scope hierarchy:
  Organization → Hub → User Role → Permissions

Permissions control:
  - Can check in (GetPermissionCheckInUseCase)
  - Can add task (noAddTaskPermission)
  - Can do task (noDoTaskPermission)
  - Can view entity data (noViewEntityDataPermission)
```

### 10.4 Page Webhook System

Each task page can trigger webhooks:
- `PageWebhookConfigurationEntity` defines webhook URL per page
- `AddPageWebhookDataToQueueUseCase` queues webhook data
- `SyncPageWebhookUseCase` processes queue
- On failure: sync failure report generated

### 10.5 Offline-First Strategy

```
Read path:
  1. Try local DB first
  2. If stale or empty → fetch from remote
  3. Save to local
  4. Return data

Write path:
  1. Save to local DB immediately
  2. Mark as unsynced
  3. Queue for sync
  4. Background sync attempts
  5. On success → mark as synced
  6. On failure → retry + sync failure report
```

### 10.6 Route Optimization

`RouteOptimizationPage` reorders task visits by nearest location:
- `GetRouteOptimizationUseCase` calls backend optimization API
- `RouteOptimizationReqModel` contains task locations
- `RouteOptimizationResModel` returns optimized order
- User confirms: "This will change the delivery order to prioritize the closest locations first."

### 10.7 Mock GPS Detection

`libtoolChecker.so` (native) + `mock_gps_app_detected` feature:
- Detects installed mock GPS apps
- Detects enabled mock location provider
- Shows `MockGpsAppDetectedPage` blocking app usage

### 10.8 App Version Enforcement

- `IsRequireToUpdateAppUseCase` checks if current version >= minimum required
- `GetLastMinVersionUpdateUseCase` gets minimum version from backend
- If below minimum: force update screen
- `GetAppUpdateAvailableFlagUseCase` checks for optional updates

### 10.9 Password Expiry

- `IsPasswordExpiredUseCase` checks password age
- If expired: redirect to `/change-password`
- Message: "Your password has expired, please change your password to keep your account secure."

### 10.10 QR Login (Cross-Device)

- `LoginWithTokenUseCase` processes QR token
- `LoginWithTokenThenSaveToLocalUseCase` saves session
- Mobile scans QR from web app → authenticates web session

---

## 11. End-to-End Flow

### Complete Task Lifecycle (E2E)

```
[Web Admin]                    [Backend API]                    [Mobile Field Worker]
     │                              │                                    │
     │  1. Create Flow              │                                    │
     │  (define pages, components)  │                                    │
     ├─────────────────────────────►│                                    │
     │                              │                                    │
     │  2. Create Task              │                                    │
     │  (assign to user, hub)       │                                    │
     ├─────────────────────────────►│                                    │
     │                              │                                    │
     │                              │  3. Push FCM notification          │
     │                              ├───────────────────────────────────►│
     │                              │                                    │
     │                              │                          4. Check-in│
     │                              │                          (geofence)│
     │                              │◄───────────────────────────────────┤
     │                              │                                    │
     │                              │  5. Fetch task list                │
     │                              │◄───────────────────────────────────┤
     │                              │  Return tasks                      │
     │                              ├───────────────────────────────────►│
     │                              │                                    │
     │                              │  6. Fetch flow definition          │
     │                              │◄───────────────────────────────────┤
     │                              │  Return flow + pages               │
     │                              ├───────────────────────────────────►│
     │                              │                                    │
     │                              │              7. Execute task pages │
     │                              │              (photo, video, input,│
     │                              │               select, bill, OTP,  │
     │                              │               scan, voice, etc.)  │
     │                              │                                    │
     │                              │  8. Upload media to S3             │
     │                              │  (photos, videos)                 │
     │  [S3]                        │◄───────────────────────────────────┤
     │   ◄──────────────────────────│  9. Submit task data              │
     │                              │◄───────────────────────────────────┤
     │                              │  10. Trigger page webhooks         │
     │                              │  (if configured)                  │
     │  [External webhook]          │                                    │
     │   ◄──────────────────────────├──────────────────────────►        │
     │                              │                                    │
     │  11. View task results       │                          12. Check-out│
     │  (web dashboard)             │                          (end trip)│
     │  ◄───────────────────────────┤◄───────────────────────────────────┤
     │                              │                                    │
     │  13. View location history   │                                    │
     │  (GPS trail)                 │                                    │
     │  ◄───────────────────────────┤                                    │
     │                              │                                    │
     │  14. Generate reports        │                                    │
     │  ◄───────────────────────────┤                                    │
```

---

## 12. Pseudocode

See [ALTIUS_PSEUDOCODE.md](./ALTIUS_PSEUDOCODE.md) for detailed pseudocode of core flows.

### 12.1 Login Flow (Summary)

```
function login(email, password):
    response = POST /auth {email, password}
    
    if response.requiresMFA:
        if response.mfaType == OTP:
            POST /otp/send {recipient}
            code = getUserInput()
            result = POST /otp/verify {code}
        elif response.mfaType == TOTP:
            code = getUserInput()
            result = POST /cloud-authenticator/verify {code}
        if !result.success:
            return error(result.message)
    
    if response.passwordExpired:
        navigateTo(/change-password)
        return
    
    saveTokenToLocal(response.token)
    saveUserDataToLocal(response.user)
    registerDeviceToken(response.user.id, fcmToken)
    
    organizations = getOrganizations()
    if organizations.length > 1:
        navigateTo(/organizations)
    else:
        saveOrgId(organizations[0].id)
        syncVersionData()
        navigateTo(/main-menu)
```

### 12.2 Task Execution Engine (Summary)

```
function executeTask(task):
    flow = getDetailFlow(task.flowId)
    stepOrder = validateTaskStepOrder(task)
    if !stepOrder.valid:
        showError("Finish previous task first")
        return
    
    for page in flow.pages:
        renderPageTitle(page.title)
        for component in page.components:
            result = renderComponent(component)
            if component.required && !result.filled:
                showError("Required field incomplete")
                break
            saveComponentData(task.id, page.id, component.id, result.data)
        
        if page.webhookConfigured:
            queuePageWebhook(task.id, page.id, page.webhookUrl, pageData)
    
    updateTaskStatus(task.id, DONE)
    syncTaskToServer(task.id)
    uploadPendingMedia(task.id)
    updateLastDoneOrder(task.id)
```

### 12.3 Sync Engine (Summary)

```
function sync():
    versionData = checkVersion()
    if versionData.flowVersionChanged:
        syncFlows()
    if versionData.dataVersionChanged:
        syncEntityData()
        syncCurrencies()
        syncHubs()
    
    syncFcmToken()
    
    pendingTasks = getUnsyncedTasks()
    for task in pendingTasks:
        uploadTaskData(task)
        uploadMedia(task)
        if success:
            markSynced(task)
        else:
            generateSyncFailureReport(task)
    
    processWebhookQueue()
    syncLocationHistoryBulk()
```

---

## 13. Tenant Model

See [ALTIUS_TENANT_MODEL.md](./ALTIUS_TENANT_MODEL.md) for the complete tenant breakdown.

### Tenant Hierarchy

```
Build Flavor / Environment
  └── Runtime Organization
       └── Hub / Warehouse
            └── User Permissions & Roles
                 └── User Session & Local Data
```

### Build Flavors (8)

| Flavor | API | Web | Special |
|---|---|---|---|
| Prod | api.altius.example | app.altius.example | Standard production |
| Dev | api-dev.altius.example | app-dev.altius.example | Development |
| Beta | api-beta.altius.example | app-beta.altius.example | Beta testing |
| Sandbox | api-sandbox.altius.example | app-sandbox.altius.example | Testing |
| Unilever | api-enterprise.altius.example | app.altius.example | Unilever tenant |
| Unilever Sandbox | api-sandbox.altius.example | app.altius.example | Unilever testing |
| Y3 | api.altius.example | app.altius.example | Y3 branding |
| Y3 Sandbox | api-sandbox.altius.example | app-sandbox.altius.example | Y3 testing |

### Tenant Isolation Mechanisms

1. **Build-time:** Separate API base URLs per flavor
2. **Runtime:** Organization switching with local data clearing
3. **Hub-level:** Geofence + permission scoping
4. **User-level:** Role-based permissions per org+hub

---

## 14. Mobile & Web Connectivity

### 14.1 Connection Points

| Connection | Type | Evidence |
|---|---|---|
| Shared Backend API | Confirmed | Both mobile and web use same `/api/v3` endpoints |
| QR Login Bridge | Confirmed | `LoginWithTokenUseCase` + `ProfileQRPage` |
| WebView Embed | Confirmed | `WebViewPage` + `/web-view` route |
| Shared Assets | Confirmed | `flutter_assets/web/` contains shared fonts, icons, translations |
| Shared Auth Token | Inferred | QR login implies shared session/token |
| Direct API Calls | Unconfirmed | No evidence of mobile calling web app API directly |

### 14.2 Web application

The operations web app lives in [`apps/web`](../apps/web) (Next.js App Router). It talks to the same `/api/v3` backend as mobile when `NEXT_PUBLIC_API_BASE` and Keycloak are configured. Deploy target: **Vercel**. Marketing site: [`apps/landing`](../apps/landing).

---

## 15. UI Breakdown

See [ALTIUS_UI_BREAKDOWN.md](./ALTIUS_UI_BREAKDOWN.md) for the complete UI structure breakdown covering:
- 68 routes
- 40+ pages
- 13 task component types
- 30+ Cubits
- 614 translation keys in 7 languages
- Mobile ↔ Web UI mapping
- Design system

---

## 16. Security & Permissions

### 16.1 Android Permissions (28)

| Permission | Purpose |
|---|---|
| ACCESS_FINE_LOCATION | GPS for check-in geofence, task GPS, location history |
| ACCESS_COARSE_LOCATION | Approximate location fallback |
| ACCESS_BACKGROUND_LOCATION | Location tracking while app is closed (during check-in) |
| FOREGROUND_SERVICE | Persistent location tracking service |
| FOREGROUND_SERVICE_LOCATION | Location foreground service type |
| CAMERA | Photo/video capture, barcode scanning |
| RECORD_AUDIO | Voice recording component |
| READ_EXTERNAL_STORAGE | Gallery photo/video selection |
| WRITE_EXTERNAL_STORAGE | Saving photos/videos locally |
| INTERNET | API communication, media upload |
| ACCESS_NETWORK_STATE | Connectivity check for sync |
| ACCESS_WIFI_STATE | WiFi connectivity check |
| POST_NOTIFICATIONS | FCM push notifications |
| RECEIVE_BOOT_COMPLETED | Start services on boot |
| WAKE_LOCK | Keep device awake during sync/upload |
| VIBRATE | Notification vibration |
| SCHEDULE_EXACT_ALARM | Scheduled sync alarms |
| REQUEST_IGNORE_BATTERY_OPTIMIZATIONS | Battery optimization exemption for location tracking |
| SYSTEM_ALERT_WINDOW | Overlay display |
| BLUETOOTH / BLUETOOTH_ADMIN / BLUETOOTH_CONNECT / BLUETOOTH_SCAN | Bluetooth (printer, peripherals) |
| READ_PHONE_STATE | Device identification |
| DUMP | System diagnostics |
| BIND_JOB_SERVICE | Background job scheduling |
| ACCESS_ADSERVICES_AD_ID / ACCESS_ADSERVICES_ATTRIBUTION | Advertising ID (analytics) |

### 16.2 Security Features

- **Root detection:** `libtoolChecker.so` native library
- **Mock GPS detection:** `mock_gps_app_detected` feature
- **App tamper detection:** freeRASP integration
- **Face recognition:** Luxand liveness check (login)
- **SSL/TLS:** HTTPS for all API communication
- **Token-based auth:** Bearer token stored locally
- **MFA:** OTP + TOTP (cloud authenticator)
- **Crashlytics:** Sentry + Firebase Crashlytics

> **Security Note:** Environment files contain sensitive integration values (Sentry DSN, Discord webhook URL, Luxand key, freeRASP config). These should be treated as exposed and rotated if this document is shared externally.

---

## 17. Environment Configuration

### 17.1 Environment Variables

| Variable | Description |
|---|---|
| `BASE_URL` | API base URL (e.g., `https://api.altius.example/api/v3`) |
| `WEB_ORIGIN` | Web app origin (e.g., `https://app.altius.example/`) |
| `REGION_BASE_ENDPOINT` | S3 region endpoint (`s3-ap-southeast-1.amazonaws.com`) |
| `HMS_URL` | HMS e-learning URL for road hazard awareness |
| `UNILEVER_BASE_URL` | Unilever-specific base URL |
| `IS_DEBUG` | Debug flag (true in non-production env catalogs) |

### 17.2 App Names per Flavor

| Flavor | App Name |
|---|---|
| Production | Altius Field |
| Beta | Altius Field Beta |
| Dev | Altius Field Dev |
| Sandbox | Altius Field Sandbox |

### 17.3 Localization

| Language | File | Locale |
|---|---|---|
| English | `en-US.json` | en-US |
| Indonesian | `id-ID.json` | id-ID |
| Thai | `th-TH.json` | th-TH |
| Japanese | `ja-JP.json` | ja-JP |
| Chinese | `zh-CN.json` | zh-CN |
| Filipino | `fil-PH.json` | fil-PH |
| Vietnamese | `vi-VN.json` | vi-VN |

Total: 614 translation keys per language, organized into 3 sections:
- `__SECTION_LABELS__` — UI labels
- `__SECTION_ERROR_MESSAGES__` — Error messages
- `__SECTION_INFORMATION_MESSAGES__` — Information messages

### 17.4 Design System

| Element | Detail |
|---|---|
| Font | NunitoSans (Regular, Bold, SemiBold, ExtraBold, Light, Italic) |
| Custom Icons | CustomIcons.ttf (35 icons) |
| Primary Logo | `logo_altius.png` |
| Y3 Logo | `y3_logo_purple2.png` |
| Color Scheme | Purple-based |
| Copyright | "© 2024 • Altius • Indonesia" |
| Login Caption | "Field Worker Management App" |

---

## 18. Evidence Classification

| Finding | Status | Basis |
|---|---|---|
| Flutter Clean Architecture | Confirmed | Package paths in libapp.so |
| 29 feature modules | Confirmed | Package path enumeration |
| 13 task component types | Confirmed | Component package paths |
| 114 use cases | Confirmed | Use case class enumeration |
| 60+ domain entities | Confirmed | Entity class enumeration |
| 8 environment flavors | Confirmed | Build-time env catalogs + runtime org switch |
| Shared backend API | Confirmed | Environment files show API + web origin pairs |
| QR login bridge | Confirmed | LoginWithTokenUseCase + ProfileQRPage |
| WebView embed | Confirmed | WebViewPage + /web-view route + WebView classes |
| Offline-first sync | Confirmed | Local/remote data sources + sync use cases |
| Geofence check-in | Confirmed | GeoLockEntity + distance error messages |
| Page webhook system | Confirmed | Webhook entities + queue + sync use cases |
| Route optimization | Confirmed | RouteOptimization feature + API endpoint |
| Mock GPS detection | Confirmed | Native lib + feature module |
| Web app is admin dashboard | Inferred | Shared API + field worker vs admin roles |
| Web app built with Flutter Web | Inferred | Shared assets in flutter_assets/web/ |
| WebView uses WEB_ORIGIN | Inferred | Not found as direct literal in libapp.so |
| Mobile and web share session | Inferred | QR login implies shared auth |
| Backend database schema | Confirmed | `backend/crates/altius-api/migrations/` |
| Web app source code | Confirmed | `apps/web` (Next.js) |

---

*End of Architecture Documentation*
