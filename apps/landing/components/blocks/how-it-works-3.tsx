"use client";

import { useState } from "react";
import { motion, AnimatePresence } from "motion/react";
import { ClipboardCheck, Route, MapPinCheck, FileCheck2 } from "lucide-react";

const items = [
  {
    id: 1,
    title: "Plan the day",
    description:
      "Import visits, assign drivers, and let the optimizer order stops. Every task lands on a phone in seconds.",
    images: [
      "https://images.unsplash.com/photo-1494412574643-ff11b0a5c1c3?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1587293852726-70cdb56c2866?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1578575437130-527eed3abbec?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1553413077-190dd305871c?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1519003722824-194d4455a60c?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1601584115197-04ecc0da31d7?q=80&w=400&auto=format&fit=crop",
    ],
  },
  {
    id: 2,
    title: "Execute in the field",
    description:
      "Drivers tap arrive, work, done — three actions per stop. Check-ins are geofenced and events queue offline.",
    images: [
      "https://images.unsplash.com/photo-1566576721346-d4a3b4eaeb55?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1524522173746-f628baad3644?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1580674285054-bed31e145f59?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1591768793355-74d04bb6608f?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1616401784845-180882ba9c8a?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1454165804606-c3d57bc86b40?q=80&w=400&auto=format&fit=crop",
    ],
  },
  {
    id: 3,
    title: "Review and close",
    description:
      "ETA meets ATA, anomalies get a human review, and the daily driver report locks with costs attached.",
    images: [
      "https://images.unsplash.com/photo-1551288049-bebda4e38f71?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1460925895917-afdab827c52f?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1504868584819-f8e8b4b6d7e3?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1554224155-6726b3ff858f?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1450101499163-c8848c66ca85?q=80&w=400&auto=format&fit=crop",
      "https://images.unsplash.com/photo-1543286386-713bdd548da4?q=80&w=400&auto=format&fit=crop",
    ],
  },
];

export function HowItWorks3() {
  const [activeItem, setActiveItem] = useState(1);
  const currentItem = items.find((item) => item.id === activeItem)!;

  return (
    <section
      id="how-it-works"
      className="w-full py-12 px-4 sm:px-6 lg:px-8 bg-white dark:bg-neutral-950"
      aria-label="How it works"
    >
      <div className="max-w-[1400px] mx-auto w-full">
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-12 lg:gap-16 items-center">
          <motion.div
            initial={{ opacity: 0, x: -30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5 }}
          >
            <h2 className="text-3xl sm:text-4xl lg:text-5xl font-medium tracking-tight text-neutral-900 dark:text-white mb-6">
              A field day, <span className="italic font-serif">three moves.</span>
            </h2>

            <div className="relative border-l-2 border-dashed border-neutral-200 dark:border-neutral-800">
              {items.map((item) => (
                <div
                  key={item.id}
                  className="relative cursor-pointer group"
                  onClick={() => setActiveItem(item.id)}
                >
                  <motion.div
                    className="absolute left-0 top-0 bottom-0 w-0.5 -ml-px bg-[#00677e] dark:bg-white"
                    initial={false}
                    animate={{
                      opacity: activeItem === item.id ? 1 : 0,
                      scaleY: activeItem === item.id ? 1 : 0,
                    }}
                    transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                    style={{ originY: 0.5 }}
                  />
                  <motion.div
                    className="pl-6"
                    initial={false}
                    animate={{ paddingTop: 12, paddingBottom: 12 }}
                    transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                  >
                    <h3
                      className={`text-base sm:text-lg font-medium transition-colors duration-200 ${
                        activeItem === item.id
                          ? "text-neutral-900 dark:text-white"
                          : "text-neutral-400 dark:text-neutral-600"
                      }`}
                    >
                      {item.title}
                    </h3>
                    <motion.div
                      initial={false}
                      animate={{
                        height: activeItem === item.id ? "auto" : 0,
                        opacity: activeItem === item.id ? 1 : 0,
                        marginTop: activeItem === item.id ? 8 : 0,
                      }}
                      transition={{ duration: 0.3, ease: [0.4, 0, 0.2, 1] }}
                      className="overflow-hidden"
                    >
                      <p className="text-sm sm:text-base text-neutral-500 dark:text-neutral-400 leading-relaxed max-w-md">
                        {item.description}
                      </p>
                    </motion.div>
                  </motion.div>
                </div>
              ))}
            </div>
          </motion.div>

          <motion.div
            initial={{ opacity: 0, x: 30 }}
            whileInView={{ opacity: 1, x: 0 }}
            viewport={{ once: true }}
            transition={{ duration: 0.5, delay: 0.2 }}
            className="relative flex justify-center"
            style={{ perspective: "600px" }}
          >
            <div
              className="relative rounded-2xl p-4 sm:p-5 max-w-sm w-full border border-neutral-200/50 dark:border-neutral-700/50 bg-linear-to-br from-cyan-50/80 via-sky-50/80 to-amber-50/80 dark:from-neutral-800/80 dark:via-neutral-900/80 dark:to-neutral-800/80"
              style={{
                transform: "rotateY(-20deg) rotateX(8deg)",
                transformStyle: "preserve-3d",
              }}
            >
              <div className="absolute inset-0 rounded-2xl bg-white/40 dark:bg-neutral-900/60 backdrop-blur-sm" />

              <AnimatePresence mode="wait">
                <motion.div
                  key={activeItem}
                  initial={{ opacity: 0, scale: 0.95 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.95 }}
                  transition={{ duration: 0.4, ease: "easeOut" }}
                  className="relative z-10 grid grid-cols-3 gap-3 sm:gap-4"
                >
                  {currentItem.images.map((image, idx) => (
                    <motion.div
                      key={idx}
                      initial={{ opacity: 0, y: 20 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{ duration: 0.3, delay: idx * 0.05 }}
                      className="overflow-hidden rounded-xl shadow-lg aspect-square"
                    >
                      <img
                        src={image}
                        alt=""
                        className="w-full h-full object-cover"
                        loading="lazy"
                      />
                    </motion.div>
                  ))}
                </motion.div>
              </AnimatePresence>

              <div className="relative z-10 flex items-center justify-center gap-2 mt-6">
                {[ClipboardCheck, Route, MapPinCheck, FileCheck2].map((Icon, idx) => (
                  <div
                    key={idx}
                    className="w-10 h-10 rounded-full bg-white dark:bg-neutral-800 shadow-md flex items-center justify-center text-[#00677e] dark:text-cyan-300 hover:bg-neutral-50 dark:hover:bg-neutral-700 transition-colors cursor-pointer"
                  >
                    <Icon className="w-4 h-4" />
                  </div>
                ))}
              </div>
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}

export default HowItWorks3;
