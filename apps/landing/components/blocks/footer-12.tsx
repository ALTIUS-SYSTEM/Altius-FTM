"use client";

import { motion, useReducedMotion, type Variants } from "motion/react";
import { ArrowRight } from "lucide-react";
import { docsApiUrl, docsHomeUrl } from "@/lib/docs";

const GithubIcon = () => (
  <svg viewBox="0 0 24 24" className="h-[18px] w-[18px] fill-current" aria-hidden="true">
    <path d="M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" />
  </svg>
);

const LinkedinIcon = () => (
  <svg viewBox="0 0 24 24" className="h-[18px] w-[18px] fill-current" aria-hidden="true">
    <path d="M20.447 20.452h-3.554v-5.569c0-1.328-.027-3.037-1.852-3.037-1.853 0-2.136 1.445-2.136 2.939v5.667H9.351V9h3.414v1.561h.046c.477-.9 1.637-1.85 3.37-1.85 3.601 0 4.267 2.37 4.267 5.455v6.286zM5.337 7.433c-1.144 0-2.063-.926-2.063-2.065 0-1.138.92-2.063 2.063-2.063 1.14 0 2.064.925 2.064 2.063 0 1.139-.925 2.065-2.064 2.065zm1.782 13.019H3.555V9h3.564v11.452zM22.225 0H1.771C.792 0 0 .774 0 1.729v20.542C0 23.227.792 24 1.771 24h20.451C23.2 24 24 23.227 24 22.271V1.729C24 .774 23.2 0 22.222 0h.003z" />
  </svg>
);

const focusRing =
  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#00677e] focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:focus-visible:ring-white dark:focus-visible:ring-offset-neutral-950";

type FooterLink = { label: string; href: string };

const navGroups: { title: string; links: FooterLink[] }[] = [
  {
    title: "Platform",
    links: [
      { label: "Dispatch", href: "#features" },
      { label: "Routing", href: "#features" },
      { label: "Tracking", href: "#how-it-works" },
      { label: "Reports", href: "#stats" },
    ],
  },
  {
    title: "Build",
    links: [
      { label: "Altius-FTM Docs", href: "__DOCS__" },
      { label: "API reference", href: "__DOCS_API__" },
      { label: "Examples", href: "__DOCS__" },
      { label: "Status", href: "#" },
    ],
  },
  {
    title: "Company",
    links: [
      { label: "About", href: "#about" },
      { label: "Careers", href: "#" },
      { label: "Customers", href: "#" },
      { label: "Contact", href: "#faq" },
    ],
  },
  {
    title: "Legal",
    links: [
      { label: "Privacy", href: "#" },
      { label: "Terms", href: "#" },
      { label: "Security", href: "#" },
      { label: "Cookies", href: "#" },
    ],
  },
];

const dispatches = ["Route radar", "Release notes", "Ops memo"];

const vitals = [
  { label: "Status", value: "All systems operational", live: true },
  { label: "Uptime", value: "99.98%, last 90 days" },
  { label: "Latest release", value: "v1.4. Geofence review" },
  { label: "Next dispatch", value: "Friday, 06:00 UTC" },
];

const container: Variants = {
  hidden: {},
  visible: { transition: { staggerChildren: 0.12, delayChildren: 0.05 } },
};

const item: Variants = {
  hidden: { opacity: 0, y: 24 },
  visible: {
    opacity: 1,
    y: 0,
    transition: { duration: 0.7, ease: [0.22, 1, 0.36, 1] },
  },
};

