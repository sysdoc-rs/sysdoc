//! Sandbox module for restricting sysdoc's access to system resources
//!
//! This module provides platform-specific sandboxing capabilities:
//! - On Linux 5.13+: Uses landlock (filesystem) + seccomp (syscall filtering)
//! - On other platforms: No-op implementation that logs warnings
//!
//! # Security Model
//!
//! The sandbox provides defense-in-depth by restricting:
//! 1. **Filesystem access**: Only specified paths plus /tmp are accessible
//! 2. **Network access**: All network syscalls are blocked
//! 3. **Process spawning**: exec, fork, vfork are blocked
//!
//! # Usage
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//! use sandbox::{enter_sandbox, SandboxStatus};
//!
//! let allowed_paths = vec![
//!     PathBuf::from("/path/to/input"),
//!     PathBuf::from("/path/to/output"),
//! ];
//!
//! match enter_sandbox(&allowed_paths) {
//!     Ok(status) => {
//!         println!("Sandbox active: {}", status);
//!     }
//!     Err(e) => {
//!         eprintln!("Sandbox failed: {}", e);
//!     }
//! }
//! ```

mod error;

// Platform-specific implementations
#[cfg(target_os = "linux")]
mod linux;

#[cfg(not(target_os = "linux"))]
mod noop;

// Re-export public types
pub use error::{SandboxError, SandboxStatus};

// Re-export the platform-specific implementation
#[cfg(target_os = "linux")]
pub use linux::{enter_sandbox, is_sandboxing_available};

#[cfg(not(target_os = "linux"))]
pub use noop::{enter_sandbox, is_sandboxing_available};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sandboxing_available() {
        // Just test that the function doesn't panic
        let available = is_sandboxing_available();
        println!("Sandboxing available: {}", available);

        #[cfg(target_os = "linux")]
        {
            // On Linux, we expect it might be available (depends on kernel version)
            // Don't assert, just log
            println!("Linux platform detected");
        }

        #[cfg(not(target_os = "linux"))]
        {
            // On non-Linux, it should definitely not be available
            assert!(!available);
        }
    }

    #[test]
    fn test_sandbox_status_display() {
        assert_eq!(
            format!("{}", SandboxStatus::Full),
            "Full (filesystem + syscall filtering)"
        );
        assert_eq!(
            format!("{}", SandboxStatus::FilesystemOnly),
            "Filesystem only (landlock active)"
        );
        assert_eq!(
            format!("{}", SandboxStatus::Unsupported),
            "Unsupported platform"
        );
    }

    #[test]
    fn test_sandbox_status_checks() {
        assert!(SandboxStatus::Full.is_fully_protected());
        assert!(!SandboxStatus::FilesystemOnly.is_fully_protected());
        assert!(!SandboxStatus::Unsupported.is_fully_protected());

        assert!(SandboxStatus::Full.has_any_protection());
        assert!(SandboxStatus::FilesystemOnly.has_any_protection());
        assert!(!SandboxStatus::Unsupported.has_any_protection());
        assert!(!SandboxStatus::None.has_any_protection());
    }
}
