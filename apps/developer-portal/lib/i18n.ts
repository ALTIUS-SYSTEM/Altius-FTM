import type { Locale } from "./locales";

type UiCopy = {
  homeHeadline: string;
  homeLead: string;
  ctaGuides: string;
  ctaApi: string;
  guidesIndexTitle: string;
  guidesIndexLead: string;
  footerSecurity: string;
  footerSupport: string;
  footerCopyright: string;
  apiTitle: string;
  apiLead: string;
};

const UI_EN: UiCopy = {
  homeHeadline: "Build on Altius-FTM",
  homeLead:
    "Guides for customer IT and integrators — product surfaces, identity, sync, and the HTTP API — in English and Bahasa Indonesia.",
  ctaGuides: "Browse guides",
  ctaApi: "Open API reference",
  guidesIndexTitle: "Guides",
  guidesIndexLead: "Start with overview, then follow identity, clients, and API auth.",
  footerSecurity: "Security reporting",
  footerSupport: "Support",
  footerCopyright: "© 2026 Altius-FTM · PT Antero Daemon Technologies",
  apiTitle: "API reference",
  apiLead: "Interactive OpenAPI for Altius-FTM `/api/v3`.",
};

const UI_ID: UiCopy = {
  homeHeadline: "Bangun di atas Altius-FTM",
  homeLead:
    "Panduan untuk IT pelanggan dan integrator — permukaan produk, identitas, sinkronisasi, dan HTTP API — dalam bahasa Inggris dan Indonesia.",
  ctaGuides: "Jelajahi panduan",
  ctaApi: "Buka referensi API",
  guidesIndexTitle: "Panduan",
  guidesIndexLead: "Mulai dari ringkasan, lalu identitas, klien, dan autentikasi API.",
  footerSecurity: "Pelaporan keamanan",
  footerSupport: "Dukungan",
  footerCopyright: "© 2026 Altius-FTM · PT Antero Daemon Technologies",
  apiTitle: "Referensi API",
  apiLead: "OpenAPI interaktif untuk Altius-FTM `/api/v3`.",
};

/**
 * Returns UI chrome strings for the locale.
 */
export function uiCopy(locale: Locale): UiCopy {
  return locale === "id" ? UI_ID : UI_EN;
}
