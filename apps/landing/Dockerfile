# syntax=docker/dockerfile:1
# Build context is the repo root (pnpm workspace); see docker-compose.yml.

# ---------- deps: cached on the lockfile alone ----------
FROM node:22.14-bookworm-slim AS deps
ENV PNPM_HOME=/pnpm CI=true
WORKDIR /src
RUN corepack enable
# Manifests only, so this layer survives every source edit.
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json ./
COPY apps/web/package.json apps/web/
COPY apps/landing/package.json apps/landing/
COPY packages/algos/package.json packages/algos/
COPY packages/api-contracts/package.json packages/api-contracts/
COPY packages/design-tokens/package.json packages/design-tokens/
COPY packages/i18n/package.json packages/i18n/
RUN --mount=type=cache,id=pnpm-store,target=/pnpm/store \
    pnpm install --frozen-lockfile

# ---------- build ----------
FROM node:22.14-bookworm-slim AS build
ENV NEXT_TELEMETRY_DISABLED=1
WORKDIR /src
RUN corepack enable
COPY --from=deps /src/node_modules ./node_modules
COPY --from=deps /src/apps ./apps
COPY --from=deps /src/packages ./packages
COPY . .
RUN pnpm --filter @altius/landing build

# ---------- runtime ----------
FROM node:22.14-bookworm-slim AS runtime
ENV NODE_ENV=production NEXT_TELEMETRY_DISABLED=1 PORT=3000 HOSTNAME=0.0.0.0
WORKDIR /app
RUN apt-get update \
 && apt-get install -y --no-install-recommends dumb-init \
 && rm -rf /var/lib/apt/lists/*
# `node` (uid 1000) already exists in the official image; recreating it fails.
USER node
# `output: "standalone"` emits a self-contained server plus a pruned
# node_modules. Copying the workspace node_modules instead would ship a pnpm
# symlink farm that breaks the moment it leaves the build stage.
COPY --from=build --chown=node:node /src/apps/landing/.next/standalone ./
COPY --from=build --chown=node:node /src/apps/landing/.next/static ./apps/landing/.next/static
COPY --from=build --chown=node:node /src/apps/landing/public ./apps/landing/public
EXPOSE 3000
HEALTHCHECK --interval=15s --timeout=5s --retries=3 --start-period=30s \
  CMD node -e "require('http').get('http://127.0.0.1:3000/',r=>process.exit(r.statusCode<400?0:1)).on('error',()=>process.exit(1))"
ENTRYPOINT ["dumb-init", "--"]
CMD ["node", "apps/landing/server.js"]
