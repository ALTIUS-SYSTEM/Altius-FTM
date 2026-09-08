/**
 * Origin of the Altius web dashboard.
 *
 * Auth (Keycloak PKCE) runs only on the web app origin so the code verifier
 * stays in that origin's sessionStorage. The landing site never starts OAuth;
 * it only deep-links users into the dashboard login route.
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
