# status app: quickslice → hatk migration

_march 2026_

## what changed

rewrote the status app from a **quickslice** backend + **vanilla JS SPA** frontend (split across Fly.io and Cloudflare Pages) into a single **hatk** app (SvelteKit + typed XRPC backend) deployed to Fly.io only.

## architecture: before and after

### before (quickslice)

```
cloudflare pages          fly.io
┌─────────────────┐      ┌──────────────────────┐
│ site/            │      │ quickslice (pre-built)│
│  index.html      │ ──→  │  graphql api          │
│  app.js (1695L)  │      │  sqlite @ /data/      │
│  styles.css      │      │  firehose ingestion   │
│  functions/      │      └──────────────────────┘
│    status/[did]/ │
│      [rkey].js   │  ← cloudflare pages function
└─────────────────┘     for OG tag injection
```

- **backend**: quickslice — "appview in a bottle" by chad. pre-built docker image, graphql API, no code generation
- **frontend**: vanilla JS SPA. one 1695-line `app.js`, one 1152-line `styles.css`, no build step
- **link previews**: cloudflare pages function (`site/functions/status/[did]/[rkey].js`) intercepted social bot user agents, fetched status via graphql, returned HTML with OG tags
- **deploy**: two targets — `fly deploy` for backend, cloudflare pages for frontend
- **data access**: raw graphql queries inline in app.js

### after (hatk)

```
fly.io (single deploy)
┌──────────────────────────────────┐
│ hatk                             │
│  sveltekit frontend (app/)       │
│  typed XRPC backend (server/)    │
│  auto-generated from lexicons    │
│  sqlite @ /data/status.db        │
│  firehose ingestion + backfill   │
│  oauth (AT Protocol native)      │
│  SSR → link previews built-in    │
└──────────────────────────────────┘
```

- **framework**: hatk — full-stack AT Protocol framework. SvelteKit frontend + typed XRPC backend, types auto-generated from lexicons
- **frontend**: svelte 5 components (30 files, 2248 lines total). tanstack svelte query for data fetching
- **link previews**: SSR via `+page.server.ts` — no separate function needed, OG tags rendered server-side
- **deploy**: single target — `fly deploy`
- **data access**: `callXrpc('dev.hatk.getFeed', ...)` with full type safety

## infra changes

| | before | after |
|---|---|---|
| **backend** | `ghcr.io/bigmoves/quickslice:latest` (pre-built) | custom Dockerfile, `node:25-slim`, `vp build` |
| **API** | graphql | XRPC (AT Protocol native) |
| **port** | 8080 | 3000 |
| **frontend hosting** | cloudflare pages | same fly.io app (SvelteKit SSR) |
| **OG previews** | cloudflare pages function | SvelteKit SSR (`+page.server.ts`) |
| **domain** | `zzstoatzz-quickslice-status.fly.dev` (backend) + cloudflare pages (frontend) | `status.zzstoatzz.io` → fly.io (CNAME) |
| **db** | `/data/quickslice.db` | `/data/status.db` (same volume, new file) |
| **data migration** | n/a | none needed — AT Protocol repos are source of truth, hatk backfills from firehose |
| **oauth** | quickslice client SDK | hatk built-in AT Protocol OAuth |
| **dev env** | none documented | `docker-compose.yml` (PLC + PDS + postgres) |

## file structure comparison

### removed
- `site/` — entire vanilla JS SPA (app.js, styles.css, index.html, functions/)
- `notes/quickslice-migration.md` — old migration notes

### added

