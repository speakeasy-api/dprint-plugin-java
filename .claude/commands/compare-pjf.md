# Compare dprint-plugin-java vs spotless:PJF

Compare the output of our dprint-plugin-java formatter against spotless:palantir-java-format on a real Java project.

## Steps

1. **Build the WASM plugin:**
   ```
   cargo build --target wasm32-unknown-unknown --release --features wasm
   ```
   Verify the output is >500K at `target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm`.

2. **Set up comparison directories:**
   ```
   rm -rf /tmp/fmt-comparison
   mkdir -p /tmp/fmt-comparison/{dprint,spotless-runner/src}
   ```

3. **Copy Java files from test project** (default: `$ARGUMENTS` or `/home/vgd/c/speakeasy-api/openapi-generation/zSDKs/sdk-javav2`):
   ```
   cd <project-dir>
   find . -name "*.java" -not -path "*/build/*" -not -path "*/.gradle/*" | while read f; do
     mkdir -p "/tmp/fmt-comparison/dprint/$(dirname "$f")"
     mkdir -p "/tmp/fmt-comparison/spotless-runner/src/$(dirname "$f")"
     cp "$f" "/tmp/fmt-comparison/dprint/$f"
     cp "$f" "/tmp/fmt-comparison/spotless-runner/src/$f"
   done
   ```

4. **Run dprint formatter:**
   ```
   cat > /tmp/fmt-comparison/dprint/dprint.json << 'EOF'
   {
     "plugins": [
       "/home/vgd/c/speakeasy-api/dprint-plugin-java/target/wasm32-unknown-unknown/release/dprint_plugin_java.wasm"
     ],
     "java": {}
   }
   EOF
   cd /tmp/fmt-comparison/dprint && dprint fmt "**/*.java"
   ```

5. **Run spotless:PJF:**
   Create `/tmp/fmt-comparison/spotless-runner/build.gradle`:
   ```groovy
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
   ```
   Create `/tmp/fmt-comparison/spotless-runner/settings.gradle`:
   ```groovy
   rootProject.name = 'spotless-runner'
   ```
   Copy gradlew + gradle/ wrapper from any Java project, then:
   ```
   eval "$(mise activate bash 2>/dev/null)"
   cd /tmp/fmt-comparison/spotless-runner
   JAVA_HOME="$(mise where java@21)" ./gradlew spotlessApply
   ```

6. **Compare outputs:**
   dprint files: `/tmp/fmt-comparison/dprint/`
   PJF files: `/tmp/fmt-comparison/spotless-runner/src/`

   Run a diff comparison counting identical vs different files, then categorize differences.
   Normalize before comparing:
   - Remove `java.lang.*` imports (PJF strips these; formatter's job is debatable)
   - Sort imports alphabetically (import ordering is separate from formatting)
   - Strip trailing whitespace

7. **Report** the match rate and categorize remaining formatting gaps.
