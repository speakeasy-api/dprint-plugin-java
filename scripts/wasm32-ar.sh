#!/bin/sh
# Wrapper script for the archiver targeting wasm32-unknown-unknown.
#
# The `cc` crate uses this to create static libraries from the compiled
# wasm32 object files.

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
    exit 1
fi

exec "$WASI_SDK/bin/llvm-ar" "$@"
