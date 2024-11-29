#!/bin/bash
NDK_VERSION="$(ls "$ANDROID_HOME"/ndk | sort | tail -n 1)"
export PATH="$PATH:$ANDROID_HOME/ndk/${NDK_VERSION}/toolchains/llvm/prebuilt/linux-x86_64/bin/"
cargo clippy --bin=renderer-server
cargo clippy --bin=renderer-desktop --target=x86_64-unknown-linux-gnu
cargo clippy --bin=renderer-desktop --target=x86_64-pc-windows-gnu
cargo clippy --package=renderer-android --target=aarch64-linux-android
cargo clippy --package=renderer-web --target=wasm32-unknown-unknown
