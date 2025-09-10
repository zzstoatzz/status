# status

a personal status tracker built on at protocol, where i can post my current status (like slack status) decoupled from any specific platform.

live at: [status.zzstoatzz.io](https://status.zzstoatzz.io)

## about

this is my personal status url - think of it like a service health page, but for a person. i can update my status with an emoji and optional text, and it's stored permanently in my at protocol repository.

## credits

this app is based on [bailey townsend's rusty statusphere](https://github.com/fatfingers23/rusty_statusphere_example_app), which is an excellent rust implementation of the at protocol quick start guide. bailey did all the heavy lifting with the atproto integration and the overall architecture. i've adapted it for my personal use case.

major thanks to:
- [bailey townsend (@baileytownsend.dev)](https://bsky.app/profile/baileytownsend.dev) for the rusty statusphere boilerplate
- the atrium-rs maintainers for the rust at protocol libraries
- the rocketman maintainers for the jetstream consumer

## development

```bash
cp .env.template .env
cargo run
# navigate to http://127.0.0.1:8080
```

### custom emojis (no redeploys)

Emojis are now served from a runtime directory configured by `EMOJI_DIR` (defaults to `static/emojis` locally; set to `/data/emojis` on Fly.io). On startup, if the runtime emoji directory is empty, it will be seeded from the bundled `static/emojis`.

- Local dev: add image files to `static/emojis/` (or set `EMOJI_DIR` in `.env`).
- Production (Fly.io): upload files directly into the mounted volume at `/data/emojis` — no rebuild or redeploy needed.

Examples with Fly CLI:

```bash
# Open an SSH console to the machine
fly ssh console -a zzstoatzz-status

# Inside the VM, copy or fetch files into /data/emojis
mkdir -p /data/emojis
curl -L -o /data/emojis/my_new_emoji.png https://example.com/my_new_emoji.png
```

Or from your machine using SFTP:

```bash
fly ssh sftp -a zzstoatzz-status
sftp> put ./static/emojis/my_new_emoji.png /data/emojis/
```

The app serves them at `/emojis/<filename>` and lists them via `/api/custom-emojis`.

### admin upload endpoint

When logged in as the admin DID, you can upload PNG or GIF emojis without SSH via a simple endpoint:

- Endpoint: `POST /admin/upload-emoji`
- Auth: session-based; only the admin DID is allowed
- Form fields (multipart/form-data):
  - `file`: the image file (PNG or GIF), max 5MB
  - `name` (optional): base filename (letters, numbers, `-`, `_`) without extension

Example with curl:

```bash
curl -i -X POST \
  -F "file=@./static/emojis/sample.png" \
  -F "name=my_sample" \
  http://localhost:8080/admin/upload-emoji
```

Response will include the public URL (e.g., `/emojis/my_sample.png`).

### available commands

we use [just](https://github.com/casey/just) for common tasks:

```bash
just watch    # run with hot-reloading
just deploy   # deploy to fly.io
just lint     # run clippy
just fmt      # format code
just clean    # clean build artifacts
```

## tech stack

- [rust](https://www.rust-lang.org/) with [actix web](https://actix.rs/)
- [at protocol](https://atproto.com/) (via [atrium-rs](https://github.com/sugyan/atrium))
- [sqlite](https://www.sqlite.org/) for local storage
- [jetstream](https://github.com/bluesky-social/jetstream) for firehose consumption
- [fly.io](https://fly.io/) for hosting
