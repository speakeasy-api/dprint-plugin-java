/*
 * Minimal libc shims for wasm32-unknown-unknown.
 *
 * tree-sitter's C runtime calls malloc/free/calloc/realloc and a handful of
 * other libc functions.  On wasm32-unknown-unknown there is no libc, so the
 * compiled C object files reference these as *imports* from the "env" module.
 * dprint's WASM host does not provide those imports, so instantiation fails.
 *
 * Memory allocation is provided by Rust-side #[no_mangle] extern "C"
 * functions (see wasm_shims.rs) that delegate to Rust's global allocator.
 * The remaining functions are no-op stubs for tree-sitter's error-reporting
 * and debug paths that should never be exercised during normal formatting.
 */

#include <stddef.h>

/* ------------------------------------------------------------------ */
/* String comparison -- tree-sitter uses strncmp for language matching */
/* ------------------------------------------------------------------ */
int strncmp(const char *s1, const char *s2, size_t n) {
    for (size_t i = 0; i < n; i++) {
        unsigned char c1 = (unsigned char)s1[i];
        unsigned char c2 = (unsigned char)s2[i];
        if (c1 != c2) return c1 < c2 ? -1 : 1;
        if (c1 == 0) return 0;
    }
    return 0;
}

/* ------------------------------------------------------------------ */
/* Stubs -- referenced by tree-sitter but not called during formatting */
/* ------------------------------------------------------------------ */

/* FILE* operations (tree-sitter's debug/logging code) */
typedef struct { int dummy; } FILE;
int fprintf(FILE *f, const char *fmt, ...) { (void)f; (void)fmt; return 0; }
int snprintf(char *buf, size_t n, const char *fmt, ...) {
    (void)buf; (void)n; (void)fmt;
    if (n > 0) buf[0] = 0;
    return 0;
}

typedef __builtin_va_list va_list;
int vsnprintf(char *buf, size_t n, const char *fmt, va_list ap) {
    (void)buf; (void)n; (void)fmt; (void)ap;
    if (n > 0) buf[0] = 0;
    return 0;
}

int fclose(FILE *f) { (void)f; return 0; }
FILE *fdopen(int fd, const char *mode) { (void)fd; (void)mode; return (FILE *)0; }
size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *f) {
    (void)ptr; (void)size; (void)nmemb; (void)f;
    return 0;
}
int fputc(int c, FILE *f) { (void)c; (void)f; return c; }

/* time */
struct timespec { long tv_sec; long tv_nsec; };
int clock_gettime(int clk_id, struct timespec *tp) {
    (void)clk_id;
    if (tp) { tp->tv_sec = 0; tp->tv_nsec = 0; }
    return 0;
}

/* abort / assert */
_Noreturn void abort(void) { __builtin_trap(); }
void __assert_fail(const char *expr, const char *file, unsigned int line, const char *func) {
    (void)expr; (void)file; (void)line; (void)func;
    __builtin_trap();
}
