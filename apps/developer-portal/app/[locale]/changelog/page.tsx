import { notFound } from "next/navigation";
import { MDXRemote } from "next-mdx-remote/rsc";
import { ContentFade } from "@/components/content-fade";
import { mdxComponents } from "@/components/mdx-components";
import { Prose } from "@/components/prose";
import { getChangelog } from "@/lib/content";
import { isLocale, type Locale } from "@/lib/locales";
import { navLabels } from "@/lib/nav";

type ChangelogPageProps = {
  params: Promise<{ locale: string }>;
};

/**
 * Customer-facing release notes.
 */
export default async function ChangelogPage({ params }: ChangelogPageProps) {
  const { locale: raw } = await params;
  if (!isLocale(raw)) notFound();
  const locale: Locale = raw;
  const labels = navLabels(locale);

  let doc;
  try {
    doc = await getChangelog(locale);
  } catch {
    notFound();
  }

  return (
    <div className="mx-auto max-w-3xl px-4 py-12 sm:px-6 lg:px-8">
      <ContentFade>
        <article>
          <h1 className="text-headline-lg text-on-surface">{doc.frontmatter.title}</h1>
          <p className="mt-2 text-body-lg text-on-surface-variant">
            {doc.frontmatter.description || labels.changelog}
          </p>
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
