watch:
    watchexec -w src -w templates -r cargo run

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