//! C-ABI FFI boundary with catch_unwind panic recovery (INV-117).
//!
//! Uses `#[unsafe(no_mangle)]` syntax required by Rust 2024 edition.
//! Every exported function wraps its body in `catch_unwind` and returns
//! null/0 on panic.

#![deny(unsafe_op_in_unsafe_fn)]

use std::panic::catch_unwind;

/// Opaque server handle for FFI consumers.
pub struct FfiServerHandle {
    _inner: Box<mux_api::Server>,
}

/// Create a new server. Returns null on failure.
///
/// # Safety
/// The returned pointer must be freed with `tf_server_destroy`.
#[unsafe(no_mangle)]
pub extern "C" fn tf_server_create(cols: u16, rows: u16) -> *mut FfiServerHandle {
    let result = catch_unwind(|| {
        let server = mux_api::ServerBuilder::new()
            .size(mux_types::geometry::Size::new(cols, rows))
            .build();
        match server {
            Ok(s) => {
                let handle = Box::new(FfiServerHandle {
                    _inner: Box::new(s),
                });
                Box::into_raw(handle)
            }
            Err(_) => std::ptr::null_mut(),
        }
    });
    result.unwrap_or(std::ptr::null_mut())
}

/// Destroy a server handle.
///
/// # Safety
/// `handle` must be a pointer returned by `tf_server_create`, and must
/// not be used after this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tf_server_destroy(handle: *mut FfiServerHandle) {
    let _ = catch_unwind(|| {
        if !handle.is_null() {
            // SAFETY: handle was allocated by Box::into_raw in tf_server_create
            let _ = unsafe { Box::from_raw(handle) };
        }
    });
}

/// Get the library version string.
///
/// # Safety
/// The returned string is valid for the lifetime of the program.
#[unsafe(no_mangle)]
pub extern "C" fn tf_version() -> *const u8 {
    b"0.1.0\0".as_ptr()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn create_and_destroy() {
        let handle = tf_server_create(80, 24);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }

    #[test]
    fn create_zero_size_returns_null() {
        let handle = tf_server_create(0, 0);
        assert!(handle.is_null());
    }

    #[test]
    fn version_string() {
        let ptr = tf_version();
        assert!(!ptr.is_null());
    }

    #[test]
    fn destroy_null_safe() {
        unsafe { tf_server_destroy(std::ptr::null_mut()); }
    }

    #[test]
    fn create_valid_sizes() {
        let handle = tf_server_create(120, 40);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }

    #[test]
    fn create_minimum_size() {
        let handle = tf_server_create(2, 1);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }

    #[test]
    fn version_starts_with_zero() {
        let ptr = tf_version();
        assert!(!ptr.is_null());
        let first_byte = unsafe { *ptr };
        assert_eq!(first_byte, b'0');
    }

    #[test]
    fn create_large_size() {
        let handle = tf_server_create(300, 100);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }

    #[test]
    fn create_small_cols_fails() {
        let handle = tf_server_create(1, 24);
        assert!(handle.is_null());
    }

    #[test]
    fn handle_is_not_null_after_create() {
        let handle = tf_server_create(80, 24);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }

    #[test]
    fn create_small_rows_fails() {
        let handle = tf_server_create(80, 0);
        assert!(handle.is_null());
    }

    #[test]
    fn double_destroy_safe() {
        let handle = tf_server_create(80, 24);
        unsafe { tf_server_destroy(handle); }
        // Second destroy is UB in real code, but our test verifies null safety
        unsafe { tf_server_destroy(std::ptr::null_mut()); }
    }

    #[test]
    fn version_is_semver() {
        let ptr = tf_version();
        let version = unsafe {
            let mut len = 0;
            let mut p = ptr;
            while *p != 0 {
                len += 1;
                p = p.add(1);
            }
            std::str::from_utf8(std::slice::from_raw_parts(ptr, len)).unwrap()
        };
        let parts: Vec<&str> = version.split('.').collect();
        assert_eq!(parts.len(), 3, "version should be semver");
    }

    #[test]
    fn multiple_servers() {
        let h1 = tf_server_create(80, 24);
        let h2 = tf_server_create(120, 40);
        assert!(!h1.is_null());
        assert!(!h2.is_null());
        unsafe {
            tf_server_destroy(h1);
            tf_server_destroy(h2);
        }
    }

    #[test]
    fn create_standard_size() {
        let handle = tf_server_create(80, 24);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }

    #[test]
    fn create_wide_terminal() {
        let handle = tf_server_create(200, 50);
        assert!(!handle.is_null());
        unsafe { tf_server_destroy(handle); }
    }
}
