import 'dart:convert';
import 'package:flutter_appauth/flutter_appauth.dart';
import 'package:flutter_secure_storage/flutter_secure_storage.dart';

class AuthService {
  final FlutterAppAuth _appAuth = const FlutterAppAuth();
  final FlutterSecureStorage _secure = const FlutterSecureStorage(
    iOptions: IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device),
  );

  static const _kIssuer = 'NEXT_PUBLIC_KEYCLOAK_ISSUER';
  static const _kClientId = 'NEXT_PUBLIC_KEYCLOAK_CLIENT_ID';
  static const _kRedirectUrl = 'NEXT_PUBLIC_KEYCLOAK_REDIRECT_URI';
  static const _kApiBase = 'NEXT_PUBLIC_API_BASE';

  /// Authenticate the driver with Keycloak using authorization code + PKCE.
  /// Returns the access token when the flow completes successfully.
  Future<String> login() async {
    final issuer = await _secure.read(key: _kIssuer);
    final clientId = await _secure.read(key: _kClientId);
    final redirectUrl = await _secure.read(key: _kRedirectUrl);
    if (issuer == null || clientId == null || redirectUrl == null) {
      throw StateError('Keycloak configuration not configured');
    }

    final result = await _appAuth.authorizeAndExchangeCode(
      AuthorizationTokenRequest(
        clientId,
        redirectUrl,
        discoveryUrl: '$issuer/.well-known/openid-configuration',
        scopes: ['openid', 'profile', 'email'],
        promptValues: ['login'],
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

  /// Refresh the access token if a refresh token is stored.
  Future<String?> refresh() async {
    final refreshToken = await _secure.read(key: 'refreshToken');
    final issuer = await _secure.read(key: _kIssuer);
    final clientId = await _secure.read(key: _kClientId);
    final redirectUrl = await _secure.read(key: _kRedirectUrl);
    if (refreshToken == null || issuer == null || clientId == null || redirectUrl == null) return null;

    final result = await _appAuth.token(
      TokenRequest(
        clientId,
        redirectUrl,
        discoveryUrl: '$issuer/.well-known/openid-configuration',
        refreshToken: refreshToken,
        grantType: 'refresh_token',
        scopes: ['openid', 'profile', 'email'],
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
    await _secure.write(key: _kIssuer, value: issuer);
    await _secure.write(key: _kClientId, value: clientId);
    await _secure.write(key: _kRedirectUrl, value: redirectUri);
    await _secure.write(key: _kApiBase, value: apiBase);
  }

  /// Return the configured API base URL.
  Future<String?> apiBase() => _secure.read(key: _kApiBase);

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

  /// Whether a session is active.
  Future<bool> isSignedIn() async => await _secure.read(key: 'session') == 'true';

  /// Clear all stored tokens.
  Future<void> logout() async {
    await _secure.delete(key: 'accessToken');
    await _secure.delete(key: 'refreshToken');
    await _secure.delete(key: 'idToken');
    await _secure.write(key: 'accessTokenExpiresAt', value: '');
    await _secure.write(key: 'session', value: '');
  }
}
