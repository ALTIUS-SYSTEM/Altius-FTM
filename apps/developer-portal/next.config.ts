import path from "node:path";
import type { NextConfig } from "next";

const config: NextConfig = {
  outputFileTracingRoot: path.join(import.meta.dirname, "../.."),
          poweredByHeader: false,
  reactStrictMode: true,
  transpilePackages: ["@altius/design-tokens", "next-mdx-remote", "@scalar/api-reference-react"],
};

export default config;
