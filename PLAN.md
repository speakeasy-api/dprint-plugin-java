# dprint-plugin-java — Implementation Plan

A dprint WASM plugin for formatting Java source code, inspired by
[palantir-java-format](https://github.com/palantir/palantir-java-format).
Written in Rust, targeting `wasm32-unknown-unknown`.

## Architecture

```
src/
├── lib.rs                      # Crate root; conditionally includes wasm_plugin
├── wasm_plugin.rs              # SyncPluginHandler impl + generate_plugin_code!
├── format_text.rs              # Entry: parse → generate IR → print
├── configuration/
│   ├── mod.rs
│   ├── configuration.rs        # Configuration struct + JavaStyle enum
│   └── resolve_config.rs       # Raw config → typed Configuration
└── generation/                 # CST → PrintItems IR (the bulk of the work)
    ├── mod.rs
    ├── generate.rs             # Top-level dispatcher: walk CST, delegate per node kind
    ├── context.rs              # Formatting state (current indent, etc.)
    ├── declarations.rs         # Class, interface, enum, record, method, field, constructor
    ├── statements.rs           # if/else, for, while, switch, try/catch, return, etc.
    ├── expressions.rs          # Method chains, lambdas, ternary, binary ops, etc.
    ├── types.rs                # Type references, generics, bounds, wildcards
    ├── annotations.rs          # Annotations and annotation declarations
    ├── imports.rs              # Import sorting/grouping
    ├── comments.rs             # Line/block comment handling, Javadoc formatting
    └── helpers.rs              # Shared IR helpers (separator lists, wrapping logic)
```

### Pipeline

```
Java source (String)
  → tree-sitter-java parser → CST (tree_sitter::Tree)
  → generation/ module       → dprint_core::formatting::PrintItems (IR)
  → dprint_core::formatting::format() → formatted output (String)
```

### Key Dependencies

| Crate            | Purpose                                    |
|------------------|--------------------------------------------|
| `dprint-core`    | Plugin framework (`wasm` feature) + IR printer (`formatting` feature) |
| `tree-sitter`    | Incremental parser runtime                 |
| `tree-sitter-java` | Java grammar for tree-sitter            |
| `serde` / `serde_json` | Configuration serialization           |
| `anyhow`         | Error handling                             |

## Configuration

Inspired by palantir-java-format's deliberately minimal configuration:

| Key                    | Type    | Default       | Description                                    |
|------------------------|---------|---------------|------------------------------------------------|
| `style`                | enum    | `"palantir"`  | Preset: `palantir`, `google`, or `aosp`        |
| `lineWidth`            | u32     | 120 (palantir)| Maximum line width before wrapping             |
| `indentWidth`          | u8      | 4 (palantir)  | Spaces per indentation level                   |
| `useTabs`              | bool    | false         | Use tabs instead of spaces                     |
| `newLineKind`          | enum    | `"lf"`        | `auto`, `lf`, or `crlf`                       |
| `formatJavadoc`        | bool    | false         | Whether to format Javadoc comments             |
| `methodChainThreshold` | u32     | 80            | Width at which method chains break to new lines|
| `inlineLambdas`        | bool    | true          | Prefer single-line lambdas when they fit       |

Style presets:
- **palantir**: 120 line width, 4-space indent
- **google**: 100 line width, 2-space indent
- **aosp**: 100 line width, 4-space indent

## Palantir-Style Formatting Rules

Key behaviors to replicate from palantir-java-format:

### Indentation
- 4-space indent (when using palantir style)
- Continuation indent = 2× base indent (8 spaces for palantir)
- `switch` case bodies indented once from `case:`

### Line Wrapping
- 120-character max line width
- Wrap before `.` in method chains
- Break after `(` and before `)` for long parameter/argument lists
- Break after binary operators (`+`, `&&`, `||`, etc.)
- Prefer breaking at higher precedence points first

### Lambda Formatting
- Inline lambdas when the entire expression fits on one line
- For multi-statement lambdas, format as block with `{` on same line

### Method Chains
- When a chained expression exceeds `methodChainThreshold` (80 chars),
  break each `.method()` call onto its own line, indented once
- First receiver stays on the opening line

### Import Sorting
- Sort imports lexicographically
- Group: `java.*`, then `javax.*`, then everything else, then `static`
- Single blank line between groups

### Blank Lines
- Exactly one blank line between top-level declarations
- No blank line after opening brace or before closing brace
- Preserve one blank line between method-local blocks when present

### Comments
- Preserve inline comments at end of lines
- Block comments reformatted to fit within line width (if `formatJavadoc` enabled)
- `//` comments preserve their content

---

## Phased Development Roadmap

### Phase 0: Scaffold (DONE)
- [x] Project structure, Cargo.toml, dependencies
- [x] Configuration module with style presets
- [x] Pass-through formatting (parse → emit unchanged)
- [x] SyncPluginHandler wired up
- [x] Native compilation + 6 passing tests

### Phase 1: WASM Build Toolchain (DONE)
- [x] Resolve tree-sitter C runtime compilation for `wasm32-unknown-unknown`
  - **Chosen: Option A** — wasi-sdk clang with wrapper scripts
  - `.cargo/config.toml` sets `CC_wasm32_unknown_unknown` and `AR_wasm32_unknown_unknown`
    to wrapper scripts in `scripts/` that locate the wasi-sdk installation
  - `scripts/wasm32-clang.sh` invokes wasi-sdk's clang with
    `--target=wasm32-unknown-unknown`, `--sysroot`, and `-D__wasi__`
  - `-D__wasi__` tells tree-sitter to stub out POSIX `dup()` which is
    unavailable in wasm32-unknown-unknown
- [x] Verify `cargo build --release --target=wasm32-unknown-unknown --features wasm` produces a working `.wasm`
- [ ] Test plugin loading with dprint CLI

### Phase 2: Core Formatting Infrastructure
- [ ] Implement `context.rs` — formatting state tracking (current indent level, parent nodes, etc.)
- [ ] Implement `helpers.rs` — common IR patterns:
  - Comma-separated lists with wrapping
  - Block formatting (opening brace, indented body, closing brace)
  - Conditional wrapping based on line width
- [ ] Implement `generate.rs` dispatcher — match on `node.kind()` and delegate to per-construct generators
- [ ] Add comprehensive test fixtures in `tests/specs/`

### Phase 3: Declarations
- [ ] Package declaration
- [ ] Import declarations + sorting/grouping
- [ ] Class declarations (modifiers, extends, implements, permits)
- [ ] Interface declarations
- [ ] Enum declarations (constants, body)
- [ ] Record declarations (components, body)
- [ ] Annotation type declarations
- [ ] Method declarations (modifiers, type params, params, throws, body)
- [ ] Constructor declarations
- [ ] Field declarations
- [ ] Initializer blocks (static and instance)

### Phase 4: Statements
- [ ] Block statements (`{ ... }`)
- [ ] Variable declaration statements
- [ ] Expression statements
- [ ] `if` / `else if` / `else`
- [ ] `for` / enhanced-for
- [ ] `while` / `do-while`
- [ ] `switch` statements and expressions (including arrow-case)
- [ ] `try` / `catch` / `finally` / try-with-resources
- [ ] `return`, `throw`, `break`, `continue`, `yield`
- [ ] `synchronized` blocks
- [ ] `assert` statements
- [ ] Labeled statements

### Phase 5: Expressions
- [ ] Binary expressions (with operator precedence-based wrapping)
- [ ] Unary expressions
- [ ] Method invocation (argument wrapping)
- [ ] Method chains (threshold-based breaking)
- [ ] Lambda expressions (inline vs block)
- [ ] Ternary / conditional expressions
- [ ] `new` expressions (constructor calls, anonymous classes)
- [ ] Array creation and access
- [ ] Cast expressions
- [ ] `instanceof` (including pattern matching)
- [ ] String concatenation (with smart wrapping)
- [ ] Parenthesized expressions
- [ ] Method references (`Class::method`)

### Phase 6: Types and Annotations
- [ ] Primitive types, void
- [ ] Class/interface types with generics (`Map<K, V>`)
- [ ] Wildcard types (`? extends T`, `? super T`)
- [ ] Array types
- [ ] Type parameters and bounds
- [ ] Annotations (marker, single-value, normal)
- [ ] Type annotations (Java 8+)

### Phase 7: Comments and Javadoc
- [ ] Line comment preservation
- [ ] Block comment preservation
- [ ] Comment association (attach to correct AST node)
- [ ] Javadoc formatting (when `formatJavadoc` is enabled):
  - Reflow `@param`, `@return`, `@throws` tags
  - Code blocks, lists, links
  - Line wrapping within Javadoc

### Phase 8: Polish and Release
- [ ] Idempotency testing (format twice = same result)
- [ ] Test against real-world Java codebases (OpenJDK, Spring, Guava)
- [ ] Performance benchmarking
- [ ] JSON Schema for configuration
- [ ] GitHub release pipeline (build WASM, publish)
- [ ] npm package for distribution
- [ ] Plugin registry entry at plugins.dprint.dev

---

## Open Questions / Decisions

1. **tree-sitter WASM compilation strategy**: RESOLVED — using wasi-sdk's clang
   via wrapper scripts.  See `scripts/wasm32-clang.sh` and `.cargo/config.toml`.

2. **Continuation indent**: palantir uses 2× base indent for continuation
   lines. Should this be configurable or fixed to style preset?

3. **Import grouping**: palantir groups as `java.` → `javax.` → third-party → `static`.
   Should custom grouping be supported?

4. **Blank line policy**: How strictly should we match palantir's blank line
   behavior vs. offering configuration?

5. **Error recovery formatting**: Currently we skip formatting on parse errors.
   Should we attempt best-effort formatting for files with errors?

6. **Range formatting**: tree-sitter gives us byte offsets. We should implement
   `format_range` support for editor integrations, but this can come later.

---

## dprint.json Example Usage

```json
{
  "java": {
    "style": "palantir",
    "lineWidth": 120,
    "indentWidth": 4,
    "formatJavadoc": true
  },
  "plugins": [
    "https://plugins.dprint.dev/speakeasy-api/dprint-plugin-java/0.1.0/plugin.wasm"
  ]
}
```

## References

- [dprint WASM plugin development](https://github.com/dprint/dprint/blob/main/docs/wasm-plugin-development.md)
- [dprint-core crate](https://crates.io/crates/dprint-core)
- [palantir-java-format](https://github.com/palantir/palantir-java-format)
- [tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java)
- [dprint-plugin-json (reference plugin)](https://github.com/dprint/dprint-plugin-json)
- [dprint-plugin-typescript (complex reference)](https://github.com/dprint/dprint-plugin-typescript)
