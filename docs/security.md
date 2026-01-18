# sysdoc Security Model

## Overview

sysdoc implements defense-in-depth security to ensure it only accesses files and resources explicitly specified by the user. This is particularly important when processing untrusted or sensitive documents, as it provides strong guarantees about what the application can and cannot do.

## Linux Sandboxing

On Linux 5.13+, sysdoc uses kernel-level sandboxing with two complementary mechanisms.

**Important:** Git metadata (version, revision history) is collected **before** sandbox initialization, since the sandbox blocks process execution. The build process order is:

1. Parse source files (read markdown, config)
2. Collect git metadata (execute git commands)
3. **Enter sandbox** (blocks network, exec, limits filesystem)
4. Transform and export (uses pre-collected git metadata)

### Landlock (Filesystem Restriction)

Landlock is a Linux kernel security module that restricts filesystem access at the kernel level. When enabled, sysdoc can only access:

- **Input directory**: The path specified via the `--input` argument (or current directory)
- **Output directory**: The path specified via the `--output` argument
- **`/tmp` directory**: For temporary files used during processing

Any attempt to access other paths will fail with `EPERM` (Permission Denied) at the system call level, even if the user running sysdoc has filesystem permissions.

**Requirements:**
- Linux kernel 5.13 or later
- Available in: Debian 12+, Ubuntu 22.04+, RHEL 9+, and equivalent distributions

### Seccomp (Syscall Filtering)

Seccomp-BPF (Secure Computing Mode with Berkeley Packet Filter) blocks dangerous system calls at the kernel level:

**Blocked syscalls:**
- **Network operations**: `socket`, `socketpair`, `connect`, `bind`, `listen`, `accept`, `accept4`
  - Prevents any network communication (inbound or outbound)
  - No data exfiltration via network

- **Process execution**: `execve`, `execveat`
  - Cannot execute external programs
  - Prevents shell command injection attacks

- **Process spawning**: `fork`, `vfork`
  - Cannot create child processes
  - Note: `clone` with thread flags is allowed for Rust's threading

Any attempt to use these syscalls will fail with `EPERM` (Operation not permitted).

**Requirements:**
- Linux kernel with `CONFIG_SECCOMP` enabled (nearly all modern distributions)

## Container Deployment

When running as a Docker container, additional protections apply at the container level:

### Docker Security Features

```yaml
# From docker-compose.yml
security_opt:
  - no-new-privileges:true    # Cannot gain additional privileges
cap_drop:
  - ALL                        # Drop all Linux capabilities
read_only: true                # Root filesystem is read-only
tmpfs:
  - /tmp:size=100M,mode=1777  # Only /tmp is writable
network_mode: none             # No network access at all
```

**Defense in Depth:**
- Even if the sysdoc sandbox is bypassed, container-level restrictions still apply
- Network isolation at both application (seccomp) and container level
- Filesystem isolation at both application (landlock) and container level

### Dev Container

The development container (`.devcontainer/`) uses `--security-opt seccomp=unconfined` to allow sysdoc's own seccomp filters to be developed and tested. This is appropriate for development but **not** for production use.

## Windows and Other Platforms

Native Windows, macOS, and other non-Linux builds do not currently have sandbox protection.

**For Windows users:**
- Use the dev container (`.devcontainer/`) for sandboxed development
- Use Docker deployment for production workloads
- Both provide Linux-based sandboxing via WSL2 or Docker Desktop

**For macOS users:**
- Use Docker Desktop for sandboxed execution
- Native macOS sandboxing (App Sandbox) may be added in future versions

## Usage

### Basic Usage

By default, sysdoc attempts to enable sandboxing but continues if it fails:

```bash
sysdoc build --input ./docs --output ./build/output.docx
```

Log output will show the sandbox status:
```
INFO  sysdoc::sandbox: Sandbox status: Full (filesystem + syscall filtering)
```

### Enforcing Sandbox

Use `--require-sandbox` to fail if full sandboxing cannot be established:

```bash
sysdoc build --input ./docs --output ./build/output.docx --require-sandbox
```

This is recommended for:
- Processing untrusted documents
- CI/CD pipelines
- Production environments
- Security-sensitive workflows

You can also set the environment variable:

```bash
export SYSDOC_REQUIRE_SANDBOX=true
sysdoc build --input ./docs --output ./build/output.docx
```

### Docker Usage

Build and run with Docker Compose:

```bash
# Place input files in ./test-input/
mkdir -p test-input test-output

# Run sysdoc in a sandboxed container
docker-compose run --rm sysdoc build --input /input --output /output/document.docx
```