export default function Footer12() {
  const reduce = useReducedMotion();
  const docs = docsHomeUrl();
  const docsApi = docsApiUrl();
  const resolvedGroups = navGroups.map((group) => ({
    ...group,
    links: group.links.map((link) => ({
      ...link,
      href:
        link.href === "__DOCS__"
          ? docs
          : link.href === "__DOCS_API__"
            ? docsApi
            : link.href,
    })),
  }));

  return (
    <footer className="w-full bg-white px-4 py-16 dark:bg-neutral-950 sm:px-6 sm:py-20 lg:px-8 lg:py-24">
      <motion.div
        variants={container}
        initial={reduce ? false : "hidden"}
        whileInView="visible"
        viewport={{ once: true, margin: "-80px" }}
        className="mx-auto w-full max-w-[1400px]"
      >
        <motion.div
          variants={item}
          className="relative overflow-hidden rounded-3xl bg-[#00677e] p-7 ring-1 ring-[#00677e]/10 dark:ring-white/10 sm:p-10 lg:p-14"
        >
          <div
            aria-hidden="true"
            className="pointer-events-none absolute -right-24 -top-24 h-72 w-72 rounded-full bg-white/[0.06] blur-3xl"
          />
          <div className="relative grid grid-cols-1 gap-10 lg:grid-cols-[1.2fr_1fr] lg:gap-16">
            <div>
              <div className="flex items-center gap-3">
                <span className="flex h-10 w-10 items-center justify-center rounded-xl bg-white text-base font-semibold text-[#00677e]">
                  A
                </span>
                <span className="text-sm font-medium text-cyan-100">
                  Altius Field Dispatch
                </span>
              </div>
              <h2 className="mt-8 max-w-xl text-3xl font-medium leading-[1.12] tracking-tight text-white sm:text-4xl">
                The briefing for teams that move things between shifts.
              </h2>
              <p className="mt-4 max-w-md text-sm leading-relaxed text-cyan-100 sm:text-base">
                Release notes, routing playbooks, and the numbers that moved.
                One email, every other Friday.
              </p>
              <form
                className="mt-8 flex w-full max-w-md flex-col gap-2 rounded-2xl border border-white/10 bg-white/[0.06] p-2 transition-colors duration-200 focus-within:border-white/30 sm:flex-row sm:items-center sm:rounded-full sm:p-1.5 sm:pl-5"
                onSubmit={(e) => e.preventDefault()}
              >
                <label htmlFor="footer-12-email" className="sr-only">
                  Work email
                </label>
                <input
                  id="footer-12-email"
                  type="email"
                  placeholder="work@email.com"
                  className="min-w-0 flex-1 rounded-xl bg-transparent px-3 py-2 text-sm text-white placeholder:text-cyan-200/70 focus-visible:outline-none sm:px-0 sm:py-0"
                />
                <button
                  type="submit"
                  className="inline-flex cursor-pointer items-center justify-center gap-2 rounded-xl bg-white px-5 py-2.5 text-sm font-medium text-[#00677e] transition-colors duration-200 hover:bg-neutral-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-offset-2 focus-visible:ring-offset-[#00677e] sm:rounded-full"
                >
                  Subscribe
                  <ArrowRight className="h-4 w-4" />
                </button>
              </form>
              <div className="mt-5 flex flex-wrap gap-2">
                {dispatches.map((chip) => (
                  <span
                    key={chip}
                    className="rounded-full border border-white/15 px-3 py-1 text-xs font-medium text-cyan-100"
                  >
                    {chip}
                  </span>
                ))}
              </div>
            </div>

            <div className="flex flex-col justify-center border-t border-white/10 pt-10 lg:border-l lg:border-t-0 lg:pl-14 lg:pt-0">
              <dl className="divide-y divide-white/10">
                {vitals.map((vital) => (
                  <div
                    key={vital.label}
                    className="flex items-center justify-between gap-6 py-4 first:pt-0 last:pb-0"
                  >
                    <dt className="text-xs font-medium uppercase tracking-[0.16em] text-cyan-200/80">
                      {vital.label}
                    </dt>
                    <dd className="flex items-center gap-2.5 text-right text-sm font-medium text-white">
                      {vital.live && (
                        <span className="relative flex h-2.5 w-2.5">
                          {!reduce && (
                            <motion.span
                              aria-hidden="true"
                              className="absolute inline-flex h-full w-full rounded-full bg-emerald-300"
                              animate={{ scale: [1, 2.1], opacity: [0.5, 0] }}
                              transition={{
                                duration: 1.8,
                                repeat: Infinity,
                                ease: "easeOut",
                              }}
                            />
                          )}
                          <span className="relative inline-flex h-2.5 w-2.5 rounded-full bg-emerald-300" />
                        </span>
                      )}
                      {vital.value}
                    </dd>
                  </div>
                ))}
              </dl>
            </div>
          </div>
        </motion.div>

        <motion.div
          variants={item}
          className="mt-14 grid grid-cols-2 gap-x-8 gap-y-10 sm:grid-cols-4 lg:grid-cols-[1.35fr_1fr_1fr_1fr_1fr]"
        >
          <div className="col-span-2 sm:col-span-4 lg:col-span-1">
            <div className="flex items-center">
              <img src="/altius-logo.svg" alt="Altius" className="h-7 w-auto dark:hidden" />
              <img src="/altius-logo-white.svg" alt="Altius" className="h-7 w-auto hidden dark:block" />
            </div>
            <p className="mt-3 max-w-[28ch] text-sm leading-relaxed text-neutral-500 dark:text-neutral-400">
              Field task management for operators who move real things on real
              deadlines.
            </p>
          </div>
          {resolvedGroups.map((group) => (
            <nav key={group.title} aria-label={group.title} className="min-w-0">
              <h3 className="mb-5 text-xs font-medium uppercase tracking-[0.16em] text-neutral-500 dark:text-neutral-500">
                {group.title}
              </h3>
              <ul className="space-y-3">
                {group.links.map((link) => (
                  <li key={link.label}>
                    <a
                      href={link.href}
                      className={`rounded-sm text-sm text-neutral-600 transition-colors duration-200 hover:text-[#00677e] dark:text-neutral-400 dark:hover:text-cyan-300 ${focusRing}`}
                    >
                      {link.label}
                    </a>
                  </li>
                ))}
              </ul>
            </nav>
          ))}
        </motion.div>

        <motion.div
          variants={item}
          className="mt-14 flex flex-col gap-4 border-t border-neutral-200 pt-6 dark:border-neutral-800 sm:flex-row sm:items-center sm:justify-between"
        >
          <p className="text-sm text-neutral-500 dark:text-neutral-500">
            © 2026 Altius Field Task Management
          </p>
          <div className="flex items-center gap-1">
            <a
              href="#"
              aria-label="X profile"
              className="flex h-9 w-9 items-center justify-center rounded-full text-neutral-500 transition-colors duration-200 hover:bg-neutral-100 hover:text-neutral-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#00677e] dark:text-neutral-500 dark:hover:bg-neutral-900 dark:hover:text-white dark:focus-visible:ring-white"
            >
              <svg viewBox="0 0 24 24" className="h-4 w-4 fill-current">
                <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
              </svg>
            </a>
            <a
              href="#"
              aria-label="GitHub"
              className="flex h-9 w-9 items-center justify-center rounded-full text-neutral-500 transition-colors duration-200 hover:bg-neutral-100 hover:text-neutral-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#00677e] dark:text-neutral-500 dark:hover:bg-neutral-900 dark:hover:text-white dark:focus-visible:ring-white"
            >
              <GithubIcon />
            </a>
            <a
              href="#"
              aria-label="LinkedIn"
              className="flex h-9 w-9 items-center justify-center rounded-full text-neutral-500 transition-colors duration-200 hover:bg-neutral-100 hover:text-neutral-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#00677e] dark:text-neutral-500 dark:hover:bg-neutral-900 dark:hover:text-white dark:focus-visible:ring-white"
            >
              <LinkedinIcon />
            </a>
          </div>
        </motion.div>
      </motion.div>
    </footer>
  );
}
