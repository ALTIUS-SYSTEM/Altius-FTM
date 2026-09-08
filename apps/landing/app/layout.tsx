import type { Metadata, Viewport } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Altius — Field Task Management",
  description: "Dispatch, track, and report field operations from one workspace. Altius FTM coordinates drivers, routes, and daily reports.",
};

export const viewport: Viewport = { themeColor: "#00677e" };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body>{children}</body>
    </html>
  );
}
