import { notFound } from "next/navigation";
import { ScalarApiReference } from "@/components/scalar-api-reference";
import { uiCopy } from "@/lib/i18n";
import { isLocale, type Locale } from "@/lib/locales";

type ApiPageProps = {
  params: Promise<{ locale: string }>;
};

/**
 * Full-bleed Scalar API reference page.
 */
export default async function ApiPage({ params }: ApiPageProps) {
  const { locale: raw } = await params;
  if (!isLocale(raw)) notFound();
  const locale: Locale = raw;
  const copy = uiCopy(locale);

  return (
    <main>
      <div className="border-b border-outline-variant/60 bg-surface-container-low px-4 py-4 sm:px-6 lg:px-8">
        <div className="mx-auto max-w-[1400px]">
          <h1 className="text-headline-sm text-on-surface">{copy.apiTitle}</h1>
          <p className="mt-1 text-body-md text-on-surface-variant">{copy.apiLead}</p>
        </div>
      </div>
      <ScalarApiReference />
    </main>
  );
}
