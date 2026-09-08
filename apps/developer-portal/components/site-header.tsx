"use client";

import Image from "next/image";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { navLabels } from "@/lib/nav";
import { isLocale, type Locale } from "@/lib/locales";
import { webLoginUrl } from "@/lib/web-app";

type Props = {
  locale: Locale;
};

/**
 * Top navigation: brand, Guides / API / Changelog, locale switch, dashboard CTA.
 */
export function SiteHeader({ locale }: Props) {
  const labels = navLabels(locale);
  const pathname = usePathname();
  const loginHref = webLoginUrl();

  const swapLocale = (next: Locale) => {
    const parts = pathname.split("/");
    if (parts.length >= 2 && isLocale(parts[1])) {
      parts[1] = next;
      return parts.join("/") || `/${next}`;
    }
    return `/${next}`;
  };

  const linkClass = (href: string) => {
    const active = pathname === href || pathname.startsWith(`${href}/`);
    return [
      "rounded-lg px-3 py-2 text-sm font-medium transition-colors",
      active
        ? "bg-secondary-container text-on-secondary-container"
        : "text-on-surface-variant hover:bg-surface-container-low hover:text-on-surface",
    ].join(" ");
  };

  return (
    <header className="sticky top-0 z-40 border-b border-outline-variant/60 bg-surface-container-lowest/90 backdrop-blur-md">
      <div className="mx-auto flex h-16 max-w-[1400px] items-center gap-4 px-4 sm:px-6 lg:px-8">
        <Link href={`/${locale}`} className="flex shrink-0 items-center gap-2.5">
          <Image src="/altius-logo.svg" alt="" width={32} height={32} priority />
          <span className="font-heading text-lg font-bold tracking-tight text-primary">
            Altius-FTM
            <span className="ml-1.5 hidden text-sm font-semibold text-on-surface-variant sm:inline">
              {labels.productShortLabel}
            </span>
          </span>
        </Link>

        <nav className="ml-2 hidden items-center gap-1 md:flex" aria-label="Primary">
          <Link href={`/${locale}/guides`} className={linkClass(`/${locale}/guides`)}>
            {labels.guides}
          </Link>
          <Link href={`/${locale}/api`} className={linkClass(`/${locale}/api`)}>
            {labels.api}
          </Link>
          <Link href={`/${locale}/changelog`} className={linkClass(`/${locale}/changelog`)}>
            {labels.changelog}
          </Link>
        </nav>

        <div className="ml-auto flex items-center gap-2 sm:gap-3">
          <div
            className="flex rounded-lg border border-outline-variant p-0.5 text-xs font-semibold"
            role="group"
            aria-label="Language"
          >
            {(["en", "id"] as const).map((code) => (
              <Link
                key={code}
                href={swapLocale(code)}
                className={[
                  "rounded-md px-2.5 py-1.5 uppercase",
                  locale === code
                    ? "bg-primary text-on-primary"
                    : "text-on-surface-variant hover:text-on-surface",
                ].join(" ")}
                hrefLang={code}
              >
                {code}
              </Link>
            ))}
          </div>
          <a
            href={loginHref}
            className="rounded-lg bg-primary px-3 py-2 text-sm font-semibold text-on-primary hover:bg-on-primary-fixed-variant"
          >
            {labels.openDashboard}
          </a>
        </div>
      </div>
    </header>
  );
}
