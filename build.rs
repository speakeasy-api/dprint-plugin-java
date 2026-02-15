/// Build script for dprint-plugin-java.
///
/// When targeting wasm32-unknown-unknown, we compile a small C shim that
/// provides malloc/free/calloc/realloc and other libc stubs.  tree-sitter's C
/// runtime calls these functions and, without this shim, they show up as
/// unsatisfied imports from the "env" WASM module.
fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();

    if target == "wasm32-unknown-unknown" {
        cc::Build::new()
            .file("src/wasm_libc_shims.c")
            // Suppress warnings from our intentional stubs.
            .warnings(false)
            .compile("wasm_libc_shims");
    }
}
