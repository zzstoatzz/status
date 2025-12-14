# migrating to quickslice: a status app rewrite

## what we built

a bluesky status app that lets users set emoji statuses (like slack status) stored in their AT protocol repository. the app has two parts:

- **backend**: [quickslice](https://github.com/bigmoves/quickslice) on fly.io - handles OAuth, GraphQL API, and jetstream ingestion
- **frontend**: vanilla JS SPA on cloudflare pages

live at https://status.zzstoatzz.io

## why quickslice

the original implementation was a custom rust backend using atrium-rs. it worked, but maintaining OAuth, jetstream ingestion, and all the AT protocol plumbing was a lot. quickslice handles all of that out of the box:

- OAuth 2.0 with PKCE + DPoP (the hard part of AT protocol)
- GraphQL API auto-generated from your lexicons
- jetstream consumer for real-time firehose data
- admin UI for managing OAuth clients

## the migration

### 1. lexicon design

quickslice ingests data based on lexicons you define. we have two:

**io.zzstoatzz.status.record** - the actual status
```json
{
  "emoji": "🔥",
  "text": "shipping code",
  "createdAt": "2025-12-13T12:00:00Z"
}
```

**io.zzstoatzz.status.preferences** - user display preferences
```json
{
  "accentColor": "#4a9eff",
  "theme": "dark"
}
```

### 2. frontend architecture

since quickslice serves its own admin UI at the root path, we couldn't bundle our frontend into the same container. this led to a clean separation:

- quickslice backend on fly.io (`zzstoatzz-quickslice-status.fly.dev`)
- static frontend on cloudflare pages (`status.zzstoatzz.io`)

the frontend uses the `quickslice-client-js` library for OAuth:
```html
<script src="https://cdn.jsdelivr.net/gh/bigmoves/quickslice@v0.17.3/quickslice-client-js/dist/quickslice-client.min.js"></script>
```

### 3. OAuth flow

quickslice handles the OAuth server side. the frontend just needs to:

1. create a client with `QuicksliceClient.create()`
2. call `client.signIn()` to start the flow
3. handle the callback (quickslice redirects back with auth tokens)
4. use `client.agent` for authenticated AT protocol operations

the redirect URI is just the root of your site (e.g., `https://status.zzstoatzz.io/`).

## problems we hit

### the `sub` claim fix

the biggest issue: after OAuth login, the app would redirect loop infinitely. the AT protocol SDK needs a `sub` claim in the OAuth token response to identify the user, but quickslice v0.17.2 didn't include it.

the fix was in v0.17.3 (commit `0b2d54a`), but `ghcr.io/bigmoves/quickslice:latest` still pointed to v0.17.2. we had to build from source:

```dockerfile
# Clone quickslice at the v0.17.3 tag (includes sub claim fix)
RUN git clone --depth 1 --branch v0.17.3 https://github.com/bigmoves/quickslice.git /build
```

### secrets configuration

quickslice needs two secrets for OAuth to work:

```bash
fly secrets set SECRET_KEY_BASE="$(openssl rand -base64 64 | tr -d '\n')"
fly secrets set OAUTH_SIGNING_KEY="$(goat key generate -t p256 | tail -1)"
```

the `OAUTH_SIGNING_KEY` must be just the multibase key (starts with `z`), not the full output from goat.

### EXTERNAL_BASE_URL

without this, quickslice uses `0.0.0.0:8080` in its OAuth client metadata, which breaks the flow. set it to your public URL:

```toml
[env]
  EXTERNAL_BASE_URL = 'https://zzstoatzz-quickslice-status.fly.dev'
```

### PDS caching

when debugging OAuth issues, be aware that your PDS caches OAuth client metadata. if you fix something on the server, the PDS might still have the old metadata cached. this caused some confusion during debugging.

## deployment architecture

```
┌─────────────────────────────────────────────────────────┐
│                    cloudflare pages                      │
│                  status.zzstoatzz.io                     │
│                                                          │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────┐     │
│   │  index.html │  │   app.js    │  │  styles.css │     │
│   └─────────────┘  └─────────────┘  └─────────────┘     │
└─────────────────────────────────────────────────────────┘
                           │
                           │ GraphQL + OAuth
                           ▼
┌─────────────────────────────────────────────────────────┐
│                       fly.io                             │
│            zzstoatzz-quickslice-status.fly.dev           │
│                                                          │
│   ┌─────────────────────────────────────────────────┐   │
│   │                  quickslice                      │   │
│   │  • OAuth server (PKCE + DPoP)                   │   │
│   │  • GraphQL API (auto-generated from lexicons)   │   │
│   │  • Jetstream consumer                           │   │
│   │  • SQLite database                              │   │
│   └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                           │
                           │ Jetstream
                           ▼
┌─────────────────────────────────────────────────────────┐
│                    AT Protocol                           │
│           (bluesky PDS, jetstream firehose)             │
└─────────────────────────────────────────────────────────┘
```

## key takeaways

1. **quickslice eliminates the hard parts** - OAuth and jetstream are notoriously tricky. quickslice handles them so you can focus on your app logic.

2. **separate frontend and backend** - quickslice serves its own admin UI, so host your frontend elsewhere. cloudflare pages is free and fast.

3. **pin your dependencies** - we got bit by `:latest` not being latest. pin to specific versions/tags.

4. **check the image version** - `fly image show` tells you exactly what's deployed. don't assume.

5. **GraphQL is your API** - quickslice auto-generates a GraphQL API from your lexicons. no need to write endpoints.

6. **the sub claim matters** - AT protocol OAuth needs the `sub` claim in token responses. this was the root cause of our redirect loop.

## resources

- [quickslice](https://github.com/bigmoves/quickslice) - the framework
- [AT protocol OAuth](https://atproto.com/specs/oauth) - the spec
- [quickslice-client-js](https://github.com/bigmoves/quickslice/tree/main/quickslice-client-js) - frontend OAuth helper
