import Link from "next/link";
import { uiCopy } from "@/lib/i18n";
import type { Locale } from "@/lib/locales";
import { supportEmail } from "@/lib/web-app";

type Props = {
  locale: Locale;
};

/**
 * Site footer with security link and optional support mailto.
 */
export function SiteFooter({ locale }: Props) {
  const copy = uiCopy(locale);
  const email = supportEmail();

  return (
    <footer className="mt-auto border-t border-outline-variant/60 bg-surface-container-low">
      <div className="mx-auto flex max-w-[1400px] flex-col gap-3 px-4 py-8 text-sm text-on-surface-variant sm:flex-row sm:items-center sm:justify-between sm:px-6 lg:px-8">
        <p>{copy.footerCopyright}</p>
        <div className="flex flex-wrap gap-4">
          <Link href={`/${locale}/guides/security`} className="hover:text-primary">
            {copy.footerSecurity}
          </Link>
          {email ? (
            <a href={`mailto:${email}`} className="hover:text-primary">
              {email}
            </a>
          ) : (
            <span title="Set NEXT_PUBLIC_SUPPORT_EMAIL">{copy.footerSupport}</span>
          )}
        </div>
      </div>
    </footer>
  );
}
