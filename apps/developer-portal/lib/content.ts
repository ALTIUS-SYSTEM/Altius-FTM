import fs from "node:fs/promises";
import path from "node:path";
import matter from "gray-matter";
import { GUIDE_SLUGS, type GuideSlug } from "./nav";
import type { Locale } from "./locales";

const CONTENT_ROOT = path.join(process.cwd(), "content");

export type GuideFrontmatter = {
  title: string;
  description: string;
  order?: number;
};

export type GuideDocument = {
  slug: GuideSlug;
  frontmatter: GuideFrontmatter;
  content: string;
};

/**
 * Reads a guide MDX file for the given locale and slug.
 */
export async function getGuide(locale: Locale, slug: GuideSlug): Promise<GuideDocument> {
  const filePath = path.join(CONTENT_ROOT, locale, `${slug}.mdx`);
  const raw = await fs.readFile(filePath, "utf8");
  const { data, content } = matter(raw);
  return {
    slug,
    frontmatter: {
      title: String(data.title ?? slug),
      description: String(data.description ?? ""),
      order: typeof data.order === "number" ? data.order : undefined,
    },
    content,
  };
}

/**
 * Lists all guides for a locale in nav order.
 */
export async function listGuides(locale: Locale): Promise<GuideDocument[]> {
  const docs = await Promise.all(GUIDE_SLUGS.map((slug) => getGuide(locale, slug)));
  return docs;
}

/**
 * Reads the changelog MDX for a locale.
 */
export async function getChangelog(locale: Locale): Promise<{
  frontmatter: GuideFrontmatter;
  content: string;
}> {
  const filePath = path.join(CONTENT_ROOT, locale, "changelog.mdx");
  const raw = await fs.readFile(filePath, "utf8");
  const { data, content } = matter(raw);
  return {
    frontmatter: {
      title: String(data.title ?? "Changelog"),
      description: String(data.description ?? ""),
    },
    content,
  };
}
