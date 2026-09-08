# syntax=docker/dockerfile:1
# Built rather than pulled: the stock image has no rate-limit module and the
# Caddyfile's `rate_limit` directives will not parse without it. Keep this file
# and the Caddyfile in step — deleting it breaks `docker compose build caddy`.
FROM caddy:2.9-builder-alpine AS build
RUN xcaddy build --with github.com/mholt/caddy-ratelimit

FROM caddy:2.9-alpine
COPY --from=build /usr/bin/caddy /usr/bin/caddy
EXPOSE 80 443
# The Caddyfile sets `admin off`, so :2019 is unavailable; probe the
# self-answered /healthz route. Not /dev/tcp — busybox `ash` has no such device,
# so that form reports unhealthy no matter what Caddy is doing.
HEALTHCHECK --interval=15s --timeout=5s --retries=3 --start-period=20s \
  CMD wget -qO- --tries=1 http://127.0.0.1:80/healthz >/dev/null || exit 1
