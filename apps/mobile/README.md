# Altius Field (mobile)

Flutter driver app for Altius FTM. Offline-first SQLite with an immutable event outbox; Keycloak PKCE for live sign-in.

## Auth / OAuth redirect (Android & iOS)

Canonical redirect URI (must match Keycloak client `altius-mobile`):

```text
com.altius.altiusfield:/oauthredirect
```

| Surface | Registration |
|---|---|
| Android | `RedirectUriReceiverActivity` intent-filter: scheme `com.altius.altiusfield`, path `/oauthredirect` |
| iOS | `CFBundleURLSchemes` → `com.altius.altiusfield` in `ios/Runner/Info.plist` |
| Dart | `--dart-define=KEYCLOAK_REDIRECT_URI=...` (default above) |

**Why Android exports the redirect activity:** Custom-scheme OAuth returns through the browser as a `VIEW`/`BROWSABLE` intent. The receiver must be `android:exported="true"` or Keycloak PKCE cannot complete. Integrity is enforced by PKCE (`code_verifier` stays on-device), not by hiding the activity. Do not set `exported="false"` on `RedirectUriReceiverActivity`.

`MainActivity` is also exported for `MAIN`/`LAUNCHER` only (required on API 31+); it is not a deep-link surface.

## Live vs demo

```bash
# Demo (local SQLite fixtures; no IdP)
flutter run -t lib/main_dev.dart

# Live Keycloak + API
flutter run -t lib/main_dev.dart \
  --dart-define=API_BASE=http://127.0.0.1:8080 \
  --dart-define=KEYCLOAK_ISSUER=http://127.0.0.1:8081/realms/altius \
  --dart-define=KEYCLOAK_CLIENT_ID=altius-mobile \
  --dart-define=KEYCLOAK_REDIRECT_URI=com.altius.altiusfield:/oauthredirect
```

Production entrypoint (`lib/main_prod.dart`) requires Keycloak configuration (`demoWorkspace: false`).

## Quality

```bash
flutter analyze
flutter test
```
