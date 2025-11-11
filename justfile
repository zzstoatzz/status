watch:
    watchexec -w src -w templates -r cargo run

dev:
    SERVER_PORT=3000 cargo watch -x run -w src -w templates

deploy:
    fly deploy

lint:
    cargo clippy -- -D warnings

fmt:
    cargo fmt

clean:
    cargo clean
    rm -f status.db

test:
    cargo test