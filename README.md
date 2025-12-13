# quickslice-status

a status app for bluesky, built with [quickslice](https://github.com/bigmoves/quickslice).

**live:** https://quickslice-status.pages.dev

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

redirect uri: `https://quickslice-status.pages.dev/callback`

## lexicon

uses `io.zzstoatzz.status` lexicon for user statuses.

```json
{
  "lexicon": 1,
  "id": "io.zzstoatzz.status",
  "defs": {
    "main": {
      "type": "record",
      "key": "self",
      "record": {
        "type": "object",
        "required": ["status", "createdAt"],
        "properties": {
          "status": { "type": "string", "maxLength": 128 },
          "createdAt": { "type": "string", "format": "datetime" }
        }
      }
    }
  }
}
```

## local development

serve the frontend locally:
```bash
cd site
python -m http.server 8000
```

for oauth to work locally, you'd need to register a separate oauth client with `http://localhost:8000/callback` as the redirect uri and update `CONFIG.clientId` in `app.js`.
