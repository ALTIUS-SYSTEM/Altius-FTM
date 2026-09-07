"use client";

import { useState } from "react";
import { ArrowUpRight, Plus } from "lucide-react";
import {
  AnimatePresence,
  motion,
  useReducedMotion,
  type Variants,
} from "motion/react";

const faqs = [
  {
    question: "Can Altius work offline for drivers in low-signal areas?",
    answer:
      "Yes. The driver app is offline-first: events are written to a local outbox on the device and sync when connectivity returns. Nothing a driver taps is lost.",
  },
  {
    question: "How does the geofence check-in work?",
    answer:
      "Each stop carries a hub or customer boundary. Arrival taps inside the radius confirm instantly; taps outside create a deviation review instead of blocking the driver.",
  },
  {
    question: "What does the daily driver report (LHS) include?",
    answer:
      "The day's visits, completed stops, operational costs by category, and exceptions — locked to an immutable record supervisors can review and approve.",
  },
  {
    question: "Which roles does the workspace support?",
    answer:
      "Admin, supervisor, and lead on the web dashboard, plus the driver role in the mobile app. Permissions are scoped per organization, hub, and module.",
  },
  {
    question: "Can we keep our existing GPS or telematics feed?",
    answer:
      "Altius compares app-reported positions with a vehicle GPS stream and flags divergences for human review. Bring your feed; we don't replace your hardware.",
  },
  {
    question: "How many languages are supported?",
    answer:
      "Seven today: English, Indonesian, Thai, Japanese, Simplified Chinese, Filipino, and Vietnamese. Drivers can switch languages from settings at any time.",
  },
  {
    question: "Can we export our data?",
    answer:
      "Task history, driver reports, and anomaly reviews export to CSV from the dashboard. A REST contract is available for systems that need structured sync.",
  },
  {
    question: "What happens when we add a new hub or region?",
    answer:
      "Hubs are configuration, not code. Create the hub, set its geofence, assign users and drivers — it appears in the workspace selector immediately.",
  },
];

const stats = [
  { value: "2h", label: "Median first reply" },
  { value: "98%", label: "Support CSAT" },
];

