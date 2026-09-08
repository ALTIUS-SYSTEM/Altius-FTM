import type { Locale } from "./locales";

/** Guide slug shared across locales. */
export type GuideSlug =
  | "overview"
  | "getting-started"
  | "identity-and-access"
  | "web-dashboard"
  | "mobile-field"
  | "sync-and-data"
  | "api-authentication"
  | "security"
  | "troubleshooting";

/** Ordered guide list for sidebar and index pages. */
export const GUIDE_SLUGS: readonly GuideSlug[] = [
  "overview",
  "getting-started",
  "identity-and-access",
  "web-dashboard",
  "mobile-field",
  "sync-and-data",
  "api-authentication",
  "security",
  "troubleshooting",
] as const;

type NavLabels = Record<GuideSlug, string> & {
  guides: string;
  api: string;
  changelog: string;
  home: string;
  openDashboard: string;
  /** Full product + docs brand for standalone chrome (home, etc.). */
  productLabel: string;
  /** Short suffix beside the Altius-FTM wordmark in the header. */
  productShortLabel: string;
};

const NAV_EN: NavLabels = {
  home: "Home",
  guides: "Guides",
  api: "API",
  changelog: "Changelog",
  openDashboard: "Open dashboard",
  productLabel: "Altius-FTM Docs",
  productShortLabel: "Docs",
  overview: "Overview",
  "getting-started": "Getting started",
  "identity-and-access": "Identity & access",
  "web-dashboard": "Web dashboard",
  "mobile-field": "Mobile field app",
  "sync-and-data": "Sync & data",
  "api-authentication": "API authentication",
  security: "Security",
  troubleshooting: "Troubleshooting",
};

const NAV_ID: NavLabels = {
  home: "Beranda",
  guides: "Panduan",
  api: "API",
  changelog: "Catatan rilis",
  openDashboard: "Buka dasbor",
  productLabel: "Dokumen Altius-FTM",
  productShortLabel: "Dokumen",
  overview: "Ringkasan",
  "getting-started": "Mulai cepat",
  "identity-and-access": "Identitas & akses",
  "web-dashboard": "Dasbor web",
  "mobile-field": "Aplikasi lapangan",
  "sync-and-data": "Sinkronisasi & data",
  "api-authentication": "Autentikasi API",
  security: "Keamanan",
  troubleshooting: "Pemecahan masalah",
};

/**
 * Returns localized chrome labels for the given locale.
 */
export function navLabels(locale: Locale): NavLabels {
  return locale === "id" ? NAV_ID : NAV_EN;
}

/**
 * Returns true when `slug` is a known guide.
 */
export function isGuideSlug(slug: string): slug is GuideSlug {
  return (GUIDE_SLUGS as readonly string[]).includes(slug);
}
