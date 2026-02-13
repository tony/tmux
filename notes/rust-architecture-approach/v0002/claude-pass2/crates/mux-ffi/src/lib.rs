//! # mux-ffi
//!
//! C FFI shim with `catch_unwind` panic recovery (INV-117).

#![deny(unsafe_op_in_unsafe_fn)]

use std::os::raw::{c_char, c_int, c_uint};
use std::panic;

pub struct FfiGrid {
    inner: mux_grid::ChunkedGrid,
}

/// # Safety
/// Caller must call `grid_destroy` to free the returned pointer.
#[unsafe(no_mangle)]
pub extern "C" fn grid_create(sx: c_uint, sy: c_uint, hlimit: c_uint) -> *mut FfiGrid {
    panic::catch_unwind(|| {
        Box::into_raw(Box::new(FfiGrid {
            inner: mux_grid::ChunkedGrid::new(sx, sy, hlimit),
        }))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// # Safety
/// `ptr` must be from `grid_create` or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_destroy(ptr: *mut FfiGrid) {
    if !ptr.is_null() {
        unsafe { drop(Box::from_raw(ptr)); }
    }
}

/// # Safety
/// `ptr` must be from `grid_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_sx(ptr: *const FfiGrid) -> c_uint {
    if ptr.is_null() { return 0; }
    unsafe { (*ptr).inner.sx() }
}

/// # Safety
/// `ptr` must be from `grid_create`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_sy(ptr: *const FfiGrid) -> c_uint {
    if ptr.is_null() { return 0; }
    unsafe { (*ptr).inner.sy() }
}

/// # Safety
/// `data` must point to at least `len` valid bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_tf01_frame(data: *const u8, len: c_uint) -> c_int {
    if data.is_null() || len < 4 { return 0; }
    let slice = unsafe { std::slice::from_raw_parts(data, len as usize) };
    c_int::from(mux_proto::is_tf01(slice))
}

/// # Safety
/// Returned pointer is valid for program lifetime. Do NOT free.
#[unsafe(no_mangle)]
pub extern "C" fn termforge_version() -> *const c_char {
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
            grid_destroy(std::ptr::null_mut());
        }
    }

    #[test]
    fn ffi_tf01_detection() {
        let data = [0x54u8, 0x46, 0x4F, 0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        unsafe {
            assert_eq!(is_tf01_frame(data.as_ptr(), data.len() as c_uint), 1);
        }
    }
}
