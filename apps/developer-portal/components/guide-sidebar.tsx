"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { motion } from "motion/react";
import { GUIDE_SLUGS, navLabels } from "@/lib/nav";
import type { Locale } from "@/lib/locales";

type GuideSidebarProps = {
  locale: Locale;
};

/**
 * Left nav tree for guides — shared slugs with per-locale labels.
 */
export function GuideSidebar({ locale }: GuideSidebarProps) {
  const pathname = usePathname() || "";
  const labels = navLabels(locale);
  const base = `/${locale}/guides`;

  return (
    <nav aria-label={labels.guides} className="space-y-1">
      <p className="mb-3 px-2 text-label-bold uppercase tracking-wider text-on-surface-variant">
        {labels.guides}
      </p>
      <ul className="space-y-0.5">
        {GUIDE_SLUGS.map((slug) => {
          const href = `${base}/${slug}`;
          const active = pathname === href || pathname.endsWith(`/guides/${slug}`);
          return (
            <li key={slug} className="relative">
              {active && (
                <motion.span
                  layoutId="guide-sidebar-active"
                  className="absolute inset-y-0 left-0 w-0.5 rounded-full bg-primary"
                  transition={{ type: "spring", stiffness: 380, damping: 30 }}
                />
              )}
              <Link
                href={href}
                className={
                  active
                    ? "block rounded-lg bg-secondary-container/40 px-3 py-2 text-body-md font-medium text-primary no-underline"
                    : "block rounded-lg px-3 py-2 text-body-md text-on-surface-variant no-underline hover:bg-surface-container hover:text-on-surface"
                }
              >
                {labels[slug]}
              </Link>
            </li>
          );
        })}
      </ul>
      <div className="mt-6 space-y-0.5 border-t border-outline-variant/50 pt-4">
        <Link
          href={`/${locale}/api`}
          className="block rounded-lg px-3 py-2 text-body-md text-on-surface-variant no-underline hover:bg-surface-container hover:text-on-surface"
        >
          {labels.api}
        </Link>
        <Link
          href={`/${locale}/changelog`}
          className="block rounded-lg px-3 py-2 text-body-md text-on-surface-variant no-underline hover:bg-surface-container hover:text-on-surface"
        >
          {labels.changelog}
        </Link>
      </div>
    </nav>
  );
}
