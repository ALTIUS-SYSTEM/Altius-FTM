# syntax=docker/dockerfile:1
FROM node:22-slim AS build
WORKDIR /src
RUN corepack enable
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY apps/web/package.json apps/web/package.json
COPY apps/landing/package.json apps/landing/package.json
COPY packages packages
RUN pnpm install --frozen-lockfile
COPY . .
RUN pnpm --filter @altius/landing build

FROM node:22-slim
WORKDIR /app
ENV NODE_ENV=production
COPY --from=build /src/apps/landing/.next ./.next
COPY --from=build /src/apps/landing/package.json ./package.json
COPY --from=build /src/node_modules ./node_modules
COPY --from=build /src/apps/landing/public ./public
CMD ["npx", "next", "start", "--hostname", "0.0.0.0", "--port", "3000"]
