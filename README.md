# superdeduper

Image deduplicator in Rust.

Use `cargo build --release`, then `./build/release/superdeduper <source> <target>`. Or just `cargo run --release <source> <target>`.

Since rust-image doesn't support progressive JPEGs, the script `./scripts/baseline-jpeg.sh <dir>` is there to convert them to baseline JPEGs.

