import Link from "next/link";
import { notFound } from "next/navigation";
import { ContentFade } from "@/components/content-fade";
import { GuideSidebar } from "@/components/guide-sidebar";
import { listGuides } from "@/lib/content";
import { uiCopy } from "@/lib/i18n";
import { isLocale, type Locale } from "@/lib/locales";
import { navLabels } from "@/lib/nav";

type GuidesIndexProps = {
  params: Promise<{ locale: string }>;
};

/**
 * Guide index with sidebar and article cards.
 */
export default async function GuidesIndexPage({ params }: GuidesIndexProps) {
  const { locale: raw } = await params;
  if (!isLocale(raw)) notFound();
  const locale: Locale = raw;
  const guides = await listGuides(locale);
  const copy = uiCopy(locale);
  const labels = navLabels(locale);

  return (
    <div className="mx-auto grid max-w-[1400px] gap-8 px-4 py-10 sm:px-6 lg:grid-cols-[240px_1fr] lg:px-8">
      <aside className="lg:sticky lg:top-20 lg:self-start">
        <GuideSidebar locale={locale} />
      </aside>
      <ContentFade>
        <main>
          <h1 className="text-headline-lg text-on-surface">{copy.guidesIndexTitle}</h1>
          <p className="mt-2 max-w-2xl text-body-lg text-on-surface-variant">
            {copy.guidesIndexLead}
          </p>
          <ul className="mt-10 space-y-3">
            {guides.map((guide) => (
              <li key={guide.slug}>
                <Link
                  href={`/${locale}/guides/${guide.slug}`}
                  className="block rounded-xl border border-outline-variant/60 bg-surface-container-lowest px-5 py-4 no-underline transition-colors hover:border-primary/40"
                >
                  <span className="text-body-lg font-medium text-on-surface">
                    {labels[guide.slug]}
                  </span>
                  <p className="mt-1 text-body-md text-on-surface-variant">
                    {guide.frontmatter.description}
                  </p>
                </Link>
              </li>
            ))}
          </ul>
        </main>
      </ContentFade>
    </div>
  );
}
