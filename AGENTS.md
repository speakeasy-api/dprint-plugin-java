# AGENTS.md

Guidelines for AI agents working on this codebase.

## Project Context

This is a Rust+WASM dprint plugin that formats Java code to match palantir-java-format output. It parses Java via tree-sitter and emits dprint-core `PrintItems` IR. The codebase is ~6500 lines of Rust across 11 source files.

**Current parity:** 99.6% (471/473 files) against spotless:palantir-java-format on the `sdk-javav2` corpus, after normalizing imports and trailing whitespace.

## Before Making Changes

1. **Read before editing.** Always read the relevant file(s) before proposing changes. Understand the existing patterns.
2. **Run the full test suite** before and after changes: `cargo test --lib --test spec_test`
3. **Check clippy**: `cargo clippy --all-targets -- -W clippy::pedantic` — the codebase maintains zero warnings.

## Code Style

- Use `PrintItemsExt` trait methods (`items.push_str()`, `items.space()`, `items.newline()`, `items.start_indent()`, `items.finish_indent()`) instead of raw `push_string(...to_string())` or `push_signal(Signal::...)`.
- Use `collapse_whitespace_len()` from helpers.rs for width estimation (allocation-free). Do not use string-allocating alternatives.
- Use `is_type_node()` from helpers.rs when matching Java type node kinds, but be aware that `generic_type` and `array_type` have dedicated handlers and must match before the `is_type_node` guard in the dispatcher.
- Use `gen_node_text()` from helpers.rs for source-passthrough nodes.
- Prefer `let-else` over `match` for single-pattern destructuring.
- Add `#[allow(clippy::too_many_lines)]` only on functions that genuinely need it (formatting functions with many match arms).

## Architecture Rules

- **`gen_node()` in generate.rs** is the central dispatcher. All new node types must be routed here.
- **Match arm ordering matters** in the dispatcher: specific arms (like `"generic_type"`) must appear before guard arms (like `kind if is_type_node(kind)`).
- **All formatting functions** take `(node: tree_sitter::Node, context: &mut FormattingContext)` and return `PrintItems`.
- **FormattingContext** tracks indent level, parent stack, and continuation indent. Always use `context.indent()`/`context.dedent()` rather than raw signal manipulation for block-level indent.
- **Module boundaries**: declarations.rs handles Java declarations, statements.rs handles statements, expressions.rs handles expressions, comments.rs handles comments. Don't mix responsibilities.

## Testing

- **Spec tests** use `.txt` files in `tests/specs/` with `== input ==` / `== output ==` markers.
- **Every formatting change must be idempotent**: formatting the output again must produce no change. The test framework verifies this automatically.
- To update spec expectations after intentional formatting changes: `cargo test --test update_specs -- --ignored`
- **Never silently change spec expectations** to make tests pass. If a spec test fails, understand why before updating it.

## Common Pitfalls

- **Width estimation**: `estimate_prefix_width()` and `collapse_whitespace_len()` work on source text positions, which may not reflect the formatted column. Be careful with wrapping thresholds that depend on column position.
- **Idempotency**: If pass 1 wraps differently than pass 2 (because wrapping changes column positions), the formatter oscillates. Always test idempotency with `cargo test`.
- **WASM builds**: Must use `--features wasm` flag. Without it, the binary is ~21K (missing plugin ABI). Correct size is ~1.1M.
- **tree-sitter node kinds**: Unnamed nodes (punctuation like `{`, `}`, `,`, `;`) must be handled explicitly in match arms. The `_ => {}` fallback silently drops them.

## PJF Comparison

To verify formatting parity against spotless:palantir-java-format, use the agent-agnostic comparison skill:

```bash
# Via mise:
mise run compare-pjf /path/to/java/project

# Directly:
bash skills/compare-pjf/scripts/compare.sh /path/to/java/project
```

See `skills/compare-pjf/SKILL.md` for full details.

Key points:
- Compare from **fresh source files**, not re-formatted files (re-formatting inflates match rates).
- Normalization removes `java.lang.*` imports, sorts imports, and strips trailing whitespace.
- Test corpus: `sdk-javav2` (473 Java files). Current baseline: **99.6% match rate**.

## Remaining Formatting Gaps (3 hunks across 2 files)

| Category | Files | Hunks | Notes |
|---|---|---|---|
| BINARY_WRAP | 1 | 1 | PJF wraps `&&` at ~100 cols, we use 120 |
| ARG_WRAP | 1 | 1 | Arg wrapping near trailing `//` comment |
| IMPORT | 1 | 1 | `java.lang.IllegalStateException` kept by us |

## Release Process

```bash
# Automated via mise:
VERSION=0.6.0 mise run release:tag

# Manual steps:
# 1. Update version in Cargo.toml
# 2. Run tests: cargo test
# 3. Commit: git add Cargo.toml && git commit -m "Bump version to 0.6.0"
# 4. Tag: git tag v0.6.0
# 5. Push: git push origin master && git push origin v0.6.0
# CI (.github/workflows/ci.yml) builds WASM and creates GitHub release on tag push.
```

## Build Requirements

- Rust stable toolchain (edition 2024)
- wasi-sdk 25 for WASM builds (auto-detected via `WASI_SDK_PATH` or `~/.local/share/wasi-sdk`)
- mise task runner (optional, for `mise run ci`)
