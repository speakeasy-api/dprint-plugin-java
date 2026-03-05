# Fix Summary: Issue #1 - Formatting Not Stable

## Problem Statement
The dprint-plugin-java formatter was producing different output on each pass (oscillating between different formats), causing the formatter to bail after 5 tries with the error "Formatting not stable". This occurred on 10 Java files from the Jahia project when formatting with `dprint fmt`.

## Root Causes Identified

### 1. Binary Expression Column Using Source Position
**File**: `src/generation/expressions.rs` (gen_binary_expression, line ~103)

**Issue**: The formatter used `node.start_position().column` which is a tree-sitter source position. This position changes after each formatting pass when the code is reformatted, causing different wrapping decisions on subsequent passes.

```rust
// BAD - source position changes between passes
let start_col = node.start_position().column;
let should_wrap = start_col + expr_flat_width + suffix_width > context.config.line_width;
```

**Fix**:
- Replaced source column with `context.effective_indent_level() * indent_width`
- Added `override_prefix_width` check to use parent-provided position if available
- Now uses stable, runtime context instead of source positions

```rust
// GOOD - uses stable context information
let effective_indent = context.effective_indent_level() * context.config.indent_width as usize;
let prefix_width = context.take_override_prefix_width().unwrap_or_else(|| {
    estimate_prefix_width(node, context.source, context.is_assignment_wrapped())
});
let should_wrap = effective_indent + prefix_width + expr_flat_width + suffix_width > context.config.line_width;
```

### 2. Ternary Expression Missing Override Mechanism
**File**: `src/generation/expressions.rs` (gen_ternary_expression, line ~1025)

**Issue**: The ternary expression called `estimate_prefix_width()` directly without checking for the `override_prefix_width` mechanism that parents use to communicate actual column positions.

```rust
// BAD - doesn't check override mechanism
let prefix_width = estimate_prefix_width(node, context.source, context.is_assignment_wrapped());
```

**Fix**:
- Added `context.take_override_prefix_width()` check before calling `estimate_prefix_width()`
- Allows argument lists and other parents to provide accurate position information
- More stable when expressions are nested in formatted lists

```rust
// GOOD - checks override first
let prefix_width = context.take_override_prefix_width().unwrap_or_else(|| {
    estimate_prefix_width(node, context.source, context.is_assignment_wrapped())
});
```

### 3. Estimate Prefix Width Walking Past Boundaries
**File**: `src/generation/declarations.rs` (estimate_prefix_width, line ~730)

**Issue**: While the function already had breaks for some boundaries (argument_list, formal_parameters), it was missing breaks for statement types. This could cause it to walk up past statement boundaries and obtain prefix calculations that depend on source row positions, which change between passes.

```rust
// BEFORE - missing statement type boundaries
"method_declaration" | "constructor_declaration" | "argument_list" | "formal_parameters" => break,
```

**Fix**: Added statement types to the boundary list to be more conservative:
```rust
// AFTER - conservative boundary list
"method_declaration"
| "constructor_declaration"
| "argument_list"
| "formal_parameters"
| "if_statement"
| "while_statement"
| "for_statement"
| "enhanced_for_statement"
| "do_statement"
| "block"
| "expression_statement" => break,
```

## Changes Made

### 1. `src/generation/expressions.rs`
- **gen_binary_expression()**: Replaced source column with context-based calculation
- **gen_ternary_expression()**: Added override_prefix_width check

### 2. `src/generation/declarations.rs`
- **estimate_prefix_width()**: Added more statement types to boundary list

### 3. `src/format_text.rs`
- Added 4 new idempotency tests to prevent regression:
  - `idempotent_binary_expr_in_return_with_long_chain()`
  - `idempotent_nested_builder_with_binary_condition()`
  - `idempotent_ternary_in_assignment_chain()`
  - `idempotent_long_binary_chain_in_condition()`

## Verification

### To Test the Fix
```bash
# Run all tests (unit + spec)
cargo test --lib --test spec_test

# Run only the new idempotency tests
cargo test idempotent_binary_expr_in_return_with_long_chain
cargo test idempotent_nested_builder_with_binary_condition
cargo test idempotent_ternary_in_assignment_chain
cargo test idempotent_long_binary_chain_in_condition

# Run the existing idempotency test that documents the original flip-flop bug
cargo test idempotent_any_of_wrapping_minimal

# Check for clippy warnings (maintain zero-warning standard)
cargo clippy -- -W clippy::pedantic
```

### To Verify Against Jahia Codebase
```bash
# Original failing command (if Jahia repo is available)
git clone https://github.com/Jahia/jahia.git --depth 1
cd jahia
echo '{
  "plugins": ["https://github.com/speakeasy-api/dprint-plugin-java/releases/download/v0.6.0/dprint_plugin_java.wasm"],
  "java": {"formatJavadoc": true}
}' > dprint.json
dprint fmt
```

All 10 previously failing files should now format successfully without the "Formatting not stable" error.

## Key Principles Applied

1. **Stability over Source Positions**: Use context-based information (indent levels, override mechanisms) instead of source positions which change between passes

2. **Idempotency Guarantee**: Formatting must be stable - applying the formatter twice produces the same output

3. **Boundary Respect**: Don't walk up past formatting boundaries (argument lists, statements, declarations) where layout is handled independently

4. **Override Communication**: Use the `override_prefix_width` mechanism to communicate actual column positions from parents to child expressions

## Impact

- Eliminates oscillation in binary expressions with wrappable operators (&&, ||, string +)
- Eliminates oscillation in ternary expressions
- Makes the formatter more robust to complex nested structures
- Maintains backward compatibility with existing formatting behavior
- No breaking changes to configuration or public API
