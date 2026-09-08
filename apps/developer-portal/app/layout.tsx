import type { Metadata, Viewport } from "next";
import "@fontsource/geist/600.css";
import "@fontsource/geist/700.css";
import "@fontsource/public-sans/400.css";
import "@fontsource/public-sans/500.css";
import "@fontsource/public-sans/600.css";
import "@fontsource/public-sans/700.css";
import "./globals.css";

export const metadata: Metadata = {
  title: {
    default: "Altius-FTM Developer Portal",
    template: "%s · Altius-FTM Docs",
  },
  description:
    "Product guides and API reference for Altius-FTM (Field Task Manager) — for customer IT and integrators.",
};

export const viewport: Viewport = { themeColor: "#00677e" };

/**
 * Root layout — locale-specific chrome lives under `[locale]`.
 */
export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  );
}
