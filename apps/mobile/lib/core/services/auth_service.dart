import 'dart:convert';
import 'dart:io';
import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

/// Keycloak authorization-code + PKCE for the driver app.
///
/// Config and tokens live in [FlutterSecureStorage] only — never in SQLite
/// preferences. Storage key names are app-owned (`auth.*`); they are unrelated
/// to web `NEXT_PUBLIC_*` build env vars.
class AuthService {
  final FlutterAppAuth _appAuth = const FlutterAppAuth();
  final FlutterSecureStorage _secure = const FlutterSecureStorage(
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
  );

  static const _kIssuer = 'auth.issuer';
  static const _kClientId = 'auth.clientId';
  static const _kRedirectUrl = 'auth.redirectUri';
  static const _kApiBase = 'auth.apiBase';

  /// Legacy keys from an earlier mistaken `NEXT_PUBLIC_*` naming. Read once
  /// and migrate so upgrades do not force drivers to re-enter config.
  static const _legacyIssuer = 'NEXT_PUBLIC_KEYCLOAK_ISSUER';
  static const _legacyClientId = 'NEXT_PUBLIC_KEYCLOAK_CLIENT_ID';
  static const _legacyRedirectUrl = 'NEXT_PUBLIC_KEYCLOAK_REDIRECT_URI';
  static const _legacyApiBase = 'NEXT_PUBLIC_API_BASE';

  Future<String?> _readConfig(String key, String legacyKey) async {
    final current = await _secure.read(key: key);
    if (current != null && current.isNotEmpty) return current;
    final legacy = await _secure.read(key: legacyKey);
    if (legacy == null || legacy.isEmpty) return null;
    await _secure.write(key: key, value: legacy);
    await _secure.delete(key: legacyKey);
    return legacy;
  }

  /// Authenticate the driver with Keycloak using authorization code + PKCE.
  /// Returns the access token when the flow completes successfully.
  Future<String> login() async {
    final issuer = await _readConfig(_kIssuer, _legacyIssuer);
    final clientId = await _readConfig(_kClientId, _legacyClientId);
    final redirectUrl = await _readConfig(_kRedirectUrl, _legacyRedirectUrl);
    if (issuer == null || issuer.isEmpty) {
      throw StateError('authConfigMissing');
    }
    if (clientId == null || clientId.isEmpty) {
      throw StateError('authConfigMissing');
    }
    if (redirectUrl == null || redirectUrl.isEmpty) {
      throw StateError('authConfigMissing');
    }

    final result = await _appAuth.authorizeAndExchangeCode(
      AuthorizationTokenRequest(
        clientId,
        redirectUrl,
        discoveryUrl: '$issuer/.well-known/openid-configuration',
        scopes: ['openid', 'profile', 'email'],
        promptValues: ['login'],
        // AppAuth refuses plain http; only loopback dev issuers may use it.
        allowInsecureConnections: _isInsecureDevIssuer(issuer),
      ),
    );

    if (result.accessToken == null) {
      throw StateError('authentication cancelled');
    }

    await _secure.write(key: 'accessToken', value: result.accessToken!);
    if (result.refreshToken != null) {
      await _secure.write(key: 'refreshToken', value: result.refreshToken!);
    }
    if (result.idToken != null) {
      await _secure.write(key: 'idToken', value: result.idToken!);
    }
    if (result.accessTokenExpirationDateTime != null) {
      await _secure.write(key: 'accessTokenExpiresAt', value: result.accessTokenExpirationDateTime!.toIso8601String());
    }
    await _secure.write(key: 'session', value: 'true');
    return result.accessToken!;
  }

  /// In-app login: resource-owner password grant straight to the token
  /// endpoint — no browser round-trip. Stores the same keys as [login].
  Future<String> loginWithPassword(String username, String password) async {
    final issuer = await _readConfig(_kIssuer, _legacyIssuer);
    final clientId = await _readConfig(_kClientId, _legacyClientId);
    if (issuer == null || issuer.isEmpty || clientId == null || clientId.isEmpty) {
      throw StateError('authConfigMissing');
    }
    if (username.isEmpty || password.isEmpty) {
      throw StateError('authConfigMissing');
    }

    final client = HttpClient();
    try {
      final req = await client.postUrl(
        Uri.parse('$issuer/protocol/openid-connect/token'),
      );
      req.headers.set('content-type', 'application/x-www-form-urlencoded');
      req.write(Uri(queryParameters: {
        'grant_type': 'password',
        'client_id': clientId,
        'username': username,
        'password': password,
        'scope': 'openid profile email',
      }).query);
      final res = await req.close().timeout(const Duration(seconds: 15));
      final body = await res.transform(utf8.decoder).join();
      if (res.statusCode == 401 || res.statusCode == 400) {
        throw StateError('unauthorized');
      }
      if (res.statusCode != 200) throw StateError('serverError');
      final data = jsonDecode(body) as Map<String, dynamic>;
      final accessToken = data['access_token'] as String?;
      if (accessToken == null) throw StateError('unauthorized');

      await _secure.write(key: 'accessToken', value: accessToken);
      final refreshToken = data['refresh_token'] as String?;
      if (refreshToken != null) {
        await _secure.write(key: 'refreshToken', value: refreshToken);
      }
      final idToken = data['id_token'] as String?;
      if (idToken != null) {
        await _secure.write(key: 'idToken', value: idToken);
      }
      final expiresIn = data['expires_in'] as int?;
      if (expiresIn != null) {
        final at = DateTime.now().toUtc().add(Duration(seconds: expiresIn));
        await _secure.write(key: 'accessTokenExpiresAt', value: at.toIso8601String());
      }
      await _secure.write(key: 'session', value: 'true');
      return accessToken;
    } finally {
      client.close();
    }
  }

