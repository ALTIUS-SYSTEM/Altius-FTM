"use client";

import { motion } from "motion/react";
import type { ReactNode } from "react";

type ContentFadeProps = {
  children: ReactNode;
  /** Remount key when route content changes. */
  motionKey?: string;
};

/**
 * Light content entrance fade for guide/article bodies.
 */
export function ContentFade({ children, motionKey = "content" }: ContentFadeProps) {
  return (
    <motion.div
      key={motionKey}
      initial={{ opacity: 0, y: 8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.35, ease: [0.22, 1, 0.36, 1] }}
    >
      {children}
    </motion.div>
  );
}
