"use client";

import { useRef, useState, useEffect } from "react";
import { motion, useInView } from "motion/react";

const creatingImages = [
  {
    url: "https://images.unsplash.com/photo-1587293852726-70cdb56c2866?w=400&h=200&fit=crop",
    aspectRatio: 2,
  },
  {
    url: "https://images.unsplash.com/photo-1601584115197-04ecc0da31d7?w=200&h=200&fit=crop",
    aspectRatio: 1,
  },
  {
    url: "https://images.unsplash.com/photo-1519003722824-194d4455a60c?w=350&h=200&fit=crop",
    aspectRatio: 1.75,
  },
  {
    url: "https://images.unsplash.com/photo-1578575437130-527eed3abbec?w=200&h=200&fit=crop",
    aspectRatio: 1,
  },
];

const buildingImages = [
  {
    url: "https://images.unsplash.com/photo-1553413077-190dd305871c?w=200&h=200&fit=crop",
    aspectRatio: 1,
  },
  {
    url: "https://images.unsplash.com/photo-1494412574643-ff11b0a5c1c3?w=400&h=200&fit=crop",
    aspectRatio: 2,
  },
  {
    url: "https://images.unsplash.com/photo-1566576721346-d4a3b4eaeb55?w=200&h=200&fit=crop",
    aspectRatio: 1,
  },
  {
    url: "https://images.unsplash.com/photo-1524522173746-f628baad3644?w=350&h=200&fit=crop",
    aspectRatio: 1.75,
  },
];

interface ImageData {
  url: string;
  aspectRatio: number;
}

interface MediaBetweenTextRowProps {
  leftText: string;
  rightText: string;
  images: ImageData[];
  alt: string;
  isInView: boolean;
  delay?: number;
}

function MediaBetweenTextRow({
  leftText,
  rightText,
  images,
  alt,
  isInView,
  delay = 0,
}: MediaBetweenTextRowProps) {
  const [isHovered, setIsHovered] = useState(false);
  const [currentImageIndex, setCurrentImageIndex] = useState(0);

  const shouldAnimate = isInView || isHovered;

  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentImageIndex((prev) => (prev + 1) % images.length);
    }, 5000);
    return () => clearInterval(interval);
  }, [images.length]);

  const currentImage = images[currentImageIndex];
  const baseHeight = 100;
  const targetWidth = shouldAnimate ? baseHeight * currentImage.aspectRatio : 0;

  return (
    <>
      <motion.div
        className="sm:hidden text-2xl font-medium text-neutral-900 dark:text-white leading-tight uppercase tracking-tight text-left w-full"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.5, delay }}
      >
        {leftText} {rightText}
      </motion.div>

      <div
        className="hidden sm:flex items-center justify-center gap-x-4 cursor-pointer"
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
      >
        <motion.span
          layout
          className="text-5xl md:text-6xl lg:text-7xl xl:text-8xl font-medium text-neutral-900 dark:text-white leading-none uppercase tracking-tight"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay }}
        >
          {leftText}
        </motion.span>
        <motion.div
          layout
          className="h-[60px] md:h-[70px] lg:h-20 xl:h-[100px] overflow-hidden rounded-md"
          initial={{ width: 0, opacity: 0 }}
          animate={{
            width: targetWidth,
            opacity: shouldAnimate ? 1 : 0,
          }}
          transition={{
            width: { duration: 0.5, type: "spring", bounce: 0 },
            opacity: { duration: 0.3 },
          }}
        >
          <img
            src={currentImage.url}
            alt={alt}
            className="h-full w-full object-cover"
          />
        </motion.div>
        <motion.span
          layout
          className="text-5xl md:text-6xl lg:text-7xl xl:text-8xl font-medium text-neutral-900 dark:text-white leading-none uppercase tracking-tight"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: delay + 0.1 }}
        >
          {rightText}
        </motion.span>
      </div>
    </>
  );
}

export function Hero8() {
  const sectionRef = useRef<HTMLDivElement>(null);
  const isInView = useInView(sectionRef, { once: true, amount: 0.1 });
  const [hasAnimated, setHasAnimated] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setHasAnimated(true);
    }, 100);
    return () => clearTimeout(timer);
  }, []);

  const shouldShowMedia = isInView || hasAnimated;

  return (
    <section className="w-full min-h-screen flex items-start lg:items-center py-12 sm:py-16 md:py-20 lg:py-24 px-4 sm:px-6 lg:px-8 bg-white dark:bg-neutral-950">
      <div ref={sectionRef} className="max-w-[1400px] mx-auto w-full">
        <div className="flex flex-col items-start sm:items-center space-y-8 sm:space-y-12 lg:space-y-16">
          <div className="flex flex-col items-start sm:items-center w-full space-y-2 sm:space-y-4">
            <MediaBetweenTextRow
              leftText="Dispatch"
              rightText="Smarter"
              images={creatingImages}
              alt="Coordinated field dispatch"
              isInView={shouldShowMedia}
              delay={0}
            />
            <MediaBetweenTextRow
              leftText="Deliver"
              rightText="On Time"
              images={buildingImages}
              alt="On-time field delivery"
              isInView={shouldShowMedia}
              delay={0.15}
            />
          </div>

          <motion.p
            className="text-left sm:text-center text-base sm:text-lg text-neutral-600 dark:text-neutral-400 leading-relaxed w-full sm:max-w-2xl"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, delay: 0.4 }}
          >
            Altius connects dispatchers, supervisors, and drivers in one
            workspace. Plan routes, confirm arrivals, and close every task with
            an auditable daily report — online or off.
          </motion.p>

          <motion.div
            className="w-full overflow-hidden rounded-lg sm:rounded-xl lg:rounded-2xl"
            initial={{ opacity: 0, y: 30 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6, delay: 0.5 }}
          >
            <div className="w-full aspect-[16/9] sm:aspect-[21/9] bg-cyan-50 dark:bg-neutral-800">
              <img
                src="https://images.unsplash.com/photo-1601584115197-04ecc0da31d7?w=1400&h=600&fit=crop"
                alt="Field logistics operation"
                className="w-full h-full object-cover"
              />
            </div>
          </motion.div>
        </div>
      </div>
    </section>
  );
}

export default Hero8;
