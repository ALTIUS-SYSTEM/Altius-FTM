/**
 * Origin of the public Altius-FTM developer portal (guides + API reference).
 */
export function docsOrigin(): string {
  const raw = process.env.NEXT_PUBLIC_DOCS_URL?.trim();
  if (!raw) return "http://127.0.0.1:3200";
  return raw.replace(/\/$/, "");
}

/** Docs home (default English locale). */
export function docsHomeUrl(): string {
  return `${docsOrigin()}/en`;
}

/** Interactive OpenAPI reference on the portal. */
export function docsApiUrl(): string {
  return `${docsOrigin()}/en/api`;
}
