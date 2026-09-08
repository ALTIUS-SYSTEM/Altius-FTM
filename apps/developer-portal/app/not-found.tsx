import Link from "next/link";

export default function NotFound() {
  return (
    <main className="mx-auto flex min-h-[60vh] max-w-lg flex-col items-center justify-center gap-4 px-6 text-center">
      <p className="text-label-bold uppercase tracking-widest text-on-surface-variant">404</p>
      <h1 className="text-headline-lg text-on-surface">Page not found</h1>
      <p className="text-body-lg text-on-surface-variant">
        That path is not part of the Altius-FTM developer portal.
      </p>
      <Link
        href="/en"
        className="mt-2 rounded-lg bg-primary px-5 py-2.5 text-sm font-medium text-on-primary hover:bg-on-primary-fixed-variant"
      >
        Back to docs home
      </Link>
    </main>
  );
}
