# CLAUDE.md

## Project Overview

dprint-plugin-java is a Rust+WASM [dprint](https://dprint.dev) plugin that formats Java source code, targeting [palantir-java-format](https://github.com/palantir/palantir-java-format) (PJF) parity. It uses tree-sitter-java for parsing and dprint-core for formatting IR.

## Build & Test Commands

```sh
cargo test                  # run all tests (unit + spec)
cargo test --lib            # unit tests only
cargo test --test spec_test # spec tests only
cargo clippy --all-targets -- -D warnings   # lint check
cargo fmt -- --check        # format check

# Build WASM plugin (requires wasi-sdk)
cargo build --release --target wasm32-unknown-unknown --features wasm
# Output: target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm (~1.1M)

# Full CI pipeline via mise
mise run ci                 # fmt:check -> clippy -> test -> build:wasm
```

## Architecture

### Source Layout

```
src/
  lib.rs                          # crate root, module declarations, conditional WASM exports
  format_text.rs                  # public API: format_text() — parse + generate + format
  wasm_plugin.rs                  # dprint SyncPluginHandler impl (WASM only)
  wasm_shims.rs                   # malloc/free for wasm32 target
  wasm_libc_shims.c              # C libc stubs for tree-sitter's C runtime in WASM
  configuration/
    configuration.rs              # Configuration struct, JavaStyle enum
    resolve_config.rs             # dprint config map -> typed Configuration
  generation/
    generate.rs                   # gen_node() central dispatcher + gen_program() with import sorting
    context.rs                    # FormattingContext: indent, parent stack, continuation indent
    helpers.rs                    # PrintItemsExt trait, is_type_node, collapse_whitespace_len, gen_node_text
    declarations.rs               # package, import, class, interface, enum, record, method, field, etc.
    statements.rs                 # block, if, for, while, switch, try/catch, return, throw, etc.
    expressions.rs                # binary, method invocation/chains, lambda, ternary, object creation, etc.
    comments.rs                   # line_comment, block_comment, javadoc formatting
```

### Key Design Patterns

- **`gen_node()` dispatcher** (generate.rs): routes every tree-sitter node by `kind()` to the appropriate handler. Unrecognized nodes fall back to `gen_node_text()` (source passthrough).
- **`PrintItemsExt` trait** (helpers.rs): ergonomic extension on `PrintItems` — use `items.push_str()`, `items.space()`, `items.newline()`, `items.start_indent()`, `items.finish_indent()` instead of verbose `push_string`/`push_signal` calls.
- **`FormattingContext`** (context.rs): carries `source`, `config`, indent level, parent stack, and continuation indent. Passed as `&mut` to all generation functions.
- **`is_type_node()`** (helpers.rs): deduplicates Java type-kind matching. Note: `generic_type` and `array_type` are included in `is_type_node()` but have dedicated handler arms that must appear **before** the `is_type_node` guard in the dispatcher.
- **`ChainSegment`** struct (expressions.rs): named struct for method chain segments (replaces a 5-tuple).

### Formatting Pipeline

1. `format_text(source, config)` parses Java via tree-sitter
2. `generate(source, tree, config)` walks the AST and emits `PrintItems` IR
3. `dprint_core::formatting::format()` resolves the IR to final text
4. Returns `Ok(None)` if output matches input (already formatted)

## Test Structure

### Unit Tests
- Inline in `format_text.rs`, `resolve_config.rs`, `context.rs`, `helpers.rs`
- Run with `cargo test --lib`

### Spec Tests
- File-based: `tests/specs/**/*.txt` with `== input ==` / `== output ==` markers
- Inline: defined directly in `tests/spec_test.rs`
- All spec tests verify **idempotency** (formatting twice produces no further change)
- Run with `cargo test --test spec_test`

### Updating Spec Expectations
```sh
cargo test --test update_specs -- --ignored   # rewrites all spec file outputs
python3 update_specs.py                        # alternative Python script
```

## Conventions

- **Rust edition 2024**, stable toolchain
- **Zero clippy pedantic warnings** — new code must pass `cargo clippy -- -W clippy::pedantic`
- Functions with unavoidable complexity use `#[allow(clippy::too_many_lines)]`
- Prefer `PrintItemsExt` methods over raw `push_string`/`push_signal`
- Use `collapse_whitespace_len()` (allocation-free) instead of allocating string collapse
- Formatting logic changes must be tested for idempotency
- Dual crate type: `lib` (native, for tests) + `cdylib` (WASM, for distribution)
- WASM feature flag: `--features wasm` required for WASM builds

## PJF Parity Testing

Use the `/compare-pjf` skill (`.claude/commands/compare-pjf.md`) to compare output against spotless:palantir-java-format on real Java projects. The default test corpus is `sdk-javav2` (473 files). Current baseline: ~90% match rate after normalizing import ordering and `java.lang.*` imports.

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| dprint-core | 0.67 | Formatting IR and WASM plugin ABI |
| tree-sitter | 0.24 | Incremental parser framework |
| tree-sitter-java | 0.23 | Java grammar |
| anyhow | 1 | Error handling |
| serde | 1 | Configuration serialization |
| cc | 1 | Build-time C compilation (WASM libc shims) |