Or use the Dockerfile directly:

```bash
docker build -t sysdoc .

docker run --rm \
  -v $(pwd)/docs:/input:ro \
  -v $(pwd)/build:/output \
  --network none \
  sysdoc build --input /input --output /output/document.docx
```

## Verification

### Check Sandbox Status

The sandbox status is logged during build:

```bash
sysdoc build --verbose --input ./docs --output ./output.docx
```

Look for:
```
INFO  sysdoc::sandbox: Sandbox status: Full (filesystem + syscall filtering)
```

Possible statuses:
- `Full`: Both landlock and seccomp active (best security)
- `Filesystem only`: Only landlock active
- `Network/exec blocking only`: Only seccomp active
- `None`: No sandbox active (degraded)
- `Unsupported platform`: Not on Linux

### Test Filesystem Restriction

After sandbox initialization, any access outside allowed paths should fail:

```bash
# This will fail if sandboxing is working
sysdoc build --input ./docs --output ./output.docx

# Try to read a file outside the allowed paths
# (This would need to be tested at the code level with debug/test builds)
```

### Test Network Blocking

The sandbox blocks all network syscalls. You can verify by trying to add network code:

```rust
// This will fail with EPERM after sandbox initialization
std::net::TcpStream::connect("example.com:80").unwrap();
```

## Limitations

### Current Limitations

1. **Landlock requires Linux 5.13+**
   - Older kernels will fall back to seccomp-only protection
   - Distributions: Debian 11 and Ubuntu 20.04 have older kernels

2. **Threading considerations**
   - The `clone` syscall is not blocked because Rust uses it for threading
   - With specific flags, `clone` can create processes, but this is not used by sysdoc

3. **Temporary files**
   - `/tmp` is always accessible for temporary file operations
   - Ensure sensitive data in temp files is cleaned up (sysdoc does this)

4. **Path canonicalization**
   - Paths must exist and be accessible before sandbox initialization
   - Symbolic links are resolved during canonicalization

### Known Issues

- **Output file creation**: If the output file's parent directory doesn't exist, sysdoc must create it before sandbox initialization
- **Relative paths**: Input/output paths are canonicalized to absolute paths before sandboxing

## Security Guarantees

When running with `--require-sandbox` on a supported platform:

✅ **Guaranteed:**
- Cannot read files outside input directory (except `/tmp`)
- Cannot write files outside output directory (except `/tmp`)
- Cannot create network connections
- Cannot execute external programs
- Cannot spawn child processes

❌ **Not protected against:**
- Resource exhaustion (CPU, memory, disk space in allowed paths)
- Logic bugs in sysdoc itself
- Compiler or Rust standard library vulnerabilities
- Kernel vulnerabilities (though sandboxing reduces attack surface)

## Threat Model

### In Scope

1. **Malicious input files**: Crafted Markdown, DrawIO, or CSV files attempting to:
   - Read sensitive files outside the project
   - Write files outside the output directory
   - Exfiltrate data via network
   - Execute arbitrary commands

2. **Supply chain attacks**: Compromised dependencies attempting to:
   - Access filesystem outside allowed paths
   - Establish network connections
   - Execute system commands

### Out of Scope

1. **Side-channel attacks**: Timing, cache, or speculative execution attacks
2. **Physical access**: Attacks requiring physical access to the system
3. **Social engineering**: Tricking users into running malicious commands
4. **Denial of service**: Resource exhaustion within allowed constraints

## Future Enhancements

Potential improvements being considered:

- [ ] Resource limits (CPU time, memory, disk I/O)
- [ ] macOS App Sandbox support
- [ ] Windows sandboxing (AppContainer or similar)
- [ ] Per-file granularity (only allow reading specific input files)
- [ ] Audit logging of all file access attempts
- [ ] Integration with Linux Security Modules (SELinux, AppArmor)

## References

- [Landlock documentation](https://docs.kernel.org/userspace-api/landlock.html)
- [Seccomp documentation](https://www.kernel.org/doc/html/latest/userspace-api/seccomp_filter.html)
- [Docker security](https://docs.docker.com/engine/security/)
- [Linux kernel capabilities](https://man7.org/linux/man-pages/man7/capabilities.7.html)

## Reporting Security Issues

If you discover a security vulnerability in sysdoc, please report it via:

- GitHub Security Advisories (preferred): https://github.com/[your-org]/sysdoc/security/advisories
- Email: [security email if you have one]

Please do not open public issues for security vulnerabilities.
