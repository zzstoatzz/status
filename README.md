# status

slack status without the slack. built on [quickslice](https://github.com/bigmoves/quickslice).

**live:** https://status.zzstoatzz.io

## architecture

- **backend**: [quickslice](https://github.com/bigmoves/quickslice) on fly.io - handles oauth, graphql api, jetstream ingestion
- **frontend**: static site on cloudflare pages - vanilla js spa

## deployment

### backend (fly.io)

builds quickslice from source at v0.17.3 tag.

```bash
fly deploy
```

required secrets:
```bash
fly secrets set SECRET_KEY_BASE="$(openssl rand -base64 64 | tr -d '\n')"
fly secrets set OAUTH_SIGNING_KEY="$(goat key generate -t p256 | tail -1)"
```

### frontend (cloudflare pages)

```bash
cd site
npx wrangler pages deploy . --project-name=quickslice-status
```

## oauth client registration

register an oauth client in the quickslice admin ui at `https://zzstoatzz-quickslice-status.fly.dev/`

redirect uri: `https://status.zzstoatzz.io/callback`

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

serve the frontend locally:
```bash
cd site
python -m http.server 8000
```

for oauth to work locally, register a separate oauth client with `http://localhost:8000/callback` as the redirect uri and update `CONFIG.clientId` in `app.js`.
