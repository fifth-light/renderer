#!/bin/bash
cargo build --bin=renderer-server
cargo build --bin=renderer-desktop --target=x86_64-unknown-linux-gnu
cargo build --bin=renderer-desktop --target=x86_64-pc-windows-gnu
(cd android && ./gradlew build)
