/**
 * Origin of the Altius web dashboard.
 *
 * Auth (Keycloak PKCE) runs only on the web app origin. The developer portal
 * never starts OAuth; it deep-links users into the dashboard login route.
 */
export function webAppOrigin(): string {
  const raw = process.env.NEXT_PUBLIC_WEB_APP_URL?.trim();
  if (!raw) return "http://127.0.0.1:3000";
  return raw.replace(/\/$/, "");
}

/** Dashboard login entry — Keycloak or local demo depending on web env. */
export function webLoginUrl(): string {
  return `${webAppOrigin()}/login`;
}

/**
 * Public API origin for Scalar server hints (no `/api/v3` suffix required).
 */
export function apiBaseUrl(): string {
  const raw = process.env.NEXT_PUBLIC_API_BASE?.trim();
  if (!raw) return "http://127.0.0.1:8080";
  return raw.replace(/\/$/, "");
}

/**
 * Customer support mailbox for the portal footer.
 *
 * No public support address is committed in-repo (LICENSE / SECURITY.md /
 * landing / deploy only name the company, not a mailbox). Ops sets this via
 * `NEXT_PUBLIC_SUPPORT_EMAIL` without a code change.
 */
export function supportEmail(): string {
  return process.env.NEXT_PUBLIC_SUPPORT_EMAIL?.trim() ?? "";
}
