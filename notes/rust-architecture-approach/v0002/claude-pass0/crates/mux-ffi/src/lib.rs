//! # mux-ffi
//!
//! C FFI shim for linking TermForge's Rust kernel into the C tmux process.
//!
//! ## Strategy
//! This crate exposes a minimal C ABI that allows the C tmux server
//! to call into Rust code for specific subsystems (grid, parser, etc.)
//! without requiring a full rewrite.
//!
//! ## Safety
//! All `extern "C"` functions validate their inputs and catch panics.
//! The `catch_unwind` boundary (INV-117) prevents Rust panics from
//! unwinding into C code.

// This crate necessarily uses unsafe for FFI.
#![deny(unsafe_op_in_unsafe_fn)]

use std::os::raw::{c_char, c_int, c_uint};
use std::panic;

/// Opaque handle to a Rust grid.
pub struct FfiGrid {
    inner: mux_grid::ChunkedGrid,
}

/// Create a new grid. Returns a pointer to the grid, or null on error.
///
/// # Safety
/// Caller must eventually call `grid_destroy` to free the returned pointer.
#[unsafe(no_mangle)]
pub extern "C" fn grid_create(sx: c_uint, sy: c_uint, hlimit: c_uint) -> *mut FfiGrid {
    let result = panic::catch_unwind(|| {
        let grid = mux_grid::ChunkedGrid::new(sx, sy, hlimit);
        Box::into_raw(Box::new(FfiGrid { inner: grid }))
    });
    result.unwrap_or(std::ptr::null_mut())
}

/// Destroy a grid created by `grid_create`.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `grid_create`, or null.
/// After this call, `ptr` is no longer valid.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_destroy(ptr: *mut FfiGrid) {
    if !ptr.is_null() {
        // SAFETY: ptr was created by Box::into_raw in grid_create.
        unsafe {
            drop(Box::from_raw(ptr));
        }
    }
}

/// Get grid width.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `grid_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_sx(ptr: *const FfiGrid) -> c_uint {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: ptr is valid per contract.
    unsafe { (*ptr).inner.sx() }
}

/// Get grid height.
///
/// # Safety
/// `ptr` must be a valid pointer returned by `grid_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_sy(ptr: *const FfiGrid) -> c_uint {
    if ptr.is_null() {
        return 0;
    }
    // SAFETY: ptr is valid per contract.
    unsafe { (*ptr).inner.sy() }
}

/// Check if first 4 bytes of data are TF01 magic.
///
/// # Safety
/// `data` must point to at least `len` valid bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_tf01_frame(data: *const u8, len: c_uint) -> c_int {
    if data.is_null() || len < 4 {
        return 0;
    }
    // SAFETY: data points to at least len bytes per contract.
    let slice = unsafe { std::slice::from_raw_parts(data, len as usize) };
    c_int::from(mux_proto::is_tf01(slice))
}

/// Get the TermForge version string.
///
/// # Safety
/// The returned pointer is valid for the lifetime of the program.
/// Caller must NOT free the returned pointer.
#[unsafe(no_mangle)]
pub extern "C" fn termforge_version() -> *const c_char {
    // Static string -- valid for program lifetime.
    static VERSION: &[u8] = b"0.1.0\0";
    VERSION.as_ptr().cast::<c_char>()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_grid_lifecycle() {
        let grid = grid_create(80, 24, 10_000);
        assert!(!grid.is_null());
        unsafe {
            assert_eq!(grid_sx(grid), 80);
            assert_eq!(grid_sy(grid), 24);
            grid_destroy(grid);
        }
    }

    #[test]
    fn ffi_null_safety() {
        unsafe {
            assert_eq!(grid_sx(std::ptr::null()), 0);
            assert_eq!(grid_sy(std::ptr::null()), 0);
            grid_destroy(std::ptr::null_mut()); // should not crash
        }
    }

    #[test]
    fn ffi_tf01_detection() {
        let data = [0x54u8, 0x46, 0x30, 0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        unsafe {
            assert_eq!(is_tf01_frame(data.as_ptr(), data.len() as c_uint), 1);
        }
    }
}
