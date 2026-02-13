//! # mux-ffi
//!
//! C FFI shim with `catch_unwind` panic recovery (INV-117).
//! This is one of two crates allowed to use `unsafe` (with mux-pty).
//!
//! ## Safety Model
//! 1. All functions check for null pointers.
//! 2. All functions use `catch_unwind` to prevent panic propagation.
//! 3. All functions return null/0 on error instead of panicking.
//! 4. Opaque struct types are used for C consumers.
//!
//! L5 surface crate.

// This crate is allowed to contain unsafe code for FFI.

use mux_grid::ChunkedGrid;

/// Opaque grid handle for C consumers.
pub struct FfiGrid {
    inner: ChunkedGrid,
}

/// Create a new grid. Returns null on failure.
///
/// # Safety
///
/// Caller must eventually call `grid_destroy` to free the returned pointer.
#[unsafe(no_mangle)]
pub extern "C" fn grid_create(sx: u32, sy: u32, hlimit: u32) -> *mut FfiGrid {
    std::panic::catch_unwind(|| {
        let grid = ChunkedGrid::new(sx as u16, sy as u16, hlimit);
        Box::into_raw(Box::new(FfiGrid { inner: grid }))
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Destroy a grid created by `grid_create`.
///
/// # Safety
///
/// `ptr` must be a valid pointer returned by `grid_create`, or null.
/// Must not be called more than once for the same pointer.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_destroy(ptr: *mut FfiGrid) {
    if !ptr.is_null() {
        let _ = std::panic::catch_unwind(|| {
            // SAFETY: caller guarantees ptr is valid and unique
            unsafe { drop(Box::from_raw(ptr)) };
        });
    }
}

/// Get grid width. Returns 0 if ptr is null.
///
/// # Safety
///
/// `ptr` must be a valid pointer returned by `grid_create`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_sx(ptr: *const FfiGrid) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    std::panic::catch_unwind(|| {
        // SAFETY: caller guarantees ptr is valid
        u32::from(unsafe { &*ptr }.inner.sx())
    })
    .unwrap_or(0)
}

/// Get grid height. Returns 0 if ptr is null.
///
/// # Safety
///
/// `ptr` must be a valid pointer returned by `grid_create`, or null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn grid_sy(ptr: *const FfiGrid) -> u32 {
    if ptr.is_null() {
        return 0;
    }
    std::panic::catch_unwind(|| {
        // SAFETY: caller guarantees ptr is valid
        u32::from(unsafe { &*ptr }.inner.sy())
    })
    .unwrap_or(0)
}

/// Detect whether data starts with a TF01 frame magic.
///
/// # Safety
///
/// `data` must point to at least `len` valid bytes, or be null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn is_tf01_frame(data: *const u8, len: u32) -> i32 {
    if data.is_null() || len < 4 {
        return 0;
    }
    std::panic::catch_unwind(|| {
        // SAFETY: caller guarantees data points to len valid bytes
        let slice = unsafe { std::slice::from_raw_parts(data, len as usize) };
        i32::from(mux_proto::is_tf01_frame(slice))
    })
    .unwrap_or(0)
}

/// Get TermForge version string.
///
/// # Safety
///
/// Returns a static string pointer. Caller must NOT free it.
#[unsafe(no_mangle)]
pub extern "C" fn termforge_version() -> *const std::ffi::c_char {
    // Static string with null terminator
    b"0.1.0\0".as_ptr().cast()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_grid_create_destroy() {
        let ptr = grid_create(80, 24, 1000);
        assert!(!ptr.is_null());
        unsafe { grid_destroy(ptr) };
    }

    #[test]
    fn ffi_grid_dimensions() {
        let ptr = grid_create(80, 24, 1000);
        assert!(!ptr.is_null());
        assert_eq!(unsafe { grid_sx(ptr) }, 80);
        assert_eq!(unsafe { grid_sy(ptr) }, 24);
        unsafe { grid_destroy(ptr) };
    }

    #[test]
    fn ffi_null_safety() {
        assert_eq!(unsafe { grid_sx(std::ptr::null()) }, 0);
        assert_eq!(unsafe { grid_sy(std::ptr::null()) }, 0);
        // grid_destroy with null is a no-op
        unsafe { grid_destroy(std::ptr::null_mut()) };
    }

    #[test]
    fn ffi_is_tf01_frame() {
        let magic = mux_proto::MAGIC.to_le_bytes();
        let result = unsafe { is_tf01_frame(magic.as_ptr(), 4) };
        assert_eq!(result, 1);
    }

    #[test]
    fn ffi_is_tf01_frame_null() {
        let result = unsafe { is_tf01_frame(std::ptr::null(), 0) };
        assert_eq!(result, 0);
    }

    #[test]
    fn ffi_version() {
        let ptr = termforge_version();
        assert!(!ptr.is_null());
    }
}
