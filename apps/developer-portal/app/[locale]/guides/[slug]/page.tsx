import { notFound } from "next/navigation";
import { MDXRemote } from "next-mdx-remote/rsc";
import { ContentFade } from "@/components/content-fade";
import { GuideSidebar } from "@/components/guide-sidebar";
import { mdxComponents } from "@/components/mdx-components";
import { Prose } from "@/components/prose";
import { getGuide } from "@/lib/content";
import { isLocale, type Locale } from "@/lib/locales";
import { GUIDE_SLUGS, isGuideSlug, type GuideSlug } from "@/lib/nav";

type GuidePageProps = {
  params: Promise<{ locale: string; slug: string }>;
};

/**
 * Static params for all locale × guide combinations.
 */
export function generateStaticParams() {
  const locales = ["en", "id"] as const;
  return locales.flatMap((locale) =>
    GUIDE_SLUGS.map((slug) => ({ locale, slug })),
  );
}

/**
 * Single guide article rendered from MDX.
 */
export default async function GuidePage({ params }: GuidePageProps) {
  const { locale: raw, slug } = await params;
  if (!isLocale(raw) || !isGuideSlug(slug)) notFound();
  const locale: Locale = raw;
  const guideSlug: GuideSlug = slug;

  let doc;
  try {
    doc = await getGuide(locale, guideSlug);
  } catch {
    notFound();
  }

  return (
    <div className="mx-auto grid max-w-[1400px] gap-8 px-4 py-10 sm:px-6 lg:grid-cols-[240px_1fr] lg:px-8">
      <aside className="lg:sticky lg:top-20 lg:self-start">
        <GuideSidebar locale={locale} />
      </aside>
      <ContentFade>
        <article>
          <p className="text-label-bold uppercase tracking-wider text-primary">
            {locale === "id" ? "Panduan" : "Guide"}
          </p>
          <h1 className="mt-2 text-headline-lg text-on-surface">{doc.frontmatter.title}</h1>
          {doc.frontmatter.description ? (
            <p className="mt-2 max-w-2xl text-body-lg text-on-surface-variant">
              {doc.frontmatter.description}
            </p>
          ) : null}
          <div className="mt-8">
            <Prose>
              <MDXRemote source={doc.content} components={mdxComponents} />
            </Prose>
          </div>
        </article>
      </ContentFade>
    </div>
  );
}