export default function FAQ9() {
  const reduce = useReducedMotion();
  const [openIndex, setOpenIndex] = useState(0);
  const columns = [faqs.slice(0, 4), faqs.slice(4)];

  const container: Variants = {
    hidden: {},
    show: { transition: { staggerChildren: 0.06, delayChildren: 0.05 } },
  };
  const item: Variants = {
    hidden: { opacity: 0, y: reduce ? 0 : 14 },
    show: {
      opacity: 1,
      y: 0,
      transition: { duration: 0.55, ease: [0.22, 1, 0.36, 1] },
    },
  };

  return (
    <section id="faq" className="w-full bg-white px-4 py-16 dark:bg-neutral-950 sm:px-6 sm:py-20 lg:px-8 lg:py-24">
      <div className="mx-auto w-full max-w-[1400px]">
        <div className="grid grid-cols-1 gap-8 lg:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)] lg:gap-12">
          <motion.aside
            initial={{ opacity: 0, y: reduce ? 0 : 18 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-80px" }}
            transition={{ duration: 0.65, ease: [0.22, 1, 0.36, 1] }}
            className="flex flex-col justify-between gap-12 rounded-3xl bg-[#00677e] p-7 ring-1 ring-[#00677e]/10 dark:ring-white/10 sm:p-10 lg:sticky lg:top-24 lg:self-start"
          >
            <div>
              <h2 className="text-3xl font-medium leading-[1.1] tracking-tighter text-white sm:text-4xl lg:text-5xl">
                Questions that need a dispatcher?
              </h2>
              <p className="mt-5 max-w-md text-base leading-relaxed text-cyan-100">
                The answers here cover the basics. For rollout planning,
                integrations, or fleet migrations, send us the messy context: a
                real person replies.
              </p>
            </div>
            <div>
              <div className="grid grid-cols-2 divide-x divide-white/10 border-y border-white/10">
                {stats.map((stat, index) => (
                  <div
                    key={stat.label}
                    className={`py-5 ${index === 0 ? "pr-6" : "pl-6"}`}
                  >
                    <p className="text-2xl font-semibold tracking-tight text-white sm:text-3xl">
                      {stat.value}
                    </p>
                    <p className="mt-1 text-sm text-cyan-100">
                      {stat.label}
                    </p>
                  </div>
                ))}
              </div>
              <a
                href="#"
                className="group mt-8 inline-flex w-full items-center justify-between rounded-full bg-white px-6 py-3.5 text-sm font-medium text-[#00677e] transition-colors duration-200 hover:bg-neutral-100 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-white focus-visible:ring-offset-2 focus-visible:ring-offset-[#00677e] sm:w-auto sm:min-w-56"
              >
                Email the team
                <ArrowUpRight className="h-4 w-4 text-neutral-500 transition-colors duration-200 group-hover:text-[#00677e]" />
              </a>
              <p className="mt-4 text-sm text-cyan-200">
                Weekdays 9:00–18:00 WIB, worldwide.
              </p>
            </div>
          </motion.aside>

          <motion.div
            variants={container}
            initial="hidden"
            whileInView="show"
            viewport={{ once: true, margin: "-80px" }}
            className="grid grid-cols-1 items-start gap-4 sm:grid-cols-2"
          >
            {columns.map((column, columnIndex) => (
              <div key={columnIndex} className="flex flex-col gap-4">
                {column.map((faq, indexInColumn) => {
                  const index = columnIndex * 4 + indexInColumn;
                  const isOpen = openIndex === index;
                  return (
                    <motion.div
                      key={faq.question}
                      variants={item}
                      className={`rounded-2xl border transition-colors duration-200 ${
                        isOpen
                          ? "border-cyan-300 bg-cyan-50 dark:border-cyan-800 dark:bg-neutral-900"
                          : "border-neutral-200 bg-white hover:border-cyan-300 dark:border-neutral-800 dark:bg-neutral-950 dark:hover:border-cyan-800"
                      }`}
                    >
                      <button
                        type="button"
                        aria-expanded={isOpen}
                        aria-controls={`faq9-answer-${index}`}
                        onClick={() => setOpenIndex(isOpen ? -1 : index)}
                        className="group flex w-full cursor-pointer items-start justify-between gap-4 rounded-2xl p-5 text-left focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#00677e] dark:focus-visible:ring-white sm:p-6"
                      >
                        <span className="flex-1 text-base font-medium leading-snug text-neutral-900 dark:text-white">
                          {faq.question}
                        </span>
                        <motion.span
                          animate={{ rotate: isOpen ? 45 : 0 }}
                          transition={{
                            duration: 0.35,
                            ease: [0.22, 1, 0.36, 1],
                          }}
                          className="mt-0.5 shrink-0 text-neutral-400 transition-colors duration-200 group-hover:text-[#00677e] dark:text-neutral-500 dark:group-hover:text-white"
                        >
                          <Plus className="h-4.5 w-4.5" />
                        </motion.span>
                      </button>
                      <AnimatePresence initial={false}>
                        {isOpen && (
                          <motion.div
                            id={`faq9-answer-${index}`}
                            initial={{ height: 0, opacity: 0 }}
                            animate={{ height: "auto", opacity: 1 }}
                            exit={{ height: 0, opacity: 0 }}
                            transition={{
                              height: {
                                duration: 0.4,
                                ease: [0.22, 1, 0.36, 1],
                              },
                              opacity: {
                                duration: 0.3,
                                ease: [0.22, 1, 0.36, 1],
                              },
                            }}
                            className="overflow-hidden"
                          >
                            <p className="px-5 pb-5 text-sm leading-relaxed text-neutral-600 dark:text-neutral-400 sm:px-6 sm:pb-6">
                              {faq.answer}
                            </p>
                          </motion.div>
                        )}
                      </AnimatePresence>
                    </motion.div>
                  );
                })}
              </div>
            ))}
          </motion.div>
        </div>
      </div>
    </section>
  );
}
