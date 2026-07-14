# status

slack status without the slack. built on [HATK](https://github.com/bigmoves/hatk)
and the AT Protocol.

**live:** https://status.zzstoatzz.io

**source:** https://tangled.org/zzstoatzz.io/status

## architecture

- **app**: SvelteKit 5 with server-side rendering and TanStack Query
- **backend**: HATK handles OAuth, XRPC, relay ingestion, and feed hydration
- **database**: SQLite on a persistent Fly volume
- **hosting**: one always-on Fly.io machine in `ewr`

## deployment

The app and API ship together on Fly.io:

```bash
fly deploy
```

required secrets:
```bash
fly secrets set SECRET_KEY_BASE="$(openssl rand -base64 64 | tr -d '\n')"
fly secrets set OAUTH_SIGNING_KEY="$(goat key generate -t p256 | tail -1)"
```

## lexicons

### io.zzstoatzz.status.record

user status records with emoji, optional text, and optional expiration.

```json
{
  "emoji": "🔥",
  "text": "shipping code",
  "createdAt": "2025-12-13T12:00:00Z"
}
```

### io.zzstoatzz.status.preferences

user preferences for display settings.

```json
{
  "accentColor": "#4a9eff",
  "theme": "dark"
}
```

## local development

Run the HATK and SvelteKit development server:

```bash
npm install
npm run dev
```
