//! C-ABI memory allocation shims for `wasm32-unknown-unknown`.
//!
//! tree-sitter's C runtime calls `malloc`, `free`, `calloc`, and `realloc`.
//! On `wasm32-unknown-unknown` there is no libc, so these symbols would become
//! unresolved WASM imports from the `"env"` module.  By providing them here as
//! `#[no_mangle] extern "C"` functions we resolve them at link time within the
//! same WASM module, delegating to Rust's global allocator via
//! [`std::alloc::alloc`] / [`std::alloc::dealloc`] / [`std::alloc::realloc`].
//!
//! Each allocation is prefixed with an 8-byte header that stores the usable
//! size so that `free` can pass the correct size to the deallocator.

use std::alloc::{self, Layout};

/// Alignment used for all allocations.  WASM's `max_align_t` is 8.
const ALIGN: usize = 8;
/// Size of the header prepended to every allocation (stores the usable size).
const HEADER: usize = 8;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
    unsafe {
        let size = if size == 0 { 1 } else { size };
        let total = HEADER + size;
        let layout = match Layout::from_size_align(total, ALIGN) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };
        let raw = alloc::alloc(layout);
        if raw.is_null() {
            return core::ptr::null_mut();
        }
        // Store the usable size in the header.
        *(raw as *mut usize) = size;
        raw.add(HEADER)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn free(ptr: *mut u8) {
    unsafe {
        if ptr.is_null() {
            return;
        }
        let raw = ptr.sub(HEADER);
        let size = *(raw as *const usize);
        let total = HEADER + size;
        let layout = match Layout::from_size_align(total, ALIGN) {
            Ok(l) => l,
            Err(_) => return,
        };
        alloc::dealloc(raw, layout);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn calloc(nmemb: usize, size: usize) -> *mut u8 {
    unsafe {
        let total_size = nmemb.wrapping_mul(size);
        let usable = if total_size == 0 { 1 } else { total_size };
        let total = HEADER + usable;
        let layout = match Layout::from_size_align(total, ALIGN) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };
        let raw = alloc::alloc_zeroed(layout);
        if raw.is_null() {
            return core::ptr::null_mut();
        }
        *(raw as *mut usize) = usable;
        raw.add(HEADER)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    unsafe {
        if ptr.is_null() {
            return malloc(new_size);
        }
        if new_size == 0 {
            free(ptr);
            return core::ptr::null_mut();
        }
        let raw = ptr.sub(HEADER);
        let old_size = *(raw as *const usize);
        let old_total = HEADER + old_size;
        let new_total = HEADER + new_size;
        let layout = match Layout::from_size_align(old_total, ALIGN) {
            Ok(l) => l,
            Err(_) => return core::ptr::null_mut(),
        };
        let new_raw = alloc::realloc(raw, layout, new_total);
        if new_raw.is_null() {
            return core::ptr::null_mut();
        }
        *(new_raw as *mut usize) = new_size;
        new_raw.add(HEADER)
    }
}
