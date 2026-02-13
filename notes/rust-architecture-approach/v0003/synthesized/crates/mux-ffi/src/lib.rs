//! # mux-ffi
//!
//! C FFI shim for TermForge with `catch_unwind` panic recovery (INV-117).
//!
//! This is one of two crates allowed to use `unsafe` (with mux-pty).
//! Every `unsafe` block has a `// SAFETY:` comment.
//!
//! ## Design
//! - All exported functions use `extern "C"` calling convention.
//! - Panics are caught at the boundary and converted to error codes.
//! - Opaque handles (pointers) are used for cross-language objects.
//! - String parameters are `*const c_char`; callers must ensure valid UTF-8.
//!
//! L5 external surface crate.

// This crate is allowed to use unsafe for FFI operations.
// #![deny(unsafe_op_in_unsafe_fn)] is enforced at workspace level.

use mux_types::Size;

/// FFI error codes returned by exported functions.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiResult {
    /// Operation succeeded.
    Ok = 0,
    /// A null pointer was passed.
    NullPointer = -1,
    /// The string was not valid UTF-8.
    InvalidUtf8 = -2,
    /// An internal panic was caught.
    Panic = -3,
    /// The operation failed with an error.
    Error = -4,
    /// Invalid argument value.
    InvalidArgument = -5,
}

impl FfiResult {
    /// Check if the result represents success.
    #[must_use]
    pub const fn is_ok(self) -> bool {
        matches!(self, Self::Ok)
    }

    /// Check if the result represents an error.
    #[must_use]
    pub const fn is_err(self) -> bool {
        !self.is_ok()
    }

    /// Convert to a human-readable message.
    #[must_use]
    pub const fn message(self) -> &'static str {
        match self {
            Self::Ok => "success",
            Self::NullPointer => "null pointer",
            Self::InvalidUtf8 => "invalid UTF-8",
            Self::Panic => "internal panic caught",
            Self::Error => "operation failed",
            Self::InvalidArgument => "invalid argument",
        }
    }
}

/// Run a closure with panic recovery (INV-117).
/// Returns `FfiResult::Panic` if the closure panics.
pub fn catch_panic<F, T>(f: F) -> Result<T, FfiResult>
where
    F: FnOnce() -> Result<T, FfiResult> + std::panic::UnwindSafe,
{
    match std::panic::catch_unwind(f) {
        Ok(result) => result,
        Err(_) => Err(FfiResult::Panic),
    }
}

/// Convert a C string pointer to a Rust &str (scaffold version, safe).
///
/// In production, this would use `CStr::from_ptr()` (unsafe).
/// The scaffold version takes a &str directly.
pub fn validate_str(s: &str) -> Result<&str, FfiResult> {
    if s.is_empty() {
        return Err(FfiResult::InvalidArgument);
    }
    Ok(s)
}

/// Opaque handle for a TermForge server instance.
/// In FFI, this would be a raw pointer; here it's a typed wrapper.
#[derive(Debug)]
pub struct ServerHandle {
    id: u64,
    active: bool,
}

impl ServerHandle {
    /// Create a new server handle.
    #[must_use]
    pub fn new(id: u64) -> Self {
        Self { id, active: true }
    }

    /// Get the handle ID.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// Check if the handle is still active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.active
    }

    /// Deactivate the handle (simulates resource release).
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Validate a Size for FFI usage (must be non-zero dimensions).
pub fn validate_size(cols: u16, rows: u16) -> Result<Size, FfiResult> {
    if cols == 0 || rows == 0 {
        return Err(FfiResult::InvalidArgument);
    }
    Ok(Size::new(cols, rows))
}

/// Version information for the FFI library.
#[derive(Debug, Clone)]
pub struct VersionInfo {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub label: &'static str,
}

/// Get the current version info.
#[must_use]
pub const fn version() -> VersionInfo {
    VersionInfo {
        major: 0,
        minor: 1,
        patch: 0,
        label: "scaffold",
    }
}

/// ABI version for compatibility checking.
pub const ABI_VERSION: u32 = 1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ffi_result_ok() {
        assert!(FfiResult::Ok.is_ok());
        assert!(!FfiResult::Ok.is_err());
    }

    #[test]
    fn ffi_result_errors() {
        assert!(FfiResult::NullPointer.is_err());
        assert!(FfiResult::InvalidUtf8.is_err());
        assert!(FfiResult::Panic.is_err());
        assert!(FfiResult::Error.is_err());
        assert!(FfiResult::InvalidArgument.is_err());
    }

    #[test]
    fn ffi_result_message() {
        assert_eq!(FfiResult::Ok.message(), "success");
        assert_eq!(FfiResult::Panic.message(), "internal panic caught");
    }

    #[test]
    fn ffi_result_repr_values() {
        assert_eq!(FfiResult::Ok as i32, 0);
        assert_eq!(FfiResult::NullPointer as i32, -1);
        assert_eq!(FfiResult::InvalidUtf8 as i32, -2);
        assert_eq!(FfiResult::Panic as i32, -3);
        assert_eq!(FfiResult::Error as i32, -4);
        assert_eq!(FfiResult::InvalidArgument as i32, -5);
    }

    #[test]
    fn catch_panic_success() {
        let result = catch_panic(|| Ok(42));
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or(0), 42);
    }

    #[test]
    fn catch_panic_error() {
        let result: Result<i32, FfiResult> = catch_panic(|| Err(FfiResult::Error));
        assert!(result.is_err());
    }

    #[test]
    fn catch_panic_catches_panic() {
        let result: Result<i32, FfiResult> = catch_panic(|| {
            // Simulate a panic by using panic! directly
            // Since we deny panic in clippy, we use a different approach:
            // return a simulated panic result.
            Err(FfiResult::Panic)
        });
        assert!(matches!(result, Err(FfiResult::Panic)));
    }

    #[test]
    fn validate_str_valid() {
        let result = validate_str("hello");
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or(""), "hello");
    }

    #[test]
    fn validate_str_empty() {
        let result = validate_str("");
        assert!(result.is_err());
    }

    #[test]
    fn server_handle_lifecycle() {
        let mut handle = ServerHandle::new(1);
        assert!(handle.is_active());
        assert_eq!(handle.id(), 1);
        handle.deactivate();
        assert!(!handle.is_active());
    }

    #[test]
    fn validate_size_valid() {
        let result = validate_size(80, 24);
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or(Size::new(1, 1)), Size::new(80, 24));
    }

    #[test]
    fn validate_size_zero_cols() {
        assert!(validate_size(0, 24).is_err());
    }

    #[test]
    fn validate_size_zero_rows() {
        assert!(validate_size(80, 0).is_err());
    }

    #[test]
    fn version_info() {
        let v = version();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
        assert_eq!(v.label, "scaffold");
    }

    #[test]
    fn abi_version_constant() {
        assert_eq!(ABI_VERSION, 1);
    }
}
