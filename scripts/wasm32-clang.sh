#!/bin/sh
# Wrapper script for compiling C sources targeting wasm32-unknown-unknown.
#
# The `cc` crate invokes this as the C compiler when building tree-sitter and
# tree-sitter-java for the wasm32-unknown-unknown target.
#
# It locates the wasi-sdk clang and passes the necessary flags:
#   --target=wasm32-unknown-unknown  — emit wasm32 code (not wasi)
#   --sysroot=...                    — provide C standard library headers
#   -D__wasi__                       — tell tree-sitter to stub out POSIX-only
#                                      functions (e.g. dup()) that are not
#                                      available on wasm32

set -e

# Locate wasi-sdk. Check WASI_SDK_PATH first, then common install locations.
if [ -n "$WASI_SDK_PATH" ]; then
    WASI_SDK="$WASI_SDK_PATH"
elif [ -d "$HOME/.local/wasi-sdk" ]; then
    WASI_SDK="$HOME/.local/wasi-sdk"
elif [ -d "/opt/wasi-sdk" ]; then
    WASI_SDK="/opt/wasi-sdk"
else
    echo "error: wasi-sdk not found. Install it to ~/.local/wasi-sdk or set WASI_SDK_PATH." >&2
    echo "  mkdir -p ~/.local/wasi-sdk" >&2
    echo "  curl -sL https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-30/wasi-sdk-30.0-x86_64-linux.tar.gz \\" >&2
    echo "    | tar xz -C ~/.local/wasi-sdk --strip-components=1" >&2
    exit 1
fi

SYSROOT="$WASI_SDK/share/wasi-sysroot"

exec "$WASI_SDK/bin/clang" \
    --target=wasm32-unknown-unknown \
    "--sysroot=$SYSROOT" \
    -iwithsysroot /include/wasm32-wasi \
    -D__wasi__ \
    "$@"
