"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import type { Locale } from "@/lib/locales";

type LocaleSwitcherProps = {
  locale: Locale;
};

/**
 * Switches between `en` and `id` while preserving the path after the locale segment.
 */
export function LocaleSwitcher({ locale }: LocaleSwitcherProps) {
  const pathname = usePathname() || `/${locale}`;
  const segments = pathname.split("/");
  const rest = segments.slice(2).join("/");
  const enHref = rest ? `/en/${rest}` : "/en";
  const idHref = rest ? `/id/${rest}` : "/id";

  return (
    <div className="flex items-center rounded-lg border border-outline-variant bg-surface-container-lowest p-0.5 text-body-sm">
      <Link
        href={enHref}
        className={
          locale === "en"
            ? "rounded-md bg-primary px-2.5 py-1 font-medium text-on-primary"
            : "rounded-md px-2.5 py-1 text-on-surface-variant hover:text-on-surface"
        }
        hrefLang="en"
        lang="en"
      >
        EN
      </Link>
      <Link
        href={idHref}
        className={
          locale === "id"
            ? "rounded-md bg-primary px-2.5 py-1 font-medium text-on-primary"
            : "rounded-md px-2.5 py-1 text-on-surface-variant hover:text-on-surface"
        }
        hrefLang="id"
        lang="id"
      >
        ID
      </Link>
    </div>
  );
}
