"use client";

import { useCallback, useRef, useState } from "react";
import { motion, useReducedMotion } from "motion/react";
import {
  Check,
  CircleDashed,
  Clock4,
  Layers2,
  MoveHorizontal,
} from "lucide-react";

const EASE: [number, number, number, number] = [0.22, 1, 0.36, 1];
const MIN = 0;
const MAX = 100;

const rows = [
  {
    title: "Morning dispatch",
    before: "Calls, spreadsheets, guesswork",
    after: "Routes assigned in one pass",
  },
  {
    title: "Arrival proof",
    before: "Photos buried in chat threads",
    after: "Geofenced check-in on record",
  },
  {
    title: "Driver exceptions",
    before: "Found out at day's end",
    after: "Deviations flagged live",
  },
  {
    title: "Daily close-out",
    before: "Rebuilt from memory",
    after: "Generated from the day's events",
  },
];

const stats = [
  { icon: Clock4, value: "3.2 hrs", label: "returned per dispatcher per day" },
  { icon: Layers2, value: "6 → 1", label: "tools folded into one workspace" },
];

function Pane({ variant }: { variant: "before" | "after" }) {
  const isAfter = variant === "after";
  return (
    <div
      className={`absolute inset-0 flex flex-col p-5 sm:p-6 ${
        isAfter
          ? "bg-[#00677e] dark:bg-white"
          : "bg-neutral-100 dark:bg-neutral-950"
      }`}
    >
      <div className="flex items-center justify-between">
        <span
          className={`text-sm font-semibold ${
            isAfter
              ? "text-white dark:text-[#00677e]"
              : "text-neutral-500 dark:text-neutral-400"
          }`}
        >
          {isAfter ? "After Altius" : "Before Altius"}
        </span>
        <span
          className={`rounded-full px-2.5 py-1 text-[11px] font-medium ${
            isAfter
              ? "border border-transparent bg-white/10 text-white dark:bg-[#00677e]/10 dark:text-[#00677e]"
              : "border border-neutral-300 text-neutral-500 dark:border-neutral-700 dark:text-neutral-400"
          }`}
        >
          {isAfter ? "Live system" : "6 tools"}
        </span>
      </div>
      <div className="mt-5 flex flex-1 flex-col gap-3">
        {rows.map((row) => (
          <div
            key={row.title}
            className={`flex flex-1 items-center gap-3.5 rounded-2xl px-4 ${
              isAfter
                ? "bg-white/[0.08] dark:bg-[#00677e]/[0.05]"
                : "border border-dashed border-neutral-300 bg-white/60 dark:border-neutral-700/80 dark:bg-white/[0.03]"
            }`}
          >
            <span
              className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full ${
                isAfter
                  ? "bg-white text-[#00677e] dark:bg-[#00677e] dark:text-white"
                  : "border border-neutral-300 text-neutral-500 dark:border-neutral-700 dark:text-neutral-400"
              }`}
            >
              {isAfter ? (
                <Check className="h-4 w-4" strokeWidth={2.5} />
              ) : (
                <CircleDashed className="h-4 w-4" />
              )}
            </span>
            <span className="min-w-0">
              <span
                className={`block truncate text-sm ${
                  isAfter
                    ? "font-semibold text-white dark:text-neutral-950"
                    : "font-medium text-neutral-600 dark:text-neutral-300"
                }`}
              >
                {row.title}
              </span>
              <span
                className={`mt-0.5 block truncate text-xs ${
                  isAfter
                    ? "text-white/70 dark:text-[#00677e]/70"
                    : "text-neutral-500 dark:text-neutral-400"
                }`}
              >
                {isAfter ? row.after : row.before}
              </span>
            </span>
          </div>
        ))}
      </div>
    </div>
  );
}

export default function Comparison7() {
  const containerRef = useRef<HTMLDivElement>(null);
  const handleRef = useRef<HTMLButtonElement>(null);
  const draggingRef = useRef(false);
  const [position, setPosition] = useState(56);
  const [dragging, setDragging] = useState(false);
  const reduceMotion = useReducedMotion();

  const updateFromClientX = useCallback((clientX: number) => {
    const el = containerRef.current;
    if (!el) return;
    const rect = el.getBoundingClientRect();
    if (rect.width === 0) return;
    const pct = ((clientX - rect.left) / rect.width) * 100;
    setPosition(Math.min(MAX, Math.max(MIN, pct)));
  }, []);

  const slideTransition =
    dragging || reduceMotion ? { duration: 0 } : { duration: 0.5, ease: EASE };

  return (
    <section className="w-full bg-white px-4 py-16 sm:px-6 sm:py-20 lg:px-8 lg:py-24 dark:bg-neutral-950">
      <div className="mx-auto w-full max-w-[1400px]">
        <div className="grid gap-12 lg:grid-cols-[0.9fr_1.1fr] lg:items-center lg:gap-16">
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-80px" }}
            transition={{ duration: 0.6, ease: EASE }}
          >
            <h2 className="text-balance text-2xl font-semibold tracking-tight text-neutral-950 sm:text-3xl md:text-4xl lg:text-5xl dark:text-white">
              Drag the divider. Watch dispatch clean itself up.
            </h2>
            <p className="mt-5 max-w-xl text-pretty text-base leading-relaxed text-neutral-600 sm:text-lg dark:text-neutral-400">
              Altius takes a field operation spread across six tools and folds
              it into one shared command surface: same work, one record.
            </p>
            <div className="mt-8 grid grid-cols-1 gap-4 sm:grid-cols-2">
              {stats.map((stat) => {
                const Icon = stat.icon;
                return (
                  <div
                    key={stat.value}
                    className="rounded-2xl border border-neutral-200 bg-neutral-50 p-5 dark:border-white/10 dark:bg-neutral-900"
                  >
                    <Icon className="h-5 w-5 text-[#00677e] dark:text-cyan-400" />
                    <p className="mt-4 text-2xl font-semibold tracking-tight text-neutral-950 dark:text-white">
                      {stat.value}
                    </p>
                    <p className="mt-1 text-sm leading-relaxed text-neutral-500 dark:text-neutral-400">
                      {stat.label}
                    </p>
                  </div>
                );
              })}
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, y: 20 }}
            whileInView={{ opacity: 1, y: 0 }}
            viewport={{ once: true, margin: "-80px" }}
            transition={{ duration: 0.65, ease: EASE }}
          >
            <div className="rounded-3xl border border-neutral-200 bg-white p-2 shadow-xl shadow-neutral-950/5 sm:p-3 dark:border-white/10 dark:bg-neutral-900 dark:shadow-black/40">
              <div
                ref={containerRef}
                onPointerDown={(event) => {
                  const fromHandle =
                    handleRef.current?.contains(event.target as Node) ?? false;
                  if (event.pointerType === "touch" && !fromHandle) return;
                  draggingRef.current = true;
                  setDragging(true);
                  event.currentTarget.setPointerCapture(event.pointerId);
                  updateFromClientX(event.clientX);
                }}
                onPointerMove={(event) => {
                  if (draggingRef.current) updateFromClientX(event.clientX);
                }}
                onPointerUp={() => {
                  draggingRef.current = false;
                  setDragging(false);
                }}
                onPointerCancel={() => {
                  draggingRef.current = false;
                  setDragging(false);
                }}
                className="relative h-[480px] cursor-ew-resize select-none rounded-2xl sm:h-[540px]"
              >
                <div className="absolute inset-0 overflow-hidden rounded-2xl">
                  <Pane variant="before" />
                  <motion.div
                    initial={{ clipPath: "inset(0 100% 0 0)" }}
                    whileInView={{
                      clipPath: `inset(0 ${100 - position}% 0 0)`,
                    }}
                    viewport={{ once: true, margin: "-80px" }}
                    transition={slideTransition}
                    className="absolute inset-0 z-10"
                  >
                    <Pane variant="after" />
                  </motion.div>
                </div>
                <motion.div
                  initial={{ left: "0%", opacity: 0 }}
                  whileInView={{ left: `${position}%`, opacity: 1 }}
                  viewport={{ once: true, margin: "-80px" }}
                  transition={slideTransition}
                  className="absolute inset-y-0 z-20 w-0.5 -translate-x-1/2 bg-white dark:bg-[#00677e]"
                />
                <motion.div
                  initial={{ left: "0%", opacity: 0 }}
                  whileInView={{ left: `${position}%`, opacity: 1 }}
                  viewport={{ once: true, margin: "-80px" }}
                  transition={slideTransition}
                  className="absolute top-1/2 z-30"
                >
                  <button
                    ref={handleRef}
                    type="button"
                    role="slider"
                    aria-label="Reveal the after state"
                    aria-orientation="horizontal"
                    aria-valuemin={MIN}
                    aria-valuemax={MAX}
                    aria-valuenow={Math.round(position)}
                    aria-valuetext={`${Math.round(position)}% revealed`}
                    onKeyDown={(event) => {
                      if (
                        event.key === "ArrowLeft" ||
                        event.key === "ArrowDown"
                      ) {
                        event.preventDefault();
                        setPosition((value) => Math.max(MIN, value - 6));
                      } else if (
                        event.key === "ArrowRight" ||
                        event.key === "ArrowUp"
                      ) {
                        event.preventDefault();
                        setPosition((value) => Math.min(MAX, value + 6));
                      } else if (event.key === "Home") {
                        event.preventDefault();
                        setPosition(MIN);
                      } else if (event.key === "End") {
                        event.preventDefault();
                        setPosition(MAX);
                      }
                    }}
                    className="flex h-11 w-11 -translate-x-1/2 -translate-y-1/2 cursor-ew-resize touch-none items-center justify-center rounded-full border border-neutral-200 bg-white text-neutral-950 shadow-lg shadow-neutral-950/15 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#00677e] focus-visible:ring-offset-2 focus-visible:ring-offset-white dark:border-white/15 dark:bg-[#00677e] dark:text-white dark:shadow-black/40 dark:focus-visible:ring-white dark:focus-visible:ring-offset-neutral-900"
                  >
                    <MoveHorizontal className="h-5 w-5" />
                  </button>
                </motion.div>
              </div>
            </div>
            <p className="mt-4 flex items-center justify-center gap-2 text-xs font-medium text-neutral-500 dark:text-neutral-400">
              <MoveHorizontal className="h-3.5 w-3.5" />
              Drag the divider, or focus it and use the arrow keys
            </p>
          </motion.div>
        </div>
      </div>
    </section>
  );
}
