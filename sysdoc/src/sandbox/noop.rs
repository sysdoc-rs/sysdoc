//! No-op sandbox implementation for unsupported platforms

use super::error::{SandboxError, SandboxStatus};
use std::path::PathBuf;

/// Enter the sandbox (no-op implementation)
///
/// This function does nothing on unsupported platforms and returns
/// `SandboxStatus::Unsupported` to indicate that sandboxing is not available.
pub fn enter_sandbox(_allowed_paths: &[PathBuf]) -> Result<SandboxStatus, SandboxError> {
    log::warn!("Sandboxing is not available on this platform");
    Ok(SandboxStatus::Unsupported)
}

/// Check if sandboxing is available on this platform
pub fn is_sandboxing_available() -> bool {
    false
}
