This is how to build that produces a statically compiled risp:

RUSTFLAGS="-C target-feature=+crt-static" cargo build --target x86_64-unknown-linux-gnu --release


