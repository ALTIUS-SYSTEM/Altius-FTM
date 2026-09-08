import path from "node:path";
import type { NextConfig } from "next";

const config: NextConfig = {
  // pnpm workspace: trace from the repo root so the standalone bundle picks up
  // linked workspace packages instead of dangling symlinks.
  outputFileTracingRoot: path.join(import.meta.dirname, "../.."),
  // Self-contained server + pruned node_modules. Without it the image ships
  // the whole workspace node_modules, which for pnpm is a symlink farm that
  // breaks the moment it is COPYed out of the build stage.
  output: "standalone",
  poweredByHeader: false,
  reactStrictMode: true,
  async headers() {
    return [{ source: "/(.*)", headers: [
      { key: "X-Content-Type-Options", value: "nosniff" },
      { key: "Referrer-Policy", value: "strict-origin-when-cross-origin" },
      { key: "X-Frame-Options", value: "DENY" },
      { key: "Permissions-Policy", value: "camera=(), microphone=(), geolocation=()" },
      // Content-Security-Policy is set in src/middleware.ts: it carries a
      // per-request nonce for Next's inline RSC bootstrap scripts, which a
      // static header cannot express.
    ] }];
  }
};
export default config;
