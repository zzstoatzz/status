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
RUN npm prune --omit=dev
ENV NODE_ENV=production
EXPOSE 3000
CMD ["node", "--max-old-space-size=768", "--experimental-strip-types", "node_modules/@hatk/hatk/dist/main.js", "hatk.config.ts"]
