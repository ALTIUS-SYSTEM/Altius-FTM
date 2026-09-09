import type { Metadata } from "next";
import "@fontsource/geist/600.css";
import "@fontsource/geist/700.css";
import "@fontsource/public-sans/400.css";
import "@fontsource/public-sans/500.css";
import "@fontsource/public-sans/600.css";
import "@fontsource/public-sans/700.css";
import "./globals.css";
import { DemoProvider } from "@/components/demo-provider";

export const metadata: Metadata = { title: "Altius · Field Operations", description: "Altius FTM operations workspace — tasks, routes, driver reports and GPS review." };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  // Browser extensions inject attributes into <body> before React hydrates
  // (ColorZilla's `cz-shortcut-listen`, password managers, dark-mode add-ons),
  // which the app cannot prevent or predict. suppressHydrationWarning applies
  // one level deep only, so genuine mismatches inside DemoProvider still warn.
  return <html lang="en"><body suppressHydrationWarning><DemoProvider>{children}</DemoProvider></body></html>;
}
