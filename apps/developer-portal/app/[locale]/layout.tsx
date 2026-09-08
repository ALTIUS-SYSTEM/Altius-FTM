import { notFound } from "next/navigation";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { isLocale, type Locale } from "@/lib/locales";

type Props = {
  children: React.ReactNode;
  params: Promise<{ locale: string }>;
};

/**
 * Locale shell: header + main + footer. Invalid locale → 404.
 */
export default async function LocaleLayout({ children, params }: Props) {
  const { locale: raw } = await params;
  if (!isLocale(raw)) notFound();
  const locale: Locale = raw;

  return (
    <div className="flex min-h-screen flex-col bg-background text-on-background">
      <div
        aria-hidden
        className="pointer-events-none fixed inset-x-0 top-0 h-72 opacity-70"
        style={{
          background:
            "radial-gradient(ellipse 70% 50% at 50% -10%, rgba(15, 180, 218, 0.14), transparent 70%), radial-gradient(ellipse 50% 40% at 80% 0%, rgba(0, 103, 126, 0.1), transparent 60%)",
        }}
      />
      <SiteHeader locale={locale} />
      <div className="relative z-10 flex flex-1 flex-col">{children}</div>
      <SiteFooter locale={locale} />
    </div>
  );
}
