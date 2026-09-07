import Navigation2 from "@/components/blocks/navigation-2";
import Hero8 from "@/components/blocks/hero-8";
import About3 from "@/components/blocks/about-3";
import Features6 from "@/components/blocks/features-6";
import Comparison7 from "@/components/blocks/comparison-7";
import SocialProof4 from "@/components/blocks/social-proof-4";
import HowItWorks3 from "@/components/blocks/how-it-works-3";
import Showcase3 from "@/components/blocks/showcase-3";
import Stats5 from "@/components/blocks/stats-5";
import Blog1 from "@/components/blocks/blog-1";
import Faq9 from "@/components/blocks/faq-9";
import Cta6 from "@/components/blocks/cta-6";
import Footer12 from "@/components/blocks/footer-12";

/**
 * Altius landing page
 *
 * Composed with the React Bits Landing Builder.
 *
 * The wrapper below sets `--rb-section-min-h: 0px`, which lets content
 * sections take their natural height instead of each filling the viewport.
 * Remove it and every section reverts to full-screen, which is the correct
 * behaviour when a block is used on its own.
 */
export default function Page() {
  return (
    <main
      className="w-full"
      style={{ "--rb-section-min-h": "0px" } as React.CSSProperties}
    >
      <Navigation2 />
      <Hero8 />
      <About3 />
      <Features6 />
      <Comparison7 />
      <SocialProof4 />
      <HowItWorks3 />
      <Showcase3 />
      <Stats5 />
      <Blog1 />
      <Faq9 />
      <Cta6 />
      <Footer12 />
    </main>
  );
}
