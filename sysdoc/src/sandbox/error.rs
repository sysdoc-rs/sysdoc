//! Error types for the sandbox module

use std::fmt;

/// Errors that can occur when initializing the sandbox
#[derive(Debug)]
pub enum SandboxError {
    /// Landlock initialization failed
    LandlockError(String),
    /// Seccomp initialization failed
    SeccompError(String),
    /// Platform does not support sandboxing
    UnsupportedPlatform,
}

impl fmt::Display for SandboxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LandlockError(msg) => write!(f, "Landlock error: {}", msg),
            Self::SeccompError(msg) => write!(f, "Seccomp error: {}", msg),
            Self::UnsupportedPlatform => write!(f, "Sandboxing is not supported on this platform"),
        }
    }
}

impl std::error::Error for SandboxError {}

/// Status of the sandbox initialization
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxStatus {
    /// Full protection: both filesystem restriction and syscall filtering active
    Full,
    /// Only filesystem restriction active (landlock working, seccomp failed)
    FilesystemOnly,
    /// Only network/exec blocking active (seccomp working, landlock failed)
    NetworkExecOnly,
    /// No protection active (both failed but didn't error out)
    None,
    /// Platform does not support sandboxing
    Unsupported,
}

impl SandboxStatus {
    /// Check if full protection is active
    pub fn is_fully_protected(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Check if any protection is active
    pub fn has_any_protection(self) -> bool {
        !matches!(self, Self::None | Self::Unsupported)
    }
}

impl fmt::Display for SandboxStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Full => write!(f, "Full (filesystem + syscall filtering)"),
            Self::FilesystemOnly => write!(f, "Filesystem only (landlock active)"),
            Self::NetworkExecOnly => write!(f, "Network/exec blocking only (seccomp active)"),
            Self::None => write!(f, "None (no protection active)"),
            Self::Unsupported => write!(f, "Unsupported platform"),
        }
    }
}
