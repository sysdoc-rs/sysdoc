//! Linux sandbox implementation using landlock and seccomp

use super::error::{SandboxError, SandboxStatus};
use landlock::{Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr, RulesetStatus, ABI};
use seccompiler::{BpfProgram, SeccompAction, SeccompFilter, SeccompRule};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Enter the sandbox with filesystem and syscall restrictions
///
/// This function performs two levels of sandboxing:
/// 1. Landlock: Restricts filesystem access to specified paths plus /tmp
/// 2. Seccomp: Blocks network, exec, and fork syscalls
///
/// # Arguments
/// * `allowed_paths` - Paths that should be accessible (typically input/output directories)
///
/// # Returns
/// Status indicating what protections are active, or an error if critical setup fails
pub fn enter_sandbox(allowed_paths: &[PathBuf]) -> Result<SandboxStatus, SandboxError> {
    let mut landlock_ok = false;
    let mut seccomp_ok = false;

    // Try to set up landlock first
    match setup_landlock(allowed_paths) {
        Ok(()) => {
            log::info!("Landlock filesystem restriction active");
            landlock_ok = true;
        }
        Err(e) => {
            log::warn!("Failed to initialize landlock: {}", e);
        }
    }

    // Try to set up seccomp
    match setup_seccomp() {
        Ok(()) => {
            log::info!("Seccomp syscall filtering active");
            seccomp_ok = true;
        }
        Err(e) => {
            log::warn!("Failed to initialize seccomp: {}", e);
        }
    }

    // Return status based on what succeeded
    let status = match (landlock_ok, seccomp_ok) {
        (true, true) => SandboxStatus::Full,
        (true, false) => SandboxStatus::FilesystemOnly,
        (false, true) => SandboxStatus::NetworkExecOnly,
        (false, false) => SandboxStatus::None,
    };

    Ok(status)
}

/// Set up landlock filesystem restriction
///
/// Restricts access to only the specified paths plus /tmp
fn setup_landlock(allowed_paths: &[PathBuf]) -> Result<(), SandboxError> {
    // Try to get the best ABI version available
    let abi = ABI::V1;
    // Note: ABI versions V2 and V3 add more features but V1 is sufficient for basic filesystem restriction

    // Create a ruleset with full filesystem access rights
    let mut ruleset = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| SandboxError::LandlockError(format!("Failed to create ruleset: {}", e)))?
        .create()
        .map_err(|e| SandboxError::LandlockError(format!("Failed to create ruleset: {}", e)))?;

    // Add rules for each allowed path
    for path in allowed_paths {
        // Canonicalize the path to ensure it's absolute and resolved
        let canonical_path = path.canonicalize().map_err(|e| {
            SandboxError::LandlockError(format!(
                "Failed to canonicalize path {}: {}",
                path.display(),
                e
            ))
        })?;

        // Open the directory to get a file descriptor
        let dir_fd = std::fs::File::open(&canonical_path).map_err(|e| {
            SandboxError::LandlockError(format!(
                "Failed to open path {}: {}",
                canonical_path.display(),
                e
            ))
        })?;

        ruleset = ruleset
            .add_rule(landlock::PathBeneath::new(dir_fd, AccessFs::from_all(abi)))
            .map_err(|e| {
                SandboxError::LandlockError(format!("Failed to add rule for path: {}", e))
            })?;

        log::debug!("Added landlock rule for path: {}", path.display());
    }

    // Always allow /tmp for temporary files
    let tmp_fd = std::fs::File::open("/tmp")
        .map_err(|e| SandboxError::LandlockError(format!("Failed to open /tmp: {}", e)))?;
    ruleset = ruleset
        .add_rule(landlock::PathBeneath::new(tmp_fd, AccessFs::from_all(abi)))
        .map_err(|e| SandboxError::LandlockError(format!("Failed to add /tmp rule: {}", e)))?;

    log::debug!("Added landlock rule for /tmp");

    // Restrict the process
    let status = ruleset
        .restrict_self()
        .map_err(|e| SandboxError::LandlockError(format!("Failed to restrict self: {}", e)))?;

    // Log the status
    match status.ruleset {
        RulesetStatus::FullyEnforced => log::info!("Landlock fully enforced"),
        RulesetStatus::PartiallyEnforced => {
            log::warn!("Landlock partially enforced (some restrictions may not be active)")
        }
        RulesetStatus::NotEnforced => {
            return Err(SandboxError::LandlockError(
                "Landlock not enforced".to_string(),
            ))
        }
    }

    Ok(())
}

/// Set up seccomp syscall filtering
///
/// Blocks network, exec, and fork syscalls
fn setup_seccomp() -> Result<(), SandboxError> {
    // Create filter rules
    let mut filter_map: BTreeMap<i64, Vec<SeccompRule>> = BTreeMap::new();

    // Block network-related syscalls
    let network_syscalls = [
        libc::SYS_socket,
        libc::SYS_socketpair,
        libc::SYS_connect,
        libc::SYS_bind,
        libc::SYS_listen,
        libc::SYS_accept,
        libc::SYS_accept4,
    ];

    for syscall in &network_syscalls {
        filter_map.insert(*syscall, vec![]);
    }

    // Block exec syscalls
    let exec_syscalls = [libc::SYS_execve, libc::SYS_execveat];

    for syscall in &exec_syscalls {
        filter_map.insert(*syscall, vec![]);
    }

    // Block fork/vfork (but not clone, as threads need it)
    // Note: We're being careful with clone because Rust uses it for threading
    let fork_syscalls = [libc::SYS_fork, libc::SYS_vfork];

    for syscall in &fork_syscalls {
        filter_map.insert(*syscall, vec![]);
    }

    // Create the seccomp filter
    let filter = SeccompFilter::new(
        filter_map,
        SeccompAction::Allow,                     // Default action is to allow
        SeccompAction::Errno(libc::EPERM as u32), // Block with EPERM
        std::env::consts::ARCH.try_into().map_err(|e| {
            SandboxError::SeccompError(format!("Unsupported architecture: {:?}", e))
        })?,
    )
    .map_err(|e| SandboxError::SeccompError(format!("Failed to create filter: {}", e)))?;

    // Compile to BPF program
    let bpf_program: BpfProgram = filter
        .try_into()
        .map_err(|e| SandboxError::SeccompError(format!("Failed to compile filter: {}", e)))?;

    // Apply the filter
    seccompiler::apply_filter(&bpf_program)
        .map_err(|e| SandboxError::SeccompError(format!("Failed to apply filter: {}", e)))?;

    Ok(())
}

/// Check if sandboxing is available on this platform
pub fn is_sandboxing_available() -> bool {
    // Try to create a simple ruleset to check if landlock is available
    Ruleset::default()
        .handle_access(AccessFs::from_all(ABI::V1))
        .and_then(|r| r.create())
        .is_ok()
}
