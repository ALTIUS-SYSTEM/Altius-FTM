"use client";

import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import {
  Route,
  ClipboardList,
  MapPin,
  BarChart3,
  Sparkles,
  LifeBuoy,
  FileText,
  Menu,
  X,
  MessageCircle,
  Moon,
  Sun,
} from "lucide-react";
import { docsHomeUrl } from "@/lib/docs";
import { webLoginUrl } from "@/lib/web-app";

export function Navigation2() {
  const loginHref = webLoginUrl();
  const docsHref = docsHomeUrl();
  const [activeMenu, setActiveMenu] = useState<string | null>(null);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [dark, setDark] = useState(false);

  useEffect(() => {
    const stored = window.localStorage.getItem("altius-theme");
    const enabled =
      stored === "dark" ||
      (stored === null &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
    setDark(enabled);
    document.documentElement.classList.toggle("dark", enabled);
  }, []);

  const toggleDark = () => {
    const next = !dark;
    setDark(next);
    document.documentElement.classList.toggle("dark", next);
    window.localStorage.setItem("altius-theme", next ? "dark" : "light");
  };

  const menuItems = {
    Platform: [
      {
        icon: ClipboardList,
        title: "Dispatch",
        description: "Assign and sequence field tasks",
        href: "#features",
      },
      {
        icon: Route,
        title: "Routing",
        description: "Optimized visit order with ETA/ATA",
        href: "#features",
      },
      {
        icon: MapPin,
        title: "Tracking",
        description: "Geofence and GPS anomaly review",
        href: "#how-it-works",
      },
      {
        icon: BarChart3,
        title: "Reporting",
        description: "Daily driver reports, on record",
        href: "#stats",
      },
    ],
    Resources: [
      {
        icon: Sparkles,
        title: "What's New",
        description: "Latest updates and features",
        href: "#blog",
      },
      {
        icon: LifeBuoy,
        title: "Help and Support",
        description: "Get help when you need it",
        href: "#faq",
      },
      {
        icon: FileText,
        title: "Altius-FTM Docs",
        description: "Guides for IT and integrators",
        href: docsHref,
      },
      {
        icon: MessageCircle,
        title: "Contact",
        description: "Talk to the team",
        href: "#faq",
      },
    ],
  };

  return (
    <div className="min-h-[var(--rb-section-min-h,100vh)] w-full relative bg-white dark:bg-neutral-950">
      <div
        className="absolute inset-0 z-0 bg-linear-to-b from-cyan-100/50 via-transparent to-transparent dark:from-transparent dark:via-transparent dark:to-transparent"
        style={{
          background:
            "radial-gradient(ellipse 80% 60% at 50% 0%, rgba(15, 180, 218, 0.15), transparent 70%), radial-gradient(ellipse 80% 60% at 50% 0%, rgba(0, 103, 126, 0.18), transparent 70%)",
        }}
      />
      <div
        className="absolute inset-0 z-0 dark:block hidden"
        style={{
          background:
            "radial-gradient(ellipse 80% 60% at 50% 0%, rgba(15, 180, 218, 0.18), transparent 70%), #0a0a0a",
        }}
      />

      <nav className="relative w-full px-4 sm:px-6 lg:px-8 py-6 sm:py-8 z-10">
        <div className="mx-auto w-full max-w-[1400px]">
          <motion.div
            className="relative mx-auto hidden lg:block"
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, ease: [0.4, 0, 0.2, 1] }}
            onMouseLeave={() => setActiveMenu(null)}
          >
            <div className="mx-auto w-fit rounded-3xl bg-white/40 backdrop-blur-2xl border border-neutral-200/50 shadow-xl dark:bg-neutral-950/20 dark:border-neutral-800/50 overflow-hidden">
              <div className="flex items-center justify-between gap-2 pl-6 pr-3 py-3">
                <a href="#" className="flex items-center mr-6" aria-label="Altius">
                  <img src="/altius-logo.svg" alt="Altius" className="h-8 w-auto dark:hidden" />
                  <img src="/altius-logo-white.svg" alt="Altius" className="h-8 w-auto hidden dark:block" />
                </a>

                <div className="flex items-center gap-1">
                  <button
                    onMouseEnter={() => setActiveMenu("Platform")}
                    className="px-4 py-2 text-sm tracking-tight font-light text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white rounded-full"
                  >
                    Platform
                  </button>
                  <button
                    onMouseEnter={() => setActiveMenu("Resources")}
                    className="px-4 py-2 text-sm tracking-tight font-light text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white rounded-full"
                  >
                    Resources
                  </button>
                  <a
                    href="#stats"
                    className="px-4 py-2 text-sm tracking-tight font-light text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white rounded-full no-underline"
                    onMouseEnter={() => setActiveMenu(null)}
                  >
                    Pricing
                  </a>
                </div>

                <div className="flex items-center gap-2 ml-6">
                  <button
                    type="button"
                    onClick={toggleDark}
                    aria-label={dark ? "Switch to light mode" : "Switch to dark mode"}
                    className="p-2 rounded-full text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white"
                  >
                    {dark ? <Sun className="w-4 h-4" /> : <Moon className="w-4 h-4" />}
                  </button>
                  <a
                    href={loginHref}
                    className="px-4 py-2 tracking-tight text-sm font-light text-neutral-600 dark:text-neutral-400 hover:text-neutral-900 dark:hover:text-white no-underline"
                    onMouseEnter={() => setActiveMenu(null)}
                  >
                    Log in
                  </a>
                  <a
                    href={loginHref}
                    className="px-5 py-2 rounded-lg bg-[#00677e] dark:bg-white text-white dark:text-neutral-950 text-sm font-light tracking-tight hover:bg-[#005565] dark:hover:bg-neutral-200 no-underline"
                    onMouseEnter={() => setActiveMenu(null)}
                  >
                    Start Dispatching
                  </a>
                </div>
              </div>

              <AnimatePresence>
                {activeMenu && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                    className="overflow-hidden"
                  >
                    <div className="p-2">
                      <div className="grid grid-cols-2 gap-3 w-[620px]">
                        {menuItems[activeMenu as keyof typeof menuItems].map(
                          (item, index) => {
                            const Icon = item.icon;
                            return (
                              <motion.a
                                key={item.title}
                                href={item.href}
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{
                                  duration: 0.2,
                                  delay: index * 0.05,
                                  ease: "easeOut",
                                }}
                                className="group flex items-start gap-3 rounded-2xl bg-white/20 backdrop-blur-2xl dark:bg-neutral-950/20 border border-neutral-300 dark:border-neutral-800/50 p-4 hover:border-cyan-300 dark:hover:border-cyan-800 hover:shadow-md transition-[border-color,box-shadow] duration-200"
                              >
                                <div className="shrink-0 rounded-lg bg-cyan-50 dark:bg-neutral-800 p-2">
                                  <Icon className="w-5 h-5 text-[#00677e] dark:text-cyan-300" />
                                </div>
                                <div className="flex-1 min-w-0">
                                  <h3 className="text-sm font-normal text-neutral-900 dark:text-white mb-0.5 group-hover:text-neutral-700 dark:group-hover:text-neutral-200 transition-colors">
                                    {item.title}
                                  </h3>
                                  <p className="text-xs text-neutral-600 dark:text-neutral-400 leading-snug">
                                    {item.description}
                                  </p>
                                </div>
                              </motion.a>
                            );
                          },
                        )}
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </motion.div>

          <motion.div
            className="lg:hidden"
            initial={{ opacity: 0, y: -20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, ease: [0.4, 0, 0.2, 1] }}
          >
            <div className="rounded-3xl bg-white/40 backdrop-blur-2xl border border-neutral-300 shadow-xl dark:bg-neutral-950/20 dark:border-neutral-800/50 overflow-hidden">
              <div className="flex items-center justify-between pl-4 pr-3 py-3">
                <a href="#" className="flex items-center" aria-label="Altius">
                  <img src="/altius-logo.svg" alt="Altius" className="h-8 w-auto dark:hidden" />
                  <img src="/altius-logo-white.svg" alt="Altius" className="h-8 w-auto hidden dark:block" />
                </a>

                <button
                  onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
                  className="flex h-10 w-10 items-center justify-center rounded-lg bg-[#00677e] dark:bg-white text-white dark:text-neutral-950"
                  aria-label={mobileMenuOpen ? "Close menu" : "Open menu"}
                  aria-expanded={mobileMenuOpen}
                >
                  {mobileMenuOpen ? (
                    <X className="h-5 w-5" />
                  ) : (
                    <Menu className="h-5 w-5" />
                  )}
                </button>
              </div>

              <AnimatePresence>
                {mobileMenuOpen && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                    className="overflow-hidden"
                  >
                    <div className="px-4 pb-4 pt-2">
                      <div className="space-y-4">
                        <div className="space-y-1">
                          <a
                            href="#stats"
                            className="block py-2 px-2 text-sm font-medium text-neutral-900 dark:text-white no-underline"
                          >
                            Pricing
                          </a>
                          <a
                            href={loginHref}
                            className="block py-2 px-2 text-sm font-medium text-neutral-600 dark:text-neutral-400 no-underline"
                          >
                            Log in
                          </a>
                        </div>

                        <div>
                          <a
                            href={loginHref}
                            className="block w-full text-center px-6 py-2.5 rounded-full bg-[#00677e] dark:bg-white text-white dark:text-neutral-950 text-sm font-medium no-underline"
                          >
                            Start Dispatching
                          </a>
                        </div>

                        <div className="pt-2 border-neutral-200 dark:border-neutral-800">
                          <h3 className="text-sm font-bold text-neutral-900 dark:text-white mb-2 px-2">
                            Platform
                          </h3>
                          <div className="space-y-2">
                            {menuItems.Platform.map((item) => {
                              const Icon = item.icon;
                              return (
                                <a
                                  key={item.title}
                                  href={item.href}
                                  className="flex items-start gap-3 rounded-xl bg-white/20 backdrop-blur-2xl dark:bg-neutral-950/20 border border-neutral-200/50 dark:border-neutral-800/50 p-3 no-underline"
                                >
                                  <div className="shrink-0 rounded-lg bg-cyan-50 dark:bg-neutral-800 p-2">
                                    <Icon className="w-4 h-4 text-[#00677e] dark:text-cyan-300" />
                                  </div>
                                  <div className="flex-1 min-w-0">
                                    <h4 className="text-sm font-semibold text-neutral-900 dark:text-white mb-0.5">
                                      {item.title}
                                    </h4>
                                    <p className="text-xs text-neutral-600 dark:text-neutral-400">
                                      {item.description}
                                    </p>
                                  </div>
                                </a>
                              );
                            })}
                          </div>
                        </div>

                        <div>
                          <h3 className="text-sm font-bold text-neutral-900 dark:text-white mb-2 px-2">
                            Resources
                          </h3>
                          <div className="space-y-2">
                            {menuItems.Resources.map((item) => {
                              const Icon = item.icon;
                              return (
                                <a
                                  key={item.title}
                                  href={item.href}
                                  className="flex items-start gap-3 rounded-xl bg-white/20 backdrop-blur-2xl dark:bg-neutral-950/20 border border-neutral-200/50 dark:border-neutral-800/50 p-3 no-underline"
                                >
                                  <div className="shrink-0 rounded-lg bg-cyan-50 dark:bg-neutral-800 p-2">
                                    <Icon className="w-4 h-4 text-[#00677e] dark:text-cyan-300" />
                                  </div>
                                  <div className="flex-1 min-w-0">
                                    <h4 className="text-sm font-semibold text-neutral-900 dark:text-white mb-0.5">
                                      {item.title}
                                    </h4>
                                    <p className="text-xs text-neutral-600 dark:text-neutral-400">
                                      {item.description}
                                    </p>
                                  </div>
                                </a>
                              );
                            })}
                          </div>
                        </div>
                      </div>
                    </div>
                  </motion.div>
                )}
              </AnimatePresence>
            </div>
          </motion.div>
        </div>
      </nav>
    </div>
  );
}

export default Navigation2;
