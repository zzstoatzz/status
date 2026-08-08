FROM node:25-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl sqlite3 \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY package.json package-lock.json ./
COPY patches ./patches
RUN npm ci
COPY . .
RUN npx hatk generate types
RUN npx vp build
# No `npm prune --omit=dev`. hatk loads hatk.config.ts through
# @voidzero-dev/vite-plus-core, which is a *production* dep (via hatk's own
# vitest dependency, overridden to vite-plus-test), but its native binding
# @voidzero-dev/vite-plus-linux-x64-gnu is marked dev-only in the lockfile.
# Pruning deletes the binding and keeps the loader, so the app boots into
# "Cannot find module '../rolldown-binding.linux-x64-gnu.node'" and crash-loops.
# This was masked for a long time by docker layer caching: `COPY patches` sits
# above `npm ci`, so the prune only re-runs when patches or package files change.
ENV NODE_ENV=production
EXPOSE 3000
CMD ["node", "--max-old-space-size=768", "--experimental-strip-types", "node_modules/@hatk/hatk/dist/main.js", "hatk.config.ts"]
