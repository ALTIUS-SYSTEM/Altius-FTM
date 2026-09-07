"use client";

/** Keycloak authorization-code + PKCE flow for the dashboard. */

const STORAGE = {
  verifier: "altius.pkce.verifier",
  access: "altius.token.access",
  refresh: "altius.token.refresh",
  expires: "altius.token.expires",
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

const randomString = (length: number) => {
  const bytes = crypto.getRandomValues(new Uint8Array(length));
  return Array.from(bytes, (b) => (b % 62).toString(36).padStart(1, "0"))
    .join("")
    .slice(0, length);
};

/** Redirect the browser to the Keycloak login page. */
export async function startLogin(cfg: AuthConfig, redirectUri: string) {
  const verifier = base64url(crypto.getRandomValues(new Uint8Array(32)).buffer);
  sessionStorage.setItem(STORAGE.verifier, verifier);
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
    state: randomString(16),
  });
  window.location.assign(`${endpoints(cfg).authorize}?${params}`);
}

/** Exchange the authorization code for tokens. Call once on the callback. */
export async function finishLogin(
  cfg: AuthConfig,
  code: string,
  redirectUri: string,
): Promise<void> {
  const verifier = sessionStorage.getItem(STORAGE.verifier);
  if (!verifier) throw new Error("PKCE verifier missing — restart the login");
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
  await storeTokens(await res.json());
  sessionStorage.removeItem(STORAGE.verifier);
}

const MS_PER_SECOND = 1000;
const REFRESH_WINDOW_MS = 30_000;

const storeTokens = (t: {
  access_token: string;
  refresh_token?: string;
  expires_in: number;
}) => {
  sessionStorage.setItem(STORAGE.access, t.access_token);
  if (t.refresh_token) sessionStorage.setItem(STORAGE.refresh, t.refresh_token);
  sessionStorage.setItem(STORAGE.expires, String(Date.now() + t.expires_in * MS_PER_SECOND));
};

/** Access token; refreshes transparently when within 30 s of expiry. */
export async function accessToken(cfg: AuthConfig): Promise<string | null> {
  const token = sessionStorage.getItem(STORAGE.access);
  const expires = Number(sessionStorage.getItem(STORAGE.expires) ?? 0);
  if (!token) return null;
  if (Date.now() < expires - REFRESH_WINDOW_MS) return token;

  const refresh = sessionStorage.getItem(STORAGE.refresh);
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
    logout();
    return null;
  }
  await storeTokens(await res.json());
  return sessionStorage.getItem(STORAGE.access);
}

export function logout() {
  Object.values(STORAGE).forEach((k) => sessionStorage.removeItem(k));
}

export interface JwtClaims {
  sub?: string;
  preferred_username?: string;
  realm_access?: { roles?: string[] };
  resource_access?: Record<string, { roles?: string[] }>;
}

export function decodeJwt(token: string): JwtClaims {
  try {
    const payload = token.split(".")[1];
    const json = atob(payload.replace(/-/g, "+").replace(/_/g, "/"));
    return JSON.parse(json) as JwtClaims;
  } catch {
    return {};
  }
}
