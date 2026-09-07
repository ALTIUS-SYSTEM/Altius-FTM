"use client";

import { motion } from "motion/react";
import { useEffect, useRef } from "react";
import { ArrowRight } from "lucide-react";

export default function SocialProof4() {
  const allTestimonials = [
    {
      quote:
        "Dispatch that used to take three spreadsheets now takes one morning pass. Drivers know their stops before they leave the hub.",
      name: "Rachel Thompson",
      title: "Ops Lead, Metro Distribution",
      avatar:
        "https://images.unsplash.com/photo-1600481453173-55f6a844a4ea?q=80&w=750&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "The arrival reports are gold. Customers stopped asking where their delivery is because we can show the check-in time.",
      name: "Marcus Liu",
      title: "Fleet Supervisor, Portside Logistics",
      avatar:
        "https://images.unsplash.com/photo-1629649534931-4884c197af3a?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "Our drivers adopted it in a day. Three buttons — arrive, work, done — and the office sees everything.",
      name: "Sophia Patel",
      title: "Hub Manager, Meridian Fresh",
      avatar:
        "https://images.unsplash.com/photo-1750680475124-fd1ef8c4bbc5?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "Geofence deviation alerts caught a recurring wrong-zone problem in week one. Paid for itself immediately.",
      name: "David Chen",
      title: "Regional Director, Kargoline",
      avatar:
        "https://images.unsplash.com/photo-1611403119860-57c4937ef987?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "The daily LHS report replaced a stack of paper forms. Auditing driver days is finally painless.",
      name: "Emma Rodriguez",
      title: "Compliance Manager, Atlas Freight",
      avatar:
        "https://images.unsplash.com/photo-1618508035424-73ad1a15006c?q=80&w=930&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "Offline mode actually works. Drivers in weak-signal areas finish their routes and events sync cleanly.",
      name: "Tyler Joseph",
      title: "IT Lead, Archipelago Express",
      avatar:
        "https://images.unsplash.com/photo-1562208512-ec508326186b?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "Route comparison between planned and actual let us fix a bad stop order we didn't even know we had.",
      name: "Jennifer Walsh",
      title: "Planner, Sunda Routes",
      avatar:
        "https://images.unsplash.com/photo-1549124041-8b22c6157337?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "GPS anomaly review gave us an objective way to handle disputed visits — no more he-said-she-said.",
      name: "Alex Kim",
      title: "Safety Officer, Kencana Group",
      avatar:
        "https://images.unsplash.com/photo-1705408115324-6bd2cbfa4d93?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "Seven languages out of the box. Our Thai and Vietnamese drivers finally have software that speaks to them.",
      name: "Olivia Bennett",
      title: "HR & Ops, TransJava Cargo",
      avatar:
        "https://images.unsplash.com/photo-1593207129063-c99ebcf14ea4?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "ETA vs actual arrival tracking reshaped how we schedule windows. On-time rate is up 14 points.",
      name: "Nathan Foster",
      title: "COO, Delta Last-Mile",
      avatar:
        "https://images.unsplash.com/photo-1564172556663-2bef9580fc44?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "The role model is clean: admin, supervisor, lead, driver. Permissions finally match how we actually run a hub.",
      name: "Isabella Santos",
      title: "Ops Director, CargoNusa",
      avatar:
        "https://images.unsplash.com/photo-1759906219433-44fe183acf41?q=80&w=774&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
    {
      quote:
        "CSV export for task history makes client reporting trivial. What took a day now takes minutes.",
      name: "Christopher Lee",
      title: "Account Manager, Strait Haul",
      avatar:
        "https://images.unsplash.com/photo-1530466015235-1d47696ea847?q=80&w=1674&auto=format&fit=crop&ixlib=rb-4.1.0&ixid=M3wxMjA3fDB8MHxwaG90by1wYWdlfHx8fGVufDB8fHx8fA%3D%3D",
    },
  ];

  const testimonials = [
    allTestimonials.slice(0, 4),
    allTestimonials.slice(4, 8),
    allTestimonials.slice(8, 12),
  ];

  const marquee1Ref = useRef<HTMLDivElement>(null);
  const marquee2Ref = useRef<HTMLDivElement>(null);
  const marquee3Ref = useRef<HTMLDivElement>(null);
  const marqueeMobileRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const marquee1 = marquee1Ref.current;
    const marquee2 = marquee2Ref.current;
    const marquee3 = marquee3Ref.current;
    const marqueeMobile = marqueeMobileRef.current;

    let offset1 = 0;
    let offset2 = 0;
    let offset3 = 0;
    let offsetMobile = 0;

    const animate = () => {
      if (marqueeMobile) {
        offsetMobile += 0.5;
        const heightMobile = marqueeMobile.scrollHeight / 2;
        if (offsetMobile >= heightMobile) {
          offsetMobile = 0;
        }
        marqueeMobile.style.transform = `translateY(-${offsetMobile}px)`;
      }

      if (marquee1) {
        offset1 += 0.5;
        const height1 = marquee1.scrollHeight / 2;
        if (offset1 >= height1) {
          offset1 = 0;
        }
        marquee1.style.transform = `translateY(-${offset1}px)`;
      }

      if (marquee2) {
        offset2 += 0.6;
        const height2 = marquee2.scrollHeight / 2;
        if (offset2 >= height2) {
          offset2 = 0;
        }
        marquee2.style.transform = `translateY(-${offset2}px)`;
      }

      if (marquee3) {
        offset3 += 0.4;
        const height3 = marquee3.scrollHeight / 2;
        if (offset3 >= height3) {
          offset3 = 0;
        }
        marquee3.style.transform = `translateY(-${offset3}px)`;
      }

      requestAnimationFrame(animate);
    };

    const animationId = requestAnimationFrame(animate);

    return () => cancelAnimationFrame(animationId);
  }, []);

  const renderCard = (
    testimonial: (typeof allTestimonials)[number],
    key: string,
  ) => (
    <div
      key={key}
      className="mb-4 rounded-2xl bg-white/60 p-1.5 shadow-sm backdrop-blur-md dark:bg-neutral-800/40"
    >
      <div className="rounded-[10px] border border-neutral-300/60 bg-white p-6 shadow-sm dark:border-neutral-800/50 dark:bg-neutral-900">
        <p className="mb-4 text-sm leading-relaxed text-neutral-700 dark:text-neutral-300">
          &ldquo;{testimonial.quote}&rdquo;
        </p>
        <div className="flex items-center gap-3">
          <img
            src={testimonial.avatar}
            alt={testimonial.name}
            className="h-10 w-10 rounded-lg border border-neutral-200 object-cover dark:border-neutral-700"
          />
          <div>
            <div className="text-sm font-semibold text-neutral-900 dark:text-neutral-100">
              {testimonial.name}
            </div>
            <div className="text-xs text-neutral-600 dark:text-neutral-400">
              {testimonial.title}
            </div>
          </div>
        </div>
      </div>
    </div>
  );

  return (
    <section className="relative w-full overflow-hidden bg-white py-16 dark:bg-neutral-950 sm:py-24 px-4 sm:px-6 lg:px-8">
      <div className="mx-auto max-w-[1400px]">
        <div className="mb-12 flex flex-col items-start justify-between gap-6 sm:flex-row sm:items-end lg:mb-16">
          <motion.h2
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.4 }}
            className="text-4xl font-medium leading-tight text-neutral-900 dark:text-neutral-50 sm:text-5xl lg:text-6xl"
          >
            Trusted by teams <br /> that move things
          </motion.h2>

          <motion.a
            initial={{ opacity: 0 }}
            whileInView={{ opacity: 1 }}
            viewport={{ once: true }}
            transition={{ duration: 0.4, delay: 0.1 }}
            href="#"
            className="group whitespace-nowrap inline-flex items-center gap-2 rounded-full bg-[#00677e] px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-[#005565] dark:bg-neutral-50 dark:text-neutral-900 dark:hover:bg-neutral-200"
          >
            All stories
            <ArrowRight className="h-4 w-4 transition-transform group-hover:translate-x-1" />
          </motion.a>
        </div>

        <div className="relative sm:hidden">
          <div className="relative h-[600px] overflow-hidden">
            <div ref={marqueeMobileRef}>
              {[...allTestimonials, ...allTestimonials].map((t, i) =>
                renderCard(t, `mobile-${i}`),
              )}
            </div>
            <div className="pointer-events-none absolute inset-x-0 top-0 h-32 bg-linear-to-b from-white via-white/90 to-transparent dark:from-neutral-950 dark:via-neutral-950/90" />
            <div className="pointer-events-none absolute inset-x-0 bottom-0 h-32 bg-linear-to-t from-white via-white/90 to-transparent dark:from-neutral-950 dark:via-neutral-950/90" />
          </div>
        </div>

        <div className="relative hidden gap-4 sm:grid sm:grid-cols-2 lg:grid-cols-3">
          {[marquee1Ref, marquee2Ref, marquee3Ref].map((ref, col) => (
            <div key={col} className="relative h-[600px] overflow-hidden">
              <div ref={ref}>
                {[...testimonials[col], ...testimonials[col]].map((t, i) =>
                  renderCard(t, `col${col + 1}-${i}`),
                )}
              </div>
              <div className="pointer-events-none absolute inset-x-0 top-0 h-24 bg-linear-to-b from-white to-transparent dark:from-neutral-950" />
              <div className="pointer-events-none absolute inset-x-0 bottom-0 h-24 bg-linear-to-t from-white to-transparent dark:from-neutral-950" />
            </div>
          ))}
        </div>
      </div>
    </section>
  );
}
