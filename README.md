# dprint-plugin-java

A [dprint](https://dprint.dev) plugin for formatting Java code, inspired by [palantir-java-format](https://github.com/palantir/palantir-java-format).

Built as a Rust WASM plugin using [tree-sitter-java](https://github.com/tree-sitter/tree-sitter-java) for parsing.

## Installation

Add the plugin to your `dprint.json`:

```json
{
  "plugins": [
    "https://github.com/speakeasy-api/dprint-plugin-java/releases/latest/download/dprint_plugin_java.wasm"
  ],
  "java": {}
}
```

## Configuration

| Option | Type | Default | Description |
|---|---|---|---|
| `style` | `"palantir"` \| `"google"` | `"palantir"` | Base style preset |
| `lineWidth` | number | `120` (palantir) / `100` (google) | Maximum line width |
| `indentWidth` | number | `4` (palantir) / `2` (google) | Spaces per indent level |
| `useTabs` | boolean | `false` | Use tabs instead of spaces |
| `newLineKind` | `"lf"` \| `"crlf"` \| `"system"` | `"lf"` | Line ending style |
| `formatJavadoc` | boolean | `false` | Format Javadoc comments |
| `methodChainThreshold` | number | `80` | Column threshold for breaking method chains |
| `inlineLambdas` | boolean | `true` | Keep short lambdas on one line |

Example configuration:

```json
{
  "java": {
    "style": "google",
    "lineWidth": 100,
    "indentWidth": 2
  }
}
```

## Supported Constructs

- **Declarations**: packages, imports, classes, interfaces, enums, records, methods, constructors, fields
- **Statements**: blocks, if/else, for, enhanced for, while, do-while, switch, try/catch/finally, try-with-resources, return, throw, break, continue, assert, synchronized, labeled statements
- **Expressions**: binary, unary, method invocation, field access, lambda, ternary, object creation, cast, instanceof, array access, method reference, parenthesized

## Development

### Prerequisites

- Rust (stable toolchain)
- [wasi-sdk](https://github.com/aspect-build/aspect-gcc-toolchain/releases) (for WASM builds)

### Running Tests

```sh
cargo test
```

### Building WASM

```sh
export CC_wasm32_unknown_unknown="$WASI_SDK_PATH/bin/clang"
cargo build --release --target wasm32-unknown-unknown --features wasm
```

The WASM binary will be at `target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm`.

### Testing with dprint

```sh
cd examples
dprint check    # verify formatting
dprint fmt      # apply formatting
```

## License

Apache-2.0
