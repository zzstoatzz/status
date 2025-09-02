# Nate's Status Tracker

A personal status tracker built on AT Protocol, where I can post my current status (like Slack status) decoupled from any specific platform.

Live at: [status.zzstoatzz.io](https://status.zzstoatzz.io) (coming soon)

## About

This is my personal status URL - think of it like a service health page, but for a person. I can update my status with an emoji and it's stored permanently in my AT Protocol repository.

## Credits

This app is based on [Bailey Townsend's Rusty Statusphere](https://github.com/fatfingers23/rusty_statusphere_example_app), which is an excellent Rust implementation of the AT Protocol quick start guide. Bailey did all the heavy lifting with the ATProto integration and the overall architecture. I've adapted it for my personal use case.

Major thanks to:
- Bailey Townsend ([@baileytownsend.dev](https://bsky.app/profile/baileytownsend.dev)) for the Rusty Statusphere boilerplate
- The atrium-rs maintainers for the Rust AT Protocol libraries
- The rocketman maintainers for the Jetstream consumer

## Development

```bash
cp .env.template .env
cargo run
# Navigate to http://127.0.0.1:8080
```

## Tech Stack

- Rust with Actix Web
- AT Protocol (via atrium-rs)
- SQLite for local storage
- Jetstream for firehose consumption