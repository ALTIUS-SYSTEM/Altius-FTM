import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";

/**
 * Per-request CSP nonce.
 *
 * The App Router emits inline `<script>` tags to stream the RSC payload
 * (`self.__next_f.push(...)`). A static `script-src 'self'` header blocks them,
 * so React never hydrates and every page stops at its loading state — in
 * production as well as dev. A nonce is the only way to allow exactly those
 * scripts without `'unsafe-inline'`, which would readmit every injected script
 * and defeat the policy. It must be generated per request, so it cannot live in
 * `next.config.ts` headers; Next reads it back off this request header and
 * stamps its own tags with it.
 */
export function middleware(request: NextRequest) {
  const nonce = Buffer.from(crypto.randomUUID()).toString("base64");
  const isDev = process.env.NODE_ENV !== "production";

  // Origins the browser reaches directly: the Altius API and Keycloak (PKCE
  // token exchange and REST calls both happen from the page, not the server).
  const connectSrc = ["'self'", process.env.NEXT_PUBLIC_API_BASE, process.env.NEXT_PUBLIC_KEYCLOAK_URL]
    .filter(Boolean)
    .join(" ");

  const csp = [
    "default-src 'self'",
    "base-uri 'self'",
    "form-action 'self'",
    "frame-ancestors 'none'",
    "object-src 'none'",
    // `strict-dynamic` lets the nonced bootstrap load Next's own chunks without
    // enumerating each one. Dev additionally needs eval for React Refresh.
    `script-src 'self' 'nonce-${nonce}' 'strict-dynamic'${isDev ? " 'unsafe-eval'" : ""}`,
    // Next inlines critical CSS without a nonce; scripts get no such exemption.
    "style-src 'self' 'unsafe-inline'",
    "font-src 'self' data:",
    // Static maps arrive as blobs fetched from our own API.
    "img-src 'self' data: blob:",
    `connect-src ${connectSrc}${isDev ? " ws: wss:" : ""}`,
  ].join("; ");

  const headers = new Headers(request.headers);
  headers.set("x-nonce", nonce);
  headers.set("Content-Security-Policy", csp);

  const response = NextResponse.next({ request: { headers } });
  response.headers.set("Content-Security-Policy", csp);
  return response;
}

export const config = {
  matcher: [
    // Everything except Next's own static output and prefetch requests, which
    // carry no inline script and do not need the header.
    {
      source: "/((?!_next/static|_next/image|favicon.ico).*)",
      missing: [
        { type: "header", key: "next-router-prefetch" },
        { type: "header", key: "purpose", value: "prefetch" },
      ],
    },
  ],
};
