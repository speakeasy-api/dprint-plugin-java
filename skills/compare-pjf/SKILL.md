---
name: compare-pjf
description: >
  Compare dprint-plugin-java output against spotless:palantir-java-format on a
  Java project. Builds the WASM plugin, formats with both tools, normalizes
  imports, and reports match rate with categorized diffs.
compatibility: Requires bash, python3, dprint CLI, Java 21 (via mise), and gradle wrapper.
metadata:
  author: speakeasy-api
  version: "1.0"
---

# Compare dprint-plugin-java vs spotless:PJF

Compare the output of dprint-plugin-java against spotless:palantir-java-format on a real Java project to measure formatting parity.

## Usage

```bash
# Via mise task runner:
mise run compare-pjf /path/to/java/project

# Directly:
bash skills/compare-pjf/scripts/compare.sh /path/to/java/project
```

If no project path is given, a usage message is printed.

## What it does

1. **Builds** the WASM plugin (`cargo build --release --target wasm32-unknown-unknown --features wasm`)
2. **Copies** `.java` files from the target project (excluding `build/` and `.gradle/` dirs)
3. **Formats with dprint** using the freshly-built WASM plugin
4. **Formats with spotless:PJF** using a temporary Gradle wrapper project
5. **Normalizes** both outputs (strips `java.lang.*` imports, sorts imports, strips trailing whitespace)
6. **Compares** and reports match rate with unified diffs for mismatches

## Requirements

- Rust stable toolchain with `wasm32-unknown-unknown` target
- `dprint` CLI installed
- Java 21 (auto-detected via `mise where java@21` or `$JAVA_HOME`)
- `python3`
- A Gradle wrapper (`gradlew` + `gradle/`) — the script copies one from the target project if available

## Output

```
=== PJF Comparison Results ===
Total files: 473
Matching:    471 (99.6%)
Differing:   2

Diff details for mismatching files are printed below.
```

## Key notes

- Always compare from **fresh source files**, not re-formatted output. Re-formatting inflates match rates.
- Normalization removes `java.lang.*` imports (PJF strips these; our formatter intentionally keeps them) and sorts import blocks alphabetically.
- Current baseline on `sdk-javav2` (473 files): **99.6% match rate**.
