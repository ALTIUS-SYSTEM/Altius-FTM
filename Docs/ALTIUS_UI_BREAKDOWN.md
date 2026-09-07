# Altius FTM — UI Breakdown (Mobile & Web)

> **Source:** Reverse-engineered from `app.paket.mile_field` v1.40.8  
> **Date:** 2026-09-07  

---

## Table of Contents

1. [Mobile App UI Structure](#1-mobile-app-ui-structure)
2. [Navigation & Routing](#2-navigation--routing)
3. [Screen Hierarchy](#3-screen-hierarchy)
4. [Task Component System](#4-task-component-system)
5. [Core Widgets](#5-core-widgets)
6. [State Management](#6-state-management)
7. [Localization](#7-localization)
8. [Web App UI Structure](#8-web-app-ui-structure-inferred)
9. [Mobile & Web UI Mapping](#9-mobile--web-ui-mapping)
10. [Design System](#10-design-system)

---

## 1. Mobile App UI Structure

The mobile app has two code layers:
- **Legacy layer** (`bloc/` + `ui/`) — pre-refactor code
- **Clean Architecture layer** (`clean_architecture/features/`) — refactored code

Both layers coexist. New features use Clean Architecture; some legacy screens remain.

---

## 2. Navigation & Routing

Router: `clean_architecture/core/routers/app_module.dart`  
Guard: `clean_architecture/core/routers/guards/auth_guard.dart`  
Framework: `flutter_modular`

### 2.1 Complete Route Map (68 routes)

#### Auth Routes

| Route | Page | Description |
|---|---|---|
| `/login` | LoginPage | Email + password login |
| `/otp` | OTPPage | OTP code verification |
| `/forget-password` | ForgetPasswordPage | Password recovery |
| `/change-password` | ChangePasswordPage | Password change (expired) |
| `/cloud-authenticator` | CloudAuthenticatorPage | TOTP authenticator |
| `/privacy-policy` | PrivacyPolicyPage | Privacy policy display |

#### Onboarding & Organization

| Route | Page | Description |
|---|---|---|
| `/organizations` | ChangeOrganizationPage | Org selection & switching |
| `/hub-chooser` | HubChooserPage | Hub/warehouse selection |

#### Main Navigation

| Route | Page | Description |
|---|---|---|
| `/main-menu` | MainMenuPage | Main screen with task tabs |
| `/more` | MoreMenuPage | More menu (custom modules) |
| `/setting` | SettingPage | Settings screen |
| `/profile-qr` | ProfileQRPage | QR code for web login |
| `/web-view` | WebViewPage | Embedded web content |
| `/troubleshooting` | TroubleshootingPage | Device diagnostics |
| `/road-warning` | RoadWarningPage | HMS road hazard awareness |
| `/mock-gps-app-detected` | MockGpsAppDetectedPage | Fake GPS detection block |

#### Task Routes

| Route | Page | Description |
|---|---|---|
| `/tasks` | TaskPage | Task list (legacy) |
| `/task-list` | TaskListPage | Task list (ongoing/done tabs) |
| `/task-detail` | TaskDetailPage | Task detail view |
| `/task-create` | TaskCreatePage | Task creation (flow selection) |
| `/task-page-list-detail` | TaskPagesDetail | Multi-page task execution |
| `/reorder-visits` | RouteOptimizationPage | Route optimization |
| `/resume-route` | ResumeRoute | Resume route navigation |
| `/task` | TaskPage | Task entry |
| `/task/{id}` | TaskDetail | Task by ID |

#### Check-In/Out

| Route | Page | Description |
|---|---|---|
| `/check-in-out` | CheckInOutPage | Check-in/check-out with geofence |

#### Task Map

| Route | Page | Description |
|---|---|---|
| `/task-map` | TaskMapPageClean | Google Maps task navigation |

#### History

| Route | Page | Description |
|---|---|---|
| `/history` | HistoryPage | Sync failure history |
| `/location-histories` | LocationHistoryPage | GPS trail history |

#### Data Management

| Route | Page | Description |
|---|---|---|
| `/data-source-list` | DataSourceListPage | Data source list |
| `/data-type-list` | DataTypeChooserPage | Data type chooser |
| `/data-types` | EntityFeaturePage | Entity data management |
| `/data-version` | DataVersionPage | Data version info |
| `/currency` | CurrencyPage | Currency management |
| `/custom-module` | CustomModulePage | Custom module page |
| `/custom-modules` | CustomModulesPage | Custom modules list |

#### Component Pages

| Route | Page | Description |
|---|---|---|
| `/camera` | CameraPage | Photo capture |
| `/gallery-ca` | GalleryPage | Photo gallery selection |
| `/video-camera` | VideoCameraPage | Video recording |
| `/video-gallery` | VideoGalleryPage | Video gallery selection |
| `/video-full-page` | VideoFullScreen | Full screen video player |
| `/pdf-full-page` | PDFFullScreen | Full screen PDF viewer |
| `/gallery-component-view` | ImageFullScreen | Full screen image viewer |
| `/barcode` | BarcodePage | Barcode/QR scanner |
| `/subpage-form` | SubpageFormPage | Subpage form |
| `/subpage-list` | SubpageListPage | Subpage list |
| `/bill` | BillPage | Bill component |

#### Utility Routes

| Route | Page | Description |
|---|---|---|
| `/logout` | Logout | Sign out |
| `/keys` | KeysPage | Key-value config |
| `/user` | UserPage | User info |
| `/device-token` | DeviceTokenPage | FCM token management |
| `/last` | LastPage | Last visited |
| `/mile_files` | MileFiles | File management |
| `/mile_images` | MileImages | Image management |
| `/trk` | TrackingPage | Tracking |
| `/maps/dir/2` | MapDirections | Map directions |
| `/compressed` | CompressedPage | Compressed media |
| `/store/apps/details` | PlayStorePage | App store link |
| `/to/` | WebhookTarget | Webhook target |

#### Obfuscated Routes (likely internal)

| Route | Likely Purpose |
|---|---|
| `/akn`, `/c2q`, `/cNr`, `/gt4`, `/iog`, `/jzJ`, `/l4d`, `/mb4`, `/mpF`, `/oIn`, `/sSp`, `/v4b`, `/xEW`, `/zmB`, `/secretH` | Internal/obfuscated routes |

---

## 3. Screen Hierarchy

### 3.1 User Journey (Mobile)

```
Splash → Auth Guard
  │
  ├─[NOT logged in]→ /login
  │    ├─ FormLoginWidget (email + password)
  │    ├─ LoginTopBannerWidget (logo + banner)
  │    ├─ LoginAppVersionLabelWidget
  │    ├─ LoginCopyrightLabelWidget
  │    ├─ LoginRecoveryPasswordBtnWidget → /forget-password
  │    └─ ViewSucceedLogInWidget
  │         ├─ /otp (6-digit OTP code)
  │         ├─ /cloud-authenticator (TOTP)
  │         └─ /change-password (if expired)
  │
  ├─[Logged in, no org]→ /organizations
  │    └─ Select org → POST /organization/switch
  │
  ├─[Logged in, no hub]→ HubChooserPage
  │    └─ Select hub → SaveCurrentHubUseCase
  │
  └─[Logged in, org+hub set]→ /main-menu
       │
       ├─ TAB: Ongoing (TaskPage → OngoingPage)
       │    ├─ Pull-to-refresh → sync
       │    ├─ Reorder visits → /reorder-visits
       │    ├─ Tap task → /task-detail
       │    │    └─ /task-page-list-detail → TaskDoing
       │    │         ├─ Page 1: ComponentPhoto → /camera, /gallery-ca
       │    │         ├─ Page 2: ComponentVideo → /video-camera, /video-gallery
       │    │         ├─ Page 3: ComponentInput (text/currency/date/url/entity)
       │    │         ├─ Page 4: ComponentSelect (dropdown/multi/checkbox)
       │    │         ├─ Page 5: ComponentList → items with qty
       │    │         ├─ Page 6: ComponentBill → items + costs + summary
       │    │         ├─ Page 7: ComponentScanDisplay → /barcode
       │    │         ├─ Page 8: ComponentOTP → OTP verification
       │    │         ├─ Page 9: ComponentCapture → signature
       │    │         ├─ Page 10: ComponentVoice → voice recording
       │    │         ├─ Page 11: ComponentPrint → print receipt
       │    │         ├─ Page 12: ComponentView → view-only content
       │    │         ├─ Page 13: ComponentSubpage → /subpage-form, /subpage-list
       │    │         └─ Finish → sync to server
       │    ├─ Create task → /task-create
       │    └─ Tap task on map → /task-map
       │
       ├─ TAB: Done (DonePage)
       │    └─ Completed tasks → tap to view detail
       │
       ├─ More Menu → /more
       │    ├─ Custom Modules
       │    ├─ Data Source → /data-source-list
       │    ├─ History → /history
       │    ├─ Location History → /location-histories
       │    ├─ Troubleshooting → /troubleshooting
       │    ├─ Road Warning → /road-warning
       │    └─ WebView → /web-view
       │
       ├─ Settings → /setting
       │    ├─ ProfileWidget (user info)
       │    ├─ ProfileQR → /profile-qr (QR for web login)
       │    ├─ CheckOutWidget → end trip
       │    ├─ Change Organization → /organizations
       │    ├─ Change Password → /change-password
       │    ├─ Privacy Policy → /privacy-policy
       │    └─ Sign Out → /logout
       │
       └─ Check-in/out → CheckInOutPage
            ├─ CheckInSuccessWidget
            ├─ CheckOutSuccessWidget
            ├─ CheckInDataLoadedWidget
            └─ GeoLock validation
```

### 3.2 Task Map Screen

```
TaskMapPageClean
  ├─ MapViewWidget (Google Maps)
  │    ├─ Task markers (MapMarkerGenerator)
  │    ├─ User location
  │    └─ Route lines
  ├─ TaskSlidingPanelWidget (bottom sheet)
  │    ├─ PanelDragHandleWidget
  │    ├─ TaskDetailPopupWidget
  │    └─ MinimizedTaskHeaderWidget
  └─ NavigationButtonsWidget
```

### 3.3 Check-In/Out Screen

```
CheckInOutPage
  ├─ CheckInDataLoadedWidget
  │    ├─ Hub name display
  │    ├─ Distance to hub
  │    └─ Info text field (optional)
  ├─ CheckInSuccessWidget
  │    ├─ Success message
  │    └─ Check-in details (time, hub, info)
  ├─ CheckOutSuccessWidget
  │    ├─ Success message
  │    └─ Check-out details (time, duration)
  └─ ErrorWidget (geofence failure)
```

### 3.4 Settings Screen

```
SettingPage
  ├─ ProfileWidget
  │    ├─ User name
  │    ├─ User email
  │    ├─ Organization name
  │    └─ Hub name
  ├─ ProfileQR (QR code display for web login)
  ├─ CheckOutWidget (end trip button)
  ├─ Change Organization → /organizations
  ├─ Change Password → /change-password
  ├─ Privacy Policy → /privacy-policy
  ├─ App Version
  └─ Sign Out → /logout
```

### 3.5 Troubleshooting Screen

```
TroubleshootingPage
  ├─ Header (title)
  ├─ Body
  │    ├─ API Connection check (CheckAPIConnectionUseCase)
  │    ├─ Download Connection check (CheckDownloadConnectionUseCase)
  │    ├─ Upload Connection check (CheckUploadConnectionUseCase)
  │    └─ App Version check (IsUsingLatestAppVersionUseCase)
  ├─ RowChecking (per-check row with status)
  ├─ Footer (actions)
  └─ Image upload for support
```

### 3.6 History Screen

```
HistoryPage
  ├─ HistoryInfoBannerWidget (retention info: 24 hours)
  ├─ HistoryExpandableCategoryWidget
  │    ├─ Task Sync Failed
  │    ├─ Task Status Updated
  │    ├─ Page Webhook failures
  │    ├─ Media Upload failures
  │    ├─ Currency sync failures
  │    ├─ Flow sync failures
  │    ├─ Location History sync failures
  │    └─ Other sync failures
  └─ HistoryEmptyViewWidget (when no history)
```

### 3.7 Sync Failure Report Sheet

```
SyncFailureReportSheetWidget
  ├─ SyncFailureSourceGroupWidget (grouped by source)
  ├─ SyncFailureTaskCardListWidget
  │    └─ SyncFailureTaskCardWidget (per task)
  ├─ SyncFailureCopyHintWidget ("Tap copy to report to admin")
  └─ Actions: Copy All, Retry Sync
```

---

## 4. Task Component System

### 4.1 Component Types (13)

| # | Component | Pages | Widgets | Purpose |
|---|---|---|---|---|
| 1 | **photo** | CameraPage, GalleryPage, ComponentPhotoPage | PhotoGridView, CategoriesDropdown, FooterButton | Capture photos from camera/gallery with categories, max limit |
| 2 | **video** | VideoCameraPage, VideoGalleryPage, ComponentVideoPage | ControlsOverlayVideo | Record video with max duration, gallery selection |
| 3 | **input** | ComponentInput | InputTextCurrency, InputTextDate, InputTextEntityData, InputTextURL | Text input with types: plain, currency, date, URL, entity reference |
| 4 | **select** | (inline) | DropdownWidget, MultiSelectWidget, LabeledCheckBoxWidget, SearchWidget | Dropdown, multi-select, checkbox selection |
| 5 | **list** | (inline) | ListCaCubit, ListQtyCounterModel | List items with quantity counters |
| 6 | **bill** | BillPage | BillBody, BillItemModel, BillSummary, BillList | Bill items + costs + summary with qty, discount |
| 7 | **scan_display** | ComponentScanDisplay | ScanButtonWidget | Scan QR/barcode, display result |
| 8 | **otp** | ComponentOtpPage | OTPInputFieldWidget | OTP verification to recipient (SMS/email) |
| 9 | **capture** | ComponentCapturePage | CaptureWidget | Signature capture / data capture |
| 10 | **voice** | ComponentVoicePage | PlayRecord, SoundRecord | Voice recording + playback |
| 11 | **print** | ComponentPrint | (print dialog) | Print receipt/label via Bluetooth printer |
| 12 | **view** | ComponentViewPage, ImageFullScreen, PDFFullScreen, VideoFullScreen | ComponentViewDefault, ComponentViewCurrency, ViewUrl, ErrorPreviewWidget | View-only: image, PDF, video, currency, URL |
| 13 | **subpage** | SubpageFormPage, SubpageListPage | SubpageEntryCard, SubpageListContent | Nested sub-flow / sub-form |

### 4.2 Component Validation Rules

| Rule | Message Key | Description |
|---|---|---|
| Required field | `cannotBeEmpty` | "Required" |
| Photo max limit | `maxPhotoLimit` | "Max {maxPage} photo" |
| Video too long | `videoTooLong` | "Video too long. Maximum {maxDuration} seconds." |
| Video must be watched | `finishVideoRequired` | "Watch the video to the end to continue" |
| Voice required | `voiceRecordRequired` | "Please record to continue the process" |
| OTP invalid | `otpInvalidCode` | "Invalid code. Please enter the correct code from the recipient." |
| OTP expired | `validOtpButExceedLimit` | "The one-time passcode (OTP) you entered has expired." |
| OTP max attempts | `otpMaxVerifyAttemptsReached` | "Invalid attempts reached. Please tap 'Resend' to re-verify." |
| Bill no data | `billMandatoryMessage` | "You don't have a data component bill yet, please create one first" |
| Bill no reason | `billReasonMandatoryMessage` | "There are data bills that do not have a reason. please complete it first" |
| Bill cannot combine | `cannotCombineBillWithOtherComponents` | "Bill component cannot be combined with other component." |
| List cannot combine | `cannotCombineListWithOtherComponents` | "List component cannot be combined with other component." |
| Qty below initial | `cannotDecreaseItemQtyBelowInitial` | "You cannot decrease the item quantity below the initial value" |
| Qty above initial | `cannotIncreaseItemQtyAboveInitial` | "You cannot increase the item quantity above the initial value" |
| Min value | `errorMinimumValue` | "The value you entered must not be less than {minValue}" |
| Max value | `errorMaximumValue` | "The value you entered must not exceed {maxValue}" |
| Number only | `errorNumberOnly` | "Please insert number only" |
| Phone only | `errorPhoneNumberOnly` | "Please insert phone number only" |
| Capture required | `captureComponentRequired` | "Please retrieve the {componentName} data first" |

---

## 5. Core Widgets

### 5.1 Core Reusable Widgets (`clean_architecture/core/widgets/`)

| Widget | File | Purpose |
|---|---|---|
| ButtonDefault | `button_default.dart` | Standard button |
| CustomButton | `custom_button.dart` | Customizable button |
| CustomButtonDialog | `custom_button_dialog.dart` | Dialog button |
| DefaultText | `default_text.dart` | Standard text display |
| LeadingEllipsisText | `leading_ellipsis_text.dart` | Text with leading ellipsis |
| DropdownDefault | `dropdown_default.dart` | Dropdown selector |
| ListDefault | `list_default.dart` | List container |
| SearchBox | `search_box.dart` | Search input field |
| ProgressIndicatorDefault | `progress_indicator_default.dart` | Loading spinner |
| SeparatorDefault | `separator_default.dart` | Divider line |
| TextFormFieldCustom | `text_field/text_form_field_custom.dart` | Custom form input |
| TextFormFieldDefault | `text_field/text_form_field_default.dart` | Standard form input |
| TextFormFieldPasswordDefault | `text_field/text_form_field_password_default.dart` | Password input |
| BottomSheetHelper | `sheet_helper/bottom_sheet_helper.dart` | Bottom sheet manager |
| LocationDisclosureDialogContent | `location_disclosure_dialog_content.dart` | GPS permission dialog |
| GutterScrollViewWidget | `gutter_scroll_view_widget.dart` | Scrollable container |
| DefaultComponentWidget | `default_component_widget.dart` | Default component wrapper |

### 5.2 Shared Feature Widgets (`shared_features/shared/presentation/widgets/`)

| Widget | Purpose |
|---|---|
| CustomButtonWidget | Shared button |
| CustomTextFormFieldWidget | Shared text input |
| DefaultProgressIndicatorWidget | Shared loading indicator |
| LifecycleWatcherState | App lifecycle observer |
| SyncFailureReportSheetWidget | Sync failure bottom sheet |
| SyncFailureTaskCardListWidget | Task card list in sync failure |
| SyncFailureTaskCardWidget | Individual task card |
| SyncFailureSourceGroupWidget | Group by failure source |
| SyncFailureCopyHintWidget | Copy-to-report hint |

### 5.3 Global Widgets (`widgets/`)

| Widget | Purpose |
|---|---|
| TextButton | Text button (`widgets/button/text_button.dart`) |
| DefaultDropdown | Dropdown (`widgets/dropdown/default_dropdown.dart`) |
| ImageNetworkMobile | Network image loader (`widgets/image/image_network_mobile.dart`) |
| ProgressIndicatorDefault | Loading (`widgets/loading/progress_indicator_default.dart`) |
| RowTask | Task row item (`widgets/row/row_task.dart`) |
| Signature | Signature pad (`widgets/signature/signature.dart`) |

### 5.4 Feature-Specific Widgets

#### Login Widgets

| Widget | Purpose |
|---|---|
| FormLoginWidget | Login form (email + password) |
| LoginTopBannerWidget | Logo + banner image |
| LoginAppVersionLabelWidget | App version label |
| LoginCopyrightLabelWidget | Copyright text |
| LoginRecoveryPasswordBtnWidget | Forgot password button |
| ViewSucceedLogInWidget | Success login view |

#### Check-In/Out Widgets

| Widget | Purpose |
|---|---|
| CheckInDataLoadedWidget | Check-in data display |
| CheckInSuccessWidget | Check-in success view |
| CheckOutSuccessWidget | Check-out success view |
| ErrorWidget | Geofence error display |

#### Task Map Widgets

| Widget | Purpose |
|---|---|
| MapViewWidget | Google Maps view |
| TaskSlidingPanelWidget | Bottom sliding panel |
| TaskDetailPopupWidget | Task detail popup |
| NavigationButtonsWidget | Navigation controls |
| MinimizedTaskHeaderWidget | Minimized task header |
| PanelDragHandleWidget | Drag handle for panel |
| MapMarkerGenerator | Map marker generator (helper) |

#### Route Optimization Widgets

| Widget | Purpose |
|---|---|
| RouteConfirmDialogWidget | Confirm route optimization |
| RouteLoadingDialogWidget | Loading during optimization |

#### History Widgets

| Widget | Purpose |
|---|---|
| HistoryEmptyViewWidget | Empty history state |
| HistoryExpandableCategoryWidget | Expandable category |
| HistoryInfoBannerWidget | Info banner (24h retention) |

#### Task Doing Widgets

| Widget | Purpose |
|---|---|
| GalleryPhotoViewWidget | Photo gallery viewer |
| PageTitle | Page title display |
| NoComponentFound | No component error |
| RequiredFieldsIncompleteDialogWidget | Required fields dialog |

---

## 6. State Management

### 6.1 Cubit Inventory (30+)

| Feature | Cubit | State |
|---|---|---|
| Login | CaLoginCubit | CaLoginState |
| OTP | OtpCubit | OtpState |
| ForgetPassword | ForgetPasswordCubit | ForgetPasswordState |
| ChangePassword | ChangePasswordCubit | ChangePasswordState |
| ChangeOrganization | ChangeOrganizationCubit | ChangeOrganizationState |
| CheckInOut | CheckInOutCubit | CheckInOutState |
| CloudAuthenticator | CloudAuthenticatorCubit | CloudAuthenticatorState |
| History | HistoryCubit | HistoryState |
| MoreMenu | MoreMenuCubit | MoreMenuState |
| PrivacyPolicy | PrivacyPolicyCubit | PrivacyPolicyState |
| RoadWarning | RoadWarningCubit | RoadWarningState |
| RouteOptimization | RouteOptimizationCubit | RouteOptimizationState |
| Settings | SettingCubit | SettingState |
| TaskCreate | TaskCreateCubit | TaskCreateState |
| TaskDoing | TaskDoingCubit | TaskDoingState |
| TaskPages | TaskPagesCubit | TaskPagesState |
| TaskStepOrder | TaskStepOrderCubit | TaskStepOrderState |
| TaskListOngoing | TaskListOngoingCubit | TaskListOngoingState |
| TaskMap | TaskMapCubit | TaskMapState |
| Troubleshooting | TroubleshootingCubit | TroubleshootingState |
| DataType | DataTypeCubit | DataTypeState |
| EntityFeature | EntityFeatureCubit | EntityFeatureState |
| ComponentCapture | ComponentCaptureCubit | ComponentCaptureState |
| ComponentPhoto | ComponentPhotoCubit | ComponentPhotoState |
| Camera | CameraCubit | CameraState |
| Gallery | GalleryCaCubit | GalleryCaState |
| ComponentOTP | ComponentOtpCubit | ComponentOtpState |
| ComponentPrint | ComponentPrintCubit | ComponentPrintState |
| ComponentScanDisplay | ComponentScanDisplayCubit | ComponentScanDisplayState |
| MultiSelect | MultiSelectCubit | MultiSelectState |
| List | ListCaCubit | ListCaState |
| MainMenu (legacy) | MainMenuCubit | MainMenuState |
| TaskPage (legacy) | TaskCubit | TaskState |
| TaskDetail (legacy) | TaskDetailCubit | TaskDetailState |
| SettingPage (legacy) | SettingPageCubit | SettingPageState |
| App (legacy) | AppCubit | AppState |

### 6.2 State Pattern

All states use **Freezed** for immutability:

```dart
@freezed
class TaskDoingState with _$TaskDoingState {
  const factory TaskDoingState.initial() = _Initial;
  const factory TaskDoingState.loading() = _Loading;
  const factory TaskDoingState.loaded({required TaskData data}) = _Loaded;
  const factory TaskDoingState.error({required String message}) = _Error;
  const factory TaskDoingState.success({required String message}) = _Success;
}
```

---

## 7. Localization

### 7.1 Supported Languages (7)

| Language | File | Locale Code |
|---|---|---|
| English | `en-US.json` | en-US |
| Indonesian | `id-ID.json` | id-ID |
| Thai | `th-TH.json` | th-TH |
| Japanese | `ja-JP.json` | ja-JP |
| Chinese (Simplified) | `zh-CN.json` | zh-CN |
| Filipino | `fil-PH.json` | fil-PH |
| Vietnamese | `vi-VN.json` | vi-VN |

### 7.2 Translation Statistics

- **Total keys:** 614 per language
- **Sections:** 3 (Labels, Error Messages, Information Messages)
- **Interpolation:** Supported (e.g., `{distance}`, `{hubName}`, `{maxPage}`)
- **Cross-references:** Supported (e.g., `@:failedToSaveFlowData.text`)

### 7.3 Key Translation Categories

| Category | Example Keys |
|---|---|
| Auth | `login`, `logout`, `password`, `forgetPassword`, `otpCode`, `authenticating` |
| Task | `tasks`, `taskListEmpty`, `createTask`, `finishTask`, `noTask`, `taskNotFound` |
| Check-In | `startTrip`, `endTrip`, `checkInHint`, `checkInDataNotFound`, `dontForgetToEndTrip` |
| Components | `photo`, `video`, `camera`, `voiceRecordRequired`, `scanMultipleQrBarcode` |
| Sync | `sync`, `syncData`, `dataSyncFailed`, `dataSyncInProcess`, `synchronizingData` |
| Errors | `anErrorOccurred`, `checkInternetConnectionMessage`, `noDataFound` |
| Org | `changeOrganization`, `selectOrganization`, `failedToChangeOrganization` |
| Hub | `hubNotFound`, `youDoNotHaveAHub`, `userDontHaveHub` |
| Permissions | `noAddTaskPermission`, `noDoTaskPermission`, `noViewEntityDataPermission` |
| Route | `optimizingRoute`, `routeOptimizationFailed`, `routeOptConfirm` |
| History | `history`, `historyEmpty`, `historyRetentionInfo` |
| Troubleshooting | `troubleshooting`, `troubleshootingCheckingMessage` |
| Road Warning | `roadWarning` |
| Sync Failure | `syncFailureTaskDataLabel`, `syncFailureUploadLabel`, `syncFailureReassignedLabel` |

---

## 8. Web App UI Structure (Inferred)

> **Status: INFERRED** — Web app source not extracted. Reconstructed from shared assets, translation files, and backend API.

### 8.1 Web App Hierarchy

```
Web App (web.mile.app) — INFERRED
  │
  ├─ Auth
  │    ├─ Login page (email + password)
  │    ├─ QR login (scan from mobile via LoginMileWithToken)
  │    └─ OTP/TOTP (shared auth with mobile)
  │
  ├─ Dashboard
  │    ├─ Overview stats (task completion, field worker status)
  │    ├─ Map view (real-time fleet tracking)
  │    └─ Alerts/notifications
  │
  ├─ Task Management
  │    ├─ Create task / assign to field worker
  │    ├─ Flow builder (drag-drop components: photo, video, input, etc.)
  │    ├─ Task list (filter by hub, org, status, assignee)
  │    └─ Task detail view (results, media, GPS trail)
  │
  ├─ Organization Management
  │    ├─ Organization list + switch
  │    ├─ Hub management (create, edit, geofence)
  │    └─ User management (roles, permissions, assign to hub)
  │
  ├─ Data Management
  │    ├─ Data Source types
  │    ├─ Entity data (master data)
  │    ├─ Currency settings
  │    ├─ Custom modules (org-specific)
  │    └─ Data version control
  │
  ├─ Flow Configuration
  │    ├─ Flow builder (task templates)
  │    ├─ Component configuration
  │    ├─ Subflow / subpage configuration
  │    └─ Page webhook configuration
  │
  ├─ Monitoring
  │    ├─ Location history (field worker GPS trail)
  │    ├─ Check-in/check-out logs
  │    ├─ Sync failure reports
  │    └─ Troubleshooting logs
  │
  ├─ Reports
  │    ├─ Task completion reports
  │    ├─ Field worker performance
  │    ├─ Bill/cost reports
  │    └─ Custom reports
  │
  └─ Settings
       ├─ Organization configuration
       ├─ Webhook configuration
       ├─ API keys
       ├─ Notification settings
       └─ User profile
```

---

## 9. Mobile & Web UI Mapping

| Function | Mobile UI | Web UI (Inferred) |
|---|---|---|
| Login | LoginPage (email + password) | Login page + QR scan option |
| QR Login | ProfileQRPage (scan QR) | QR code display |
| Organization switch | ChangeOrganizationPage | Org selector dropdown |
| Hub selection | HubChooserPage | Hub management + selector |
| Task list | TaskPage (Ongoing/Done tabs) | Task table with filters |
| Task detail | TaskDetailPage → TaskDoing flow | Task detail view (read-only results) |
| Task creation | TaskCreatePage (select flow) | Flow builder + task assignment |
| Check-in/out | CheckInOutPage (geofence) | Check-in logs + reports |
| Map | TaskMapPageClean (Google Maps) | Fleet map dashboard |
| Route optimization | RouteOptimizationPage | Route planning view |
| History | HistoryPage | History table + export |
| Settings | SettingPage | Admin settings panel |
| Data source | DataTypeChooserPage, EntityFeaturePage | Data source management CRUD |
| Sync failure | SyncFailureReportSheetWidget | Sync monitoring dashboard |
| Troubleshooting | TroubleshootingPage | Device health dashboard |
| WebView | WebViewPage (embedded web) | N/A (web app itself) |
| Custom module | MoreMenuPage → CustomModule | Custom module builder |
| Bill | BillPage (items + costs) | Bill report view |
| Photo/Video | CameraPage, VideoCameraPage | Media gallery view |
| OTP | OTPPage, ComponentOTPPage | OTP config for tasks |
| Cloud Auth | CloudAuthenticatorPage | TOTP management |
| Privacy Policy | PrivacyPolicyPage | Privacy policy page |
| Road Warning | RoadWarningPage (HMS e-license) | Road warning config |

---

## 10. Design System

### 10.1 Typography

| Font | Weights | Usage |
|---|---|---|
| NunitoSans | Regular, Bold, SemiBold, ExtraBold, Light, Italic | All text |

### 10.2 Iconography

| Icon Set | File | Count |
|---|---|---|
| Custom Icons | `CustomIcons.ttf` | 35 icons |
| Task Icons | `ic_task.svg`, `ic_home.svg`, `ic_location.svg`, `ic_route.svg` | SVG |
| App Icon | `Icon-192.png`, `ic_launcher_playstore.png` | PNG |

### 10.3 Imagery

| Image | File | Purpose |
|---|---|---|
| Mile Logo | `logo_mile.png` | Brand logo |
| Y3 Logo | `y3_logo_purple2.png` | Y3 tenant logo |
| Empty Task | `empty_task.png` | Empty state illustration |
| Login Success | `check_success_login.png` | Login success illustration |

### 10.4 Color Scheme

- Primary: Purple-based (Y3 branding)
- App names: "Mile Field" (prod), "Mile Field Beta/Dev/Sandbox"
- Copyright: "© 2024 • PT. Paket Informasi Digital • Indonesia"

### 10.5 App Identity

| Flavor | App Name |
|---|---|
| Production | Mile Field |
| Beta | Mile Field Beta |
| Dev | Mile Field Dev |
| Sandbox | Mile Field Sandbox |

---

*End of UI Breakdown*
