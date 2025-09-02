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