"use client";

/** Keycloak authorization-code + PKCE flow for the dashboard. */

/** One-shot PKCE values only. Tokens are never written to web storage. */
const STORAGE = {
  verifier: "altius.pkce.verifier",
  state: "altius.pkce.state",
  /** Set once a silent attempt has been made and not yet succeeded. Stops a
   *  reload loop when the IdP has no SSO session to hand back. */
  silentTried: "altius.auth.silent-tried",
  /** Where the user was when the silent attempt interrupted them. */
  returnTo: "altius.auth.return-to",
} as const;

export interface AuthConfig {
  url: string;
  realm: string;
  clientId: string;
}

export const authConfig = (): AuthConfig | null => {
  const url = process.env.NEXT_PUBLIC_KEYCLOAK_URL;
  const realm = process.env.NEXT_PUBLIC_KEYCLOAK_REALM;
  const clientId = process.env.NEXT_PUBLIC_KEYCLOAK_CLIENT_ID;
  return url && realm && clientId ? { url, realm, clientId } : null;
};

const endpoints = (c: AuthConfig) => ({
  authorize: `${c.url}/realms/${c.realm}/protocol/openid-connect/auth`,
  token: `${c.url}/realms/${c.realm}/protocol/openid-connect/token`,
});

const base64url = (buf: ArrayBuffer) =>
  btoa(String.fromCharCode(...new Uint8Array(buf)))
    .replace(/\+/g, "-")
    .replace(/\//g, "_")
    .replace(/=+$/, "");

/** 256 bits from the CSPRNG, base64url-encoded. */
const randomToken = () =>
  base64url(crypto.getRandomValues(new Uint8Array(32)).buffer);

/** Clear the single-use login parameters. Safe to call on any exit path. */
export function clearLoginState() {
  sessionStorage.removeItem(STORAGE.verifier);
  sessionStorage.removeItem(STORAGE.state);
}

/**
 * Silent re-auth bookkeeping.
 *
 * Tokens live in memory only, so a reload always starts signed out. Rather than
 * persist a refresh token where a script could read it, the dashboard asks
 * Keycloak to re-issue from its own SSO session. `tried` is the loop guard:
 * once an attempt comes back empty-handed, no further attempt is made until a
 * real sign-in succeeds.
 */
export const silentAuth = {
  tried: () => {
    try { return sessionStorage.getItem(STORAGE.silentTried) === "1"; } catch { return true; }
  },
  markTried: () => {
    try { sessionStorage.setItem(STORAGE.silentTried, "1"); } catch { /* storage unavailable */ }
  },
  reset: () => {
    try { sessionStorage.removeItem(STORAGE.silentTried); } catch { /* storage unavailable */ }
  },
  rememberReturn: (path: string) => {
    // Only same-site paths: anything else would make this an open redirect.
    if (!path.startsWith("/") || path.startsWith("//")) return;
    try { sessionStorage.setItem(STORAGE.returnTo, path); } catch { /* storage unavailable */ }
  },
  takeReturn: (): string | null => {
    try {
      const value = sessionStorage.getItem(STORAGE.returnTo);
      sessionStorage.removeItem(STORAGE.returnTo);
      return value && value.startsWith("/") && !value.startsWith("//") ? value : null;
    } catch { return null; }
  },
};

/**
 * Redirect the browser to Keycloak.
 *
 * With `prompt: "none"` this is a silent attempt: Keycloak answers immediately
 * from its own SSO cookie, returning either a code or `error=login_required`,
 * and never shows a form. A top-level navigation is used rather than a hidden
 * iframe on purpose — browsers now block third-party cookies in iframes, and
 * the IdP is a different origin from the dashboard, so the iframe form of this
 * flow fails for exactly the users it is meant to help.
 */
export async function startLogin(cfg: AuthConfig, redirectUri: string, opts?: { prompt?: "none" }) {
  const verifier = randomToken();
  const state = randomToken();
  sessionStorage.setItem(STORAGE.verifier, verifier);
  // Persist the state so the callback can prove this response answers a login
  // *this* tab started. Generating it and never checking it leaves the flow
  // with no request-forgery defence at all.
  sessionStorage.setItem(STORAGE.state, state);
  const challenge = base64url(
    await crypto.subtle.digest("SHA-256", new TextEncoder().encode(verifier)),
  );
  const params = new URLSearchParams({
    client_id: cfg.clientId,
    redirect_uri: redirectUri,
    response_type: "code",
    scope: "openid profile email",
    code_challenge: challenge,
    code_challenge_method: "S256",
    state,
  });
  if (opts?.prompt === "none") params.set("prompt", "none");
  window.location.assign(`${endpoints(cfg).authorize}?${params}`);
}

/** Exchange the authorization code for tokens. Call once on the callback.
 * Returns the access token so the callback can read claims without
 * re-reading storage (tokens are memory-only). */
export async function finishLogin(
  cfg: AuthConfig,
  code: string,
  redirectUri: string,
  returnedState: string | null,
): Promise<string> {
  const verifier = sessionStorage.getItem(STORAGE.verifier);
  const expectedState = sessionStorage.getItem(STORAGE.state);
  try {
    if (!verifier || !expectedState)
      throw new Error("Login session missing — restart the login");
    if (!returnedState || returnedState !== expectedState)
      throw new Error("Login state mismatch — restart the login");
    return await exchangeCode(cfg, code, redirectUri, verifier);
  } finally {
    // One-shot values: clear on success and on every failure, so an abandoned
    // login cannot leave a live verifier for a forced callback to spend.
    sessionStorage.removeItem(STORAGE.verifier);
    sessionStorage.removeItem(STORAGE.state);
  }
}

async function exchangeCode(
  cfg: AuthConfig,
  code: string,
  redirectUri: string,
  verifier: string,
): Promise<string> {
  const res = await fetch(endpoints(cfg).token, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "authorization_code",
      client_id: cfg.clientId,
      redirect_uri: redirectUri,
      code,
      code_verifier: verifier,
    }),
  });
  if (!res.ok) throw new Error(`token exchange failed: ${res.status}`);
  const body = (await res.json()) as {
    access_token: string;
    refresh_token?: string;
    expires_in: number;
  };
  storeTokens(body);
  return body.access_token;
}

const MS_PER_SECOND = 1000;
const REFRESH_WINDOW_MS = 30_000;

/**
 * Tokens live in memory, not `sessionStorage`.
 *
 * This is a public OAuth client (no client secret), so a persisted refresh
 * token is independently replayable against Keycloak from any machine — one
 * moment of script execution on this origin would turn into durable account
 * access. In memory, an XSS can steal at most the current short-lived access
 * token, and a reload re-authenticates silently against the Keycloak SSO
 * session instead of reading a long-lived credential back out of storage.
 */
let tokens: { access: string; refresh?: string; expiresAt: number } | null = null;
/** De-duplicates concurrent refreshes; see `accessToken`. */
let inflight: Promise<string | null> | null = null;

const storeTokens = (t: {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
}) => {
  tokens = {
    access: t.access_token,
    refresh: t.refresh_token,
    expiresAt: Date.now() + t.expires_in * MS_PER_SECOND,
  };
};

/** True when a live session exists; the route guard needs this, not a flag. */
export const hasSession = () => tokens !== null;

/** Current access token without refresh — for display/claims after login. */
export const peekAccessToken = () => tokens?.access ?? null;

async function refreshTokens(cfg: AuthConfig): Promise<string | null> {
  const refresh = tokens?.refresh;
  if (!refresh) return null;
  const res = await fetch(endpoints(cfg).token, {
    method: "POST",
    headers: { "content-type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "refresh_token",
      client_id: cfg.clientId,
      refresh_token: refresh,
    }),
  });
  if (!res.ok) {
    // Only clear if nobody else has already rotated us onto a fresh token.
    // Keycloak revokes the old refresh token on use, so a losing racer must
    // not tear down the winner's freshly issued credentials.
    if (tokens?.refresh === refresh) logout();
    return null;
  }
  storeTokens(await res.json());
  return tokens?.access ?? null;
}

/** Access token; refreshes transparently when within 30 s of expiry. */
export async function accessToken(cfg: AuthConfig): Promise<string | null> {
  if (!tokens) return null;
  if (Date.now() < tokens.expiresAt - REFRESH_WINDOW_MS) return tokens.access;
  // A dashboard mount fires several API calls at once. Without this, each one
  // spends the same rotated refresh token and all but one gets a 400 — then
  // calls logout() and destroys the winner's valid session.
  inflight ??= refreshTokens(cfg).finally(() => {
    inflight = null;
  });
  return inflight;
}

export function logout() {
  tokens = null;
  sessionStorage.removeItem(STORAGE.verifier);
  sessionStorage.removeItem(STORAGE.state);
  sessionStorage.removeItem(STORAGE.returnTo);
  // Deliberately marked, not reset: after signing out the SSO session is gone,
  // so an immediate silent attempt would only add a redirect before the login
  // form the user is already on their way to.
  silentAuth.markTried();
}

/** End the Keycloak SSO session too, so a leaked token cannot be refreshed. */
export function endSession(cfg: AuthConfig, redirectUri: string) {
  logout();
  const params = new URLSearchParams({
    client_id: cfg.clientId,
    post_logout_redirect_uri: redirectUri,
  });
  window.location.assign(
    `${cfg.url}/realms/${cfg.realm}/protocol/openid-connect/logout?${params}`,
  );
}

export interface JwtClaims {
  sub?: string;
  preferred_username?: string;
  /** Present because the authorize request asks for `profile` and `email`. */
  name?: string;
  given_name?: string;
  family_name?: string;
  email?: string;
  realm_access?: { roles?: string[] };
  resource_access?: Record<string, { roles?: string[] }>;
}

/**
 * Decodes without verifying the signature. Display only — never use the result
 * to grant access. The API re-validates every token against the realm JWKS.
 */
export function decodeJwt(token: string): JwtClaims {
  try {
    const payload = token.split(".")[1];
    const json = atob(payload.replace(/-/g, "+").replace(/_/g, "/"));
    return JSON.parse(json) as JwtClaims;
  } catch {
    return {};
  }
}
