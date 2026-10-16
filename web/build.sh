#!/bin/sh
# Build the browser front end into web/www (static files; serve with any HTTP server).
set -e
cd "$(dirname "$0")/.."
cargo build --release --target wasm32-unknown-unknown -p rv-doom-web
cp target/wasm32-unknown-unknown/release/rv_doom_web.wasm web/www/
mkdir -p web/www/programs
cp programs/mandel.bin programs/primes.bin programs/hello.bin programs/fbtest.bin web/www/programs/
if [ -f doom/doom.bin ]; then cp doom/doom.bin web/www/; else echo "note: doom/doom.bin not built; Doom entry will fall back"; fi
echo "built web/www. Serve: python3 -m http.server -d web/www 8000"
