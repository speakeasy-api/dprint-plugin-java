#!/usr/bin/env bash
set -euo pipefail

# Compare dprint-plugin-java output against spotless:palantir-java-format
# Usage: compare.sh /path/to/java/project

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

if [ $# -eq 0 ]; then
  echo "Usage: $0 /path/to/java/project"
  echo ""
  echo "Compares dprint-plugin-java formatting against spotless:palantir-java-format."
  echo "Requires: rust, dprint CLI, java 21, python3"
  exit 1
fi

PROJECT_DIR="$(cd "$1" && pwd)"
WORK_DIR="$(mktemp -d /tmp/fmt-cmp-XXXXXX)"
DPRINT_DIR="$WORK_DIR/dprint"
SPOTLESS_DIR="$WORK_DIR/spotless-runner"
WASM_PATH="$REPO_ROOT/target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm"

echo "=== PJF Comparison ==="
echo "Project:  $PROJECT_DIR"
echo "Work dir: $WORK_DIR"
echo ""

# --- Step 1: Build WASM plugin ---
echo "Building WASM plugin..."
(cd "$REPO_ROOT" && cargo build --release --target wasm32-unknown-unknown --features wasm 2>&1 | tail -1)

if [ ! -f "$WASM_PATH" ]; then
  echo "Error: WASM build failed — $WASM_PATH not found"
  exit 1
fi

WASM_SIZE=$(stat -c%s "$WASM_PATH" 2>/dev/null || stat -f%z "$WASM_PATH")
if [ "$WASM_SIZE" -lt 500000 ]; then
  echo "Error: WASM binary too small (${WASM_SIZE} bytes) — likely missing plugin ABI"
  exit 1
fi
echo "WASM plugin: $(( WASM_SIZE / 1024 ))K"

# --- Step 2: Copy Java files ---
echo "Copying Java files..."
mkdir -p "$DPRINT_DIR" "$SPOTLESS_DIR/src"

FILE_COUNT=0
cd "$PROJECT_DIR"
while IFS= read -r -d '' f; do
  rel="${f#./}"
  mkdir -p "$DPRINT_DIR/$(dirname "$rel")"
  mkdir -p "$SPOTLESS_DIR/src/$(dirname "$rel")"
  cp "$f" "$DPRINT_DIR/$rel"
  cp "$f" "$SPOTLESS_DIR/src/$rel"
  FILE_COUNT=$((FILE_COUNT + 1))
done < <(find . -name "*.java" -not -path "*/build/*" -not -path "*/.gradle/*" -print0)

echo "Copied $FILE_COUNT Java files"

# --- Step 3: Format with dprint ---
echo "Formatting with dprint..."
cat > "$DPRINT_DIR/dprint.json" << EOF
{
  "plugins": ["$WASM_PATH"],
  "java": {}
}
EOF
(cd "$DPRINT_DIR" && dprint fmt "**/*.java" 2>&1 | tail -5)
echo "dprint formatting complete"

# --- Step 4: Format with spotless:PJF ---
echo "Formatting with spotless:PJF..."

cat > "$SPOTLESS_DIR/build.gradle" << 'EOF'
plugins {
    id 'java'
    id 'com.diffplug.spotless' version '7.0.2'
}
repositories { mavenCentral() }
spotless {
    java {
        target 'src/**/*.java'
        palantirJavaFormat()
    }
}
EOF

cat > "$SPOTLESS_DIR/settings.gradle" << 'EOF'
rootProject.name = 'spotless-runner'
EOF

# Copy gradle wrapper from the project if available
if [ -f "$PROJECT_DIR/gradlew" ] && [ -d "$PROJECT_DIR/gradle" ]; then
  cp "$PROJECT_DIR/gradlew" "$SPOTLESS_DIR/"
  cp -r "$PROJECT_DIR/gradle" "$SPOTLESS_DIR/"
  chmod +x "$SPOTLESS_DIR/gradlew"
else
  echo "Warning: No gradle wrapper found in $PROJECT_DIR"
  echo "Please ensure gradlew + gradle/ are available"
  exit 1
fi

# Detect Java 21
if command -v mise &>/dev/null; then
  JAVA_HOME_DIR="$(mise where java@21 2>/dev/null || true)"
  if [ -n "$JAVA_HOME_DIR" ]; then
    export JAVA_HOME="$JAVA_HOME_DIR"
  fi
fi

(cd "$SPOTLESS_DIR" && ./gradlew spotlessApply 2>&1 | tail -5)
echo "spotless:PJF formatting complete"

# --- Step 5: Normalize and compare ---
echo ""
echo "Comparing outputs..."
python3 "$SCRIPT_DIR/normalize.py" "$DPRINT_DIR" "$SPOTLESS_DIR/src" "$FILE_COUNT"
