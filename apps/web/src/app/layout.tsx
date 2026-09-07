import type { Metadata } from "next";
import "@fontsource/geist/600.css";
import "@fontsource/geist/700.css";
import "@fontsource/public-sans/400.css";
import "@fontsource/public-sans/500.css";
import "@fontsource/public-sans/600.css";
import "@fontsource/public-sans/700.css";
import "./globals.css";
import { DemoProvider } from "@/components/demo-provider";

export const metadata: Metadata = { title: "Altius · Field Operations Demo", description: "Altius FTM operations workspace — synthetic demo data only." };

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return <html lang="en"><body><DemoProvider>{children}</DemoProvider></body></html>;
}