```
app/                              # sveltekit frontend
  app.css                         # global styles (740L, was 1152L)
  app.html                        # shell with OG fallback tags
  lib/
    components/
      Header.svelte               # nav, theme toggle, login/logout
      CreateStatusForm.svelte     # emoji picker + text + expiration
      EmojiPicker.svelte          # unicode + bufo tabs, semantic search
      StatusCard.svelte           # single status display
      StatusFeed.svelte           # paginated feed with "load more"
      LoginCard.svelte            # handle input with typeahead
      SettingsModal.svelte        # accent color, font, theme
    utils/
      emoji.ts                    # bufo image URLs, emoji search, parseLinks
      time.ts                     # relativeTime, formatExpiration
    queries.ts                    # tanstack query options wrapping callXrpc
    preferences.ts                # accent color, font, theme as CSS vars
    stores.ts                     # loginModalOpen (just one store now)
    auth.ts                       # re-export from hatk
  routes/
    +layout.server.ts             # parse viewer from session cookie
    +layout.ts                    # create QueryClient
    +layout.svelte                # shell: QueryClientProvider + Header
    +page.svelte                  # home: login or current status + form
    +page.ts                      # prefetch actor feed
    feed/+page.svelte             # global feed (all users)
    feed/+page.ts                 # prefetch recent feed
    profile/[did]/+page.svelte    # user profile with history
    profile/[did]/+page.ts        # prefetch actor feed
    status/[did]/[rkey]/
      +page.server.ts             # fetch status for SSR + OG tags
      +page.svelte                # status permalink with OG meta
    oauth/callback/+page.svelte   # OAuth redirect handler

server/                           # hatk backend
  feeds/
    recent.ts                     # global feed — all statuses DESC
    actor.ts                      # per-user feed — single DID
    _hydrate.ts                   # DB rows → statusView objects
  on-login.ts                     # backfill user's repo on first login

hatk.config.ts                    # relay, plc, OAuth, SQLite, backfill
vite.config.ts                    # hatk() + sveltekit() plugins
svelte.config.js                  # adapter-node, files.src: "app"
docker-compose.yml                # local dev: PLC + PDS + postgres
Dockerfile                        # node:25-slim, vp build, hatk start
```

## key differences in practice

### data fetching

before (graphql in vanilla JS):
```js
const res = await fetch(`${CONFIG.server}/graphql`, {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    query: `query { ioZzstoatzzStatusRecord(first: 20, sortBy: [...]) { edges { node { ... } } } }`,
  })
});
```

after (typed XRPC via tanstack query):
```ts
const feed = createQuery(() => recentFeedQuery())
// recentFeedQuery = () => callXrpc("dev.hatk.getFeed", { feed: "recent", limit: 50 })
```

### link previews

before (cloudflare pages function):
```js
// site/functions/status/[did]/[rkey].js
export async function onRequest(context) {
  if (!isSocialBot(userAgent)) return next();
  const status = await fetchStatus(did, rkey); // graphql
  return new Response(generateOgHtml(status, ...));
}
```

after (SvelteKit SSR):
```ts
// +page.server.ts — fetches on server, svelte:head renders OG tags
export const load = async ({ params }) => {
  const res = await callXrpc("dev.hatk.getRecord", { uri });
  return { status: res.record };
};
```
```svelte
<!-- +page.svelte — OG tags in svelte:head, rendered during SSR -->
<svelte:head>
  <meta property="og:image" content={ogImage} />
</svelte:head>
```

### auth

before: quickslice client SDK (`QuicksliceClient.createQuicksliceClient()`), manual OAuth flow
after: hatk built-in OAuth, session cookie parsed by `parseViewer(cookies)`

## bugs fixed during migration

- **`store.subscribe is not a function` (SSR 500)**: tanstack svelte query v5 returns reactive objects, not svelte stores. `$feed.data` → `feed.data`
- **OOM crashes**: backfill parallelism 5 on 1GB VM caused heap exhaustion. reduced to 2 + added `--max-old-space-size=768`
- **header handle sticking**: header showed viewer handle on all pages. now shows contextual titles (home: `@handle`, feed: `global feed`, etc.)
- **link previews missing**: `+page.ts` was overwriting `+page.server.ts` data, dropping the status. removed redundant `+page.ts`

## what we kept

- same Fly.io app (`zzstoatzz-quickslice-status`) and volume (`quickslice_data`)
- same lexicons (`io.zzstoatzz.status.record`, `io.zzstoatzz.status.preferences`)
- bufo emoji support (custom:name → `all-the.bufo.zone/{name}.png`)
- semantic bufo search via `find-bufo.fly.dev`
- handle typeahead via `typeahead.waow.tech`
- dark/light theme, accent colors, font preferences
- same UX: home (your status + form), global feed, profile, permalink
