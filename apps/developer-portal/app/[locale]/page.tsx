import Link from "next/link";
import { notFound } from "next/navigation";
import { ContentFade } from "@/components/content-fade";
import { isLocale, type Locale } from "@/lib/locales";
import { uiCopy } from "@/lib/i18n";
import { navLabels } from "@/lib/nav";

type HomePageProps = {
  params: Promise<{ locale: string }>;
};

/**
 * Docs home — brand, one headline, CTAs for Guides and API.
 */
export default async function LocaleHomePage({ params }: HomePageProps) {
  const { locale: raw } = await params;
  if (!isLocale(raw)) notFound();
  const locale: Locale = raw;
  const copy = uiCopy(locale);
  const labels = navLabels(locale);

  return (
    <main className="relative overflow-hidden">
      <div
        aria-hidden="true"
        className="pointer-events-none absolute inset-0 -z-10"
        style={{
          background:
            "radial-gradient(ellipse 80% 50% at 50% -10%, rgba(15, 180, 218, 0.18), transparent 60%), radial-gradient(ellipse 60% 40% at 80% 20%, rgba(0, 103, 126, 0.12), transparent 50%), linear-gradient(180deg, #f5fafd 0%, #eff4f8 100%)",
        }}
      />
      <ContentFade>
        <div className="mx-auto flex max-w-3xl flex-col gap-6 px-4 py-20 sm:px-6 sm:py-28 lg:px-8">
          <p className="text-label-bold uppercase tracking-widest text-primary">Altius-FTM</p>
          <h1 className="text-display-lg text-on-surface">{copy.homeHeadline}</h1>
          <p className="max-w-2xl text-body-lg text-on-surface-variant">{copy.homeLead}</p>
          <div className="mt-2 flex flex-wrap gap-3">
            <Link
              href={`/${locale}/guides`}
              className="rounded-lg bg-primary px-5 py-2.5 text-body-md font-medium text-on-primary no-underline hover:bg-on-primary-fixed-variant"
            >
              {copy.ctaGuides}
            </Link>
            <Link
              href={`/${locale}/api`}
              className="rounded-lg border border-outline-variant bg-surface-container-lowest px-5 py-2.5 text-body-md font-medium text-on-surface no-underline hover:border-primary hover:text-primary"
            >
              {copy.ctaApi}
            </Link>
          </div>
          <p className="mt-8 text-body-sm text-on-surface-variant">
            {labels.productLabel} · EN / ID
          </p>
        </div>
      </ContentFade>
    </main>
  );
}
