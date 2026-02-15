# WASM Plugin Validation Results

## Summary
The WASM binary compiles successfully but fails to load in dprint CLI due to missing import resolution.

## Environment
- dprint CLI version: 0.51.1
- Rust toolchain: stable (as per rust-toolchain.toml)
- WASM target: wasm32-unknown-unknown
- WASM binary location: `/home/vgd/c/speakeasy-api/dprint-plugin-java/target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm`
- WASM binary size: 978K

## Test Setup Created
1. **examples/dprint.json** - dprint configuration pointing to local WASM file
2. **examples/HelloWorld.java** - Simple test file with formatting issues (missing spaces, inconsistent braces)
3. **examples/ComplexExample.java** - Complex test file with imports, methods, if/else, for loops, streams

## Test Results

### Build Status: SUCCESS
```bash
cargo build --release --target=wasm32-unknown-unknown --features wasm
```
Build completed successfully in 6.61s.

### dprint Load Status: FAILED
```bash
cd examples && dprint check
```

**Error:**
```
Error resolving plugin /home/vgd/c/speakeasy-api/dprint-plugin-java/target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm:
Error instantiating module: Error while importing "env"."__rust_alloc": unknown import.
Expected Function(FunctionType { params: [I32, I32], results: [I32] })
```

## Root Cause Analysis

### The Problem
The WASM module expects certain memory allocation functions to be imported from the "env" module:
- `__rust_alloc(size: i32, align: i32) -> i32`
- `__rust_dealloc(ptr: i32, size: i32, align: i32)`
- `__rust_realloc(ptr: i32, old_size: i32, align: i32, new_size: i32) -> i32`

These are Rust's core allocator functions that should be internally available, but dprint's WASM host does not provide them as imports.

### Architecture
The plugin uses a C shim layer (`src/wasm_libc_shims.c`) to bridge tree-sitter's C runtime with Rust's allocator:
1. tree-sitter C code calls `malloc`, `free`, `calloc`, `realloc`
2. C shims wrap these and call Rust's `__rust_alloc`, `__rust_dealloc`, `__rust_realloc`
3. Rust's allocator (dlmalloc for wasm32-unknown-unknown) should provide these

### The Issue
When building a `cdylib` for wasm32-unknown-unknown with `panic = "abort"`, the Rust allocator symbols are apparently not being:
- Exported as WASM function exports (for internal use)
- Properly linked so the C code can call them without importing from "env"

### Evidence
- WASM binary contains references to these symbols (verified with `strings`)
- build.rs correctly compiles the C shims via the cc crate
- The symbols are being treated as imports rather than internal function calls

## Attempted Solutions

### Rebuild from Clean State
Tried `cargo clean --target wasm32-unknown-unknown` followed by full rebuild - same error.

## Related Research

### dprint Plugin Development
- [dprint WASM Plugin Development Guide](https://github.com/dprint/dprint/blob/main/docs/wasm-plugin-development.md)
  - Specifies using `dprint-core` with `wasm` feature
  - Requires `crate-type = ["lib", "cdylib"]`
  - No specific allocator configuration mentioned

### Similar Issues
- [dprint Issue #447](https://github.com/dprint/dprint/issues/447) - Different issue (wasm-bindgen imports)
- [Rust wasm32-unknown-unknown docs](https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html) - Uses dlmalloc as default allocator

### Allocator Information
- wasm32-unknown-unknown uses dlmalloc as the default global allocator
- `panic = "abort"` is the default (and our setting)
- The `__rust_alloc` functions should be automatically available in cdylib builds

## Next Steps to Resolve

1. **Investigate Symbol Export**
   - Determine why `__rust_alloc` et al. are not accessible to C shims
   - May need explicit linker directives or export attributes

2. **Alternative Allocator Approach**
   - Research if wee_alloc or another allocator needs explicit setup
   - Check if `#[global_allocator]` annotation is needed

3. **Compare with Working Plugins**
   - Examine source of dprint-plugin-typescript or dprint-plugin-toml
   - Check if they use tree-sitter or similar C dependencies
   - Look for any special WASM build configuration

4. **dprint-core Integration**
   - Verify dprint-core 0.67.4 expectations
   - Check if newer/older versions handle allocator differently
   - Review dprint-core's WASM module loading code

5. **Build System Configuration**
   - Investigate if .cargo/config.toml settings affect symbol visibility
   - Check if wasi-sdk wrapper scripts need modification
   - Explore if different rustc flags are needed for symbol export

## Validation Commands (for future testing)

Once the WASM loading issue is resolved:

```bash
cd examples

# Check formatting
dprint check

# Format files
dprint fmt

# Verify formatting applied
git diff HelloWorld.java ComplexExample.java

# Test diagnostic commands
dprint output-format-times
dprint output-resolved-config
```

## Current Status
**BLOCKED** - Cannot proceed with dprint CLI validation until WASM module loading is fixed.

## Files Created
- `/home/vgd/c/speakeasy-api/dprint-plugin-java/examples/dprint.json`
- `/home/vgd/c/speakeasy-api/dprint-plugin-java/examples/HelloWorld.java`
- `/home/vgd/c/speakeasy-api/dprint-plugin-java/examples/ComplexExample.java`
- `/home/vgd/c/speakeasy-api/dprint-plugin-java/examples/VALIDATION_RESULTS.md` (this file)
