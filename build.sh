#!/bin/bash
cargo build --bin=renderer-server
cargo build --bin=renderer-desktop --target=x86_64-unknown-linux-gnu
cargo build --bin=renderer-desktop --target=x86_64-pc-windows-gnu
(cd android && ./gradlew build)
(cd crates/renderer-web && wasm-pack build --no-opt) && (cd web && npm i && npm run build)