  /// Refresh the access token if a refresh token is stored.
  Future<String?> refresh() async {
    final refreshToken = await _secure.read(key: 'refreshToken');
    final issuer = await _readConfig(_kIssuer, _legacyIssuer);
    final clientId = await _readConfig(_kClientId, _legacyClientId);
    final redirectUrl = await _readConfig(_kRedirectUrl, _legacyRedirectUrl);
    if (refreshToken == null || issuer == null || clientId == null || redirectUrl == null) return null;

    final result = await _appAuth.token(
      TokenRequest(
        clientId,
        redirectUrl,
        discoveryUrl: '$issuer/.well-known/openid-configuration',
        refreshToken: refreshToken,
        grantType: 'refresh_token',
        scopes: ['openid', 'profile', 'email'],
        allowInsecureConnections: _isInsecureDevIssuer(issuer),
      ),
    );

    if (result.accessToken == null) return null;
    await _secure.write(key: 'accessToken', value: result.accessToken!);
    if (result.refreshToken != null) {
      await _secure.write(key: 'refreshToken', value: result.refreshToken!);
    }
    if (result.accessTokenExpirationDateTime != null) {
      await _secure.write(key: 'accessTokenExpiresAt', value: result.accessTokenExpirationDateTime!.toIso8601String());
    }
    return result.accessToken;
  }

  /// Return the current access token, refreshing if within 60 seconds of expiry.
  Future<String?> accessToken() async {
    final token = await _secure.read(key: 'accessToken');
    final expiresAt = await _secure.read(key: 'accessTokenExpiresAt');
    if (token == null) return null;
    if (expiresAt != null) {
      final expires = DateTime.tryParse(expiresAt);
      if (expires != null && DateTime.now().toUtc().isAfter(expires.subtract(const Duration(seconds: 60)))) {
        return refresh();
      }
    }
    return token;
  }

  /// Persist the Keycloak and API environment values needed for login.
  Future<void> configure({
    required String issuer,
    required String clientId,
    required String redirectUri,
    required String apiBase,
  }) async {
    if (issuer.isEmpty || clientId.isEmpty || redirectUri.isEmpty || apiBase.isEmpty) {
      throw StateError('authConfigMissing');
    }
    await _secure.write(key: _kIssuer, value: issuer);
    await _secure.write(key: _kClientId, value: clientId);
    await _secure.write(key: _kRedirectUrl, value: redirectUri);
    await _secure.write(key: _kApiBase, value: apiBase);
    // Drop legacy names so a later read cannot prefer stale values.
    await _secure.delete(key: _legacyIssuer);
    await _secure.delete(key: _legacyClientId);
    await _secure.delete(key: _legacyRedirectUrl);
    await _secure.delete(key: _legacyApiBase);
  }

  /// Return the configured API base URL.
  Future<String?> apiBase() => _readConfig(_kApiBase, _legacyApiBase);

  /// Read the raw JWT claims from the current access token.
  Future<Map<String, dynamic>?> claims() async {
    final token = await _secure.read(key: 'accessToken');
    if (token == null || token.isEmpty) return null;
    try {
      final parts = token.split('.');
      if (parts.length != 3) return null;
      final normalized = base64Url.normalize(parts[1]);
      final payload = utf8.decode(base64Url.decode(normalized));
      return jsonDecode(payload) as Map<String, dynamic>;
    } catch (_) {
      return null;
    }
  }

  /// Whether a Keycloak session flag is active.
  Future<bool> isSignedIn() async => await _secure.read(key: 'session') == 'true';

  /// AppAuth refuses plain http; allow it only for loopback dev issuers
  /// (local docker-compose Keycloak). A remote http issuer still throws.
  static bool _isInsecureDevIssuer(String issuer) {
    final uri = Uri.tryParse(issuer.trim());
    if (uri == null || uri.scheme != 'http') return false;
    final h = uri.host.toLowerCase();
    return h == 'localhost' || h == '127.0.0.1' || h == '::1' || h == '[::1]';
  }

  /// Clear all stored tokens (keeps IdP/API config for the next sign-in).
  Future<void> logout() async {
    await _secure.delete(key: 'accessToken');
    await _secure.delete(key: 'refreshToken');
    await _secure.delete(key: 'idToken');
    await _secure.delete(key: 'accessTokenExpiresAt');
    await _secure.write(key: 'session', value: '');
  }
}
