# Layer 4: Sandbox Isolation Security Audit

**Project:** web-terminal
**Version:** 0.1.0
**Date:** 2025-09-29
**Auditor:** Security Specialist (Claude)
**Audit Scope:** Layer 4 - Sandbox Isolation and Process Boundaries
**Reference:** [003-backend-spec.md](../spec-kit/003-backend-spec.md)

---

## Executive Summary

This audit evaluates the sandbox isolation mechanisms in the web-terminal project, focusing on process isolation, filesystem restrictions, resource limits, and privilege escalation vectors. The implementation provides **basic session-level isolation** with PTY per-session architecture, but **lacks comprehensive sandbox security boundaries** typically expected for multi-tenant terminal environments.

### Overall Risk Assessment: **MEDIUM-HIGH** ⚠️

**Key Findings:**
- ✅ **PTY isolation per session** is correctly implemented
- ✅ **Filesystem path validation** exists but is incomplete
- ❌ **No resource limits enforced** (CPU, memory, processes)
- ❌ **No process-level sandboxing** (chroot, namespaces, cgroups)
- ❌ **Shell runs with server privileges** (no privilege dropping)
- ❌ **Absolute path access possible** via shell commands
- ❌ **No network isolation** (explicitly out of scope)
- ⚠️ **File descriptor limits not implemented**

---

## 1. Security Boundary Analysis

### 1.1 Current Boundaries ✅

| Boundary | Status | Implementation | Notes |
|----------|--------|---------------|-------|
| **Session Isolation** | ✅ Implemented | Separate `Session` per user | Per-user session tracking with DashMap |
| **PTY per Session** | ✅ Implemented | One PTY process per session | Isolated stdin/stdout/stderr |
| **State Isolation** | ✅ Implemented | `Arc<RwLock<SessionState>>` | Memory isolation between sessions |
| **Environment Variables** | ✅ Isolated | Per-session HashMap | Sessions cannot access each other's env vars |

### 1.2 Missing Boundaries ❌

| Boundary | Status | Risk | Recommendation |
|----------|--------|------|----------------|
| **Process Isolation** | ❌ Not Implemented | HIGH | Implement Linux namespaces or containers |
| **Filesystem Jail** | ❌ Not Implemented | HIGH | Use chroot or bind mounts |
| **Resource Limits** | ❌ Not Implemented | HIGH | Implement cgroups or ulimit |
| **Network Isolation** | ❌ Out of Scope v1.0 | MEDIUM | Document as known limitation |
| **Privilege Dropping** | ❌ Not Implemented | HIGH | Drop privileges after server start |
| **File Descriptor Limits** | ❌ Not Implemented | MEDIUM | Implement per-session FD limits |

---

## 2. PTY Isolation Audit

### 2.1 PTY Process Management ✅

**File:** `src/pty/manager.rs`

**Findings:**
- ✅ Each session gets a unique PTY process via `portable-pty`
- ✅ PTY processes tracked in `DashMap<String, PtyProcessHandle>`
- ✅ Process cleanup on session termination (`kill()` and `cleanup_dead_processes()`)
- ✅ PTY resize properly isolated to individual processes

**Code Evidence:**
```rust
pub fn spawn(&self, config: Option<PtyConfig>) -> PtyResult<PtyProcessHandle> {
    let config = config.unwrap_or_else(|| self.default_config.clone());
    let handle = PtyProcess::spawn(config)?;

    // Store in registry with unique ID
    self.processes.insert(handle.id().to_string(), handle.clone());
    Ok(handle)
}
```

**Verdict:** ✅ **PASS** - PTY isolation correctly implemented.

---

## 3. Filesystem Isolation Audit

### 3.1 Workspace Path Validation ⚠️

**File:** `src/session/state.rs` (lines 172-184)

**Current Implementation:**
```rust
pub async fn update_working_dir(&self, path: PathBuf) -> Result<()> {
    let mut state = self.state.write().await;

    // Validate path is within workspace (security requirement)
    if !path.starts_with(&state.working_dir) {
        return Err(crate::error::Error::InvalidPath(
            "Path must be within workspace".to_string(),
        ));
    }

    state.working_dir = path;
    Ok(())
}
```

**Issues Identified:**

#### 3.1.1 Path Traversal Vulnerability ❌ HIGH RISK
```rust
// VULNERABLE: Session starts with workspace_root = "/workspace/user123"
// Path validation ONLY checks the stored working_dir
// But user can cd to absolute paths via shell!

// Example attack:
session.update_working_dir("/workspace/user123/mydir")  // ✅ PASS
// But user can execute: "cd /etc" or "cat /etc/passwd" via shell
```

**Root Cause:** Path validation is **application-level only**. The underlying PTY shell has **full filesystem access** with server privileges.

**Proof of Concept:**
```bash
# User connects to their session
WebSocket: {"type": "command", "data": "cd /etc"}
# ✅ Shell executes successfully (bypasses application-level check)

WebSocket: {"type": "command", "data": "cat /etc/passwd"}
# ❌ SECURITY BREACH: User can read any file the server process can access
```

**Impact:** Users can access **ANY file readable by the server process**, including:
- `/etc/passwd`, `/etc/shadow` (if server runs as root)
- Other users' workspaces at `/workspace/*`
- Server configuration files
- Application source code

#### 3.1.2 Absolute Path Access ❌ HIGH RISK

**File:** `src/pty/config.rs` (line 41)

```rust
impl Default for PtyConfig {
    fn default() -> Self {
        Self {
            working_dir: PathBuf::from("/workspace"),  // ❌ No chroot enforcement
            // ...
        }
    }
}
```

**Issue:** `working_dir` is just the **starting directory**, not a jail. The shell process can navigate anywhere.

**Verdict:** ❌ **FAIL** - Filesystem isolation is insufficient. Application-level checks do not prevent shell-level access.

---

## 4. Resource Limits Audit

### 4.1 CPU Limits ❌

**Status:** Not implemented
**Risk:** HIGH

**Current State:**
- No CPU time limits on PTY processes
- No CPU usage throttling
- No protection against CPU exhaustion attacks

**Attack Vector:**
```bash
# Malicious user spawns CPU-intensive process
WebSocket: {"type": "command", "data": ":(){ :|:& };:"}  # Fork bomb
WebSocket: {"type": "command", "data": "while true; do :; done"}  # Infinite loop
```

**Impact:** A single malicious user can consume 100% CPU, causing denial of service for all users.

**Recommendation:**
```rust
// Use Linux cgroups for CPU limits
use cgroups_rs::cgroup_builder::CgroupBuilder;
use cgroups_rs::cpu::CpuController;

let cgroup = CgroupBuilder::new("web-terminal-session")
    .cpu()
    .shares(512)  // Relative weight
    .quota(50000) // 50% of one core (50ms per 100ms period)
    .done()
    .build()?;
```

### 4.2 Memory Limits ❌

**Status:** Not implemented
**Risk:** HIGH

**Current State:**
- No memory limits on PTY processes
- No protection against memory exhaustion
- `SessionConfig.workspace_quota` defined but not enforced

**Code Evidence:**
```rust
// src/session/manager.rs
pub struct SessionConfig {
    pub workspace_quota: u64,  // ⚠️ DEFINED BUT NOT ENFORCED
    pub max_processes: usize,  // ⚠️ DEFINED BUT NOT ENFORCED
}
```

**Attack Vector:**
```bash
# Memory exhaustion attack
WebSocket: {"type": "command", "data": "yes | tr \\n x | head -c 1G > /dev/null"}
WebSocket: {"type": "command", "data": "cat /dev/zero | head -c 2G > bigfile"}
```

**Impact:** A single user can exhaust server memory, causing OOM kills or server crashes.

**Recommendation:**
```rust
// Use cgroups for memory limits
cgroup.memory()
    .limit_in_bytes(512 * 1024 * 1024)  // 512MB limit
    .oom_control(OomControl { oom_kill_disable: false })
    .done()
```

### 4.3 Process Count Limits ❌

**Status:** Not implemented
**Risk:** HIGH

**Current State:**
```rust
// src/session/manager.rs (line 28)
pub struct SessionConfig {
    pub max_processes: usize,  // ⚠️ DEFINED (default: 10) BUT NOT ENFORCED
}
```

**Issue:** Configuration exists but is never checked when spawning PTY processes.

**Attack Vector:**
```bash
# Fork bomb - unlimited process creation
WebSocket: {"type": "command", "data": ":(){ :|:& };:"}
```

**Impact:** Process table exhaustion, denial of service for entire server.

**Recommendation:**
```rust
// In PtyProcess::spawn(), enforce process limits
impl PtyProcess {
    pub fn spawn(config: PtyConfig, session: &Session) -> PtyResult<PtyProcessHandle> {
        // Check current process count
        let current_processes = session.get_processes().await.len();
        if current_processes >= config.max_processes {
            return Err(PtyError::ProcessLimitExceeded);
        }

        // Use prlimit or cgroups to enforce kernel-level limits
        // ...
    }
}
```

### 4.4 File Descriptor Limits ❌

**Status:** Not implemented
**Risk:** MEDIUM

**Current State:**
- No FD limits on PTY processes
- Inherits server's file descriptor limits

**Attack Vector:**
```bash
# File descriptor exhaustion
WebSocket: {"type": "command", "data": "for i in {1..10000}; do exec 3<>/dev/null; done"}
```

**Recommendation:** Use `setrlimit(RLIMIT_NOFILE)` per session.

**Verdict:** ❌ **FAIL** - No resource limits enforced.

---

## 5. Privilege Escalation Audit

### 5.1 Shell Privilege Level ❌ HIGH RISK

**File:** `src/pty/process.rs` (lines 142-200)

**Current Implementation:**
```rust
pub fn spawn(config: PtyConfig) -> PtyResult<PtyProcessHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(size)?;

    let mut cmd = CommandBuilder::new(&config.shell.shell_path);
    cmd.args(&config.shell.args);
    cmd.cwd(&config.working_dir);

    // ❌ No privilege dropping
    // ❌ No UID/GID change
    // ❌ Inherits server's privileges

    let child = pair.slave.spawn_command(cmd)?;
    Ok(PtyProcessHandle { ... })
}
```

**Issue:** PTY processes run with **the same privileges as the server process**.

**Attack Scenarios:**

#### Scenario 1: Server Runs as Root ❌ CRITICAL
If the server is started as root (e.g., to bind port 80):
```bash
# User's shell runs as root!
WebSocket: {"type": "command", "data": "whoami"}
# Output: root

WebSocket: {"type": "command", "data": "rm -rf /"}
# ❌ CATASTROPHIC: User has root access
```

#### Scenario 2: Server Runs as Service Account ❌ HIGH
Even if server runs as non-root user (e.g., `www-data`):
```bash
# User can access other sessions' workspaces
WebSocket: {"type": "command", "data": "ls /workspace"}
# Output: user1/ user2/ user3/ ...

WebSocket: {"type": "command", "data": "cat /workspace/user2/secrets.txt"}
# ❌ User can read other users' files
```

**Root Cause:** No UID/GID mapping or privilege isolation.

**Recommendation:**
```rust
use nix::unistd::{setuid, setgid, User, Group};

pub fn spawn(config: PtyConfig, user_id: &str) -> PtyResult<PtyProcessHandle> {
    let pair = pty_system.openpty(size)?;

    // Create dedicated user for this session
    let session_uid = create_session_user(user_id)?;
    let session_gid = create_session_group(user_id)?;

    let mut cmd = CommandBuilder::new(&config.shell.shell_path);

    // Drop privileges BEFORE executing shell
    cmd.pre_exec(move || {
        setgid(session_gid)?;
        setuid(session_uid)?;
        Ok(())
    });

    let child = pair.slave.spawn_command(cmd)?;
    Ok(PtyProcessHandle { ... })
}
```

### 5.2 Environment Variable Injection ⚠️ MEDIUM RISK

**File:** `src/pty/config.rs` (lines 34-37)

```rust
impl Default for PtyConfig {
    fn default() -> Self {
        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());
        // ❌ No sanitization of user-provided environment variables
    }
}
```

**Issue:** If user-provided environment variables are added without validation, they could inject `LD_PRELOAD`, `LD_LIBRARY_PATH`, or other dangerous variables.

**Recommendation:** Whitelist allowed environment variables.

**Verdict:** ❌ **FAIL** - Shells run with server privileges, enabling privilege escalation.

---

## 6. Process Cleanup Audit

### 6.1 Session Termination Cleanup ✅

**File:** `src/session/manager.rs` (lines 121-139)

```rust
pub async fn destroy_session(&self, session_id: &SessionId) -> Result<()> {
    if let Some((_, session)) = self.sessions.remove(session_id) {
        // Kill all processes
        session.kill_all_processes().await?;  // ✅ Cleanup invoked

        // Clean up file system
        session.cleanup_filesystem().await?;   // ✅ Cleanup invoked

        // Remove from user sessions
        if let Some(mut user_sessions) = self.user_sessions.get_mut(&session.user_id) {
            user_sessions.retain(|id| id != session_id);
        }

        tracing::info!("Destroyed session {}", session_id);
        Ok(())
    } else {
        Err(Error::SessionNotFound(session_id.to_string()))
    }
}
```

**Findings:**
- ✅ Session cleanup is invoked on destroy
- ✅ PTY processes are killed via `PtyManager::kill()`
- ⚠️ `kill_all_processes()` and `cleanup_filesystem()` are **TODO stubs** (lines 254-273)

**Code Evidence:**
```rust
// src/session/state.rs (lines 254-262)
pub async fn kill_all_processes(&self) -> Result<()> {
    let state = self.state.read().await;

    // TODO: Implement actual process killing  ❌ NOT IMPLEMENTED
    tracing::info!("Killing {} processes for session {}", state.processes.len(), self.id);

    Ok(())
}
```

**Issue:** Cleanup functions are **not yet implemented**, meaning:
- Orphaned processes may survive session termination
- Filesystem artifacts may not be cleaned up
- Resource leaks are possible

### 6.2 PTY Cleanup ✅

**File:** `src/pty/manager.rs` (lines 89-101, 178-191)

```rust
pub async fn kill(&self, id: &str) -> PtyResult<()> {
    let handle = self.remove(id)?;
    handle.kill().await?;  // ✅ Properly kills PTY process

    tracing::info!("PTY manager: killed process {} (total: {})", id, self.processes.len());
    Ok(())
}

pub async fn kill_all(&self) -> PtyResult<usize> {
    let ids: Vec<String> = self.list();
    let count = ids.len();

    for id in ids {
        if let Err(e) = self.kill(&id).await {
            tracing::error!("Failed to kill PTY process {}: {}", id, e);
        }
    }

    tracing::info!("PTY manager: killed all {} processes", count);
    Ok(count)
}
```

**Findings:**
- ✅ PTY cleanup is implemented
- ✅ Dead process cleanup exists (`cleanup_dead_processes()`)
- ✅ Orphaned PTY processes are detected and removed

**Verdict:** ⚠️ **PARTIAL PASS** - PTY cleanup works, but session-level cleanup is incomplete.

---

## 7. Integration Test Coverage

### 7.1 Test Analysis

**File:** `tests/integration/terminal_session_test.rs`

**Test Coverage:**
- ✅ Session lifecycle (create → execute → destroy)
- ✅ Session reconnection
- ✅ Multiple concurrent sessions
- ✅ Terminal resize
- ✅ Process signals (kill)
- ✅ Session resource limits (max_sessions_per_user)
- ✅ Session timeout cleanup
- ✅ Command history isolation

**Missing Tests:**
- ❌ Filesystem isolation enforcement
- ❌ Absolute path access prevention
- ❌ Resource limit enforcement (CPU, memory, processes)
- ❌ Privilege escalation prevention
- ❌ Environment variable sanitization

**Verdict:** ⚠️ **PARTIAL COVERAGE** - Tests verify session isolation but not sandbox boundaries.

---

## 8. Risk Summary

### Critical Risks (Immediate Action Required) 🔴

| Risk | Severity | Likelihood | Impact | CVSS Score |
|------|----------|-----------|---------|------------|
| **Shell runs with server privileges** | CRITICAL | HIGH | CRITICAL | 9.8 |
| **No filesystem jail (chroot)** | CRITICAL | HIGH | CRITICAL | 9.1 |
| **No resource limits (CPU, memory, processes)** | HIGH | HIGH | HIGH | 8.6 |
| **Absolute path access via shell** | HIGH | HIGH | HIGH | 8.2 |

### High Risks (Address Before Production) 🟠

| Risk | Severity | Likelihood | Impact | CVSS Score |
|------|----------|-----------|---------|------------|
| **No process-level sandboxing** | HIGH | MEDIUM | HIGH | 7.5 |
| **Cleanup functions not implemented** | MEDIUM | HIGH | MEDIUM | 6.8 |
| **No file descriptor limits** | MEDIUM | MEDIUM | MEDIUM | 5.9 |

### Medium Risks (Plan for Future Versions) 🟡

| Risk | Severity | Likelihood | Impact | CVSS Score |
|------|----------|-----------|---------|------------|
| **No network isolation** | MEDIUM | LOW | MEDIUM | 5.3 |
| **Environment variable injection** | MEDIUM | LOW | MEDIUM | 4.7 |

---

## 9. Recommendations

### 9.1 Immediate Fixes (Before v1.0 Release)

#### 1. Drop Privileges Before Shell Execution ⚠️ CRITICAL

```rust
// In src/pty/process.rs
use nix::unistd::{setuid, setgid, Uid, Gid};

impl PtyProcess {
    pub fn spawn(config: PtyConfig, session_uid: Uid, session_gid: Gid) -> PtyResult<PtyProcessHandle> {
        let pair = pty_system.openpty(size)?;

        let mut cmd = CommandBuilder::new(&config.shell.shell_path);
        cmd.args(&config.shell.args);
        cmd.cwd(&config.working_dir);

        // ✅ FIX: Drop privileges before exec
        let uid = session_uid;
        let gid = session_gid;
        cmd.pre_exec(move || {
            setgid(gid).map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
            setuid(uid).map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
            Ok(())
        });

        let child = pair.slave.spawn_command(cmd)?;
        Ok(PtyProcessHandle { ... })
    }
}
```

**Implementation Plan:**
1. Create dedicated system user per session (e.g., `web-terminal-user-<session-id>`)
2. Use `adduser` or `useradd` to create temporary users
3. Drop to this UID/GID before shell execution
4. Clean up users on session destroy

#### 2. Implement chroot Jail ⚠️ CRITICAL

```rust
// In src/pty/process.rs
use nix::unistd::chroot;

impl PtyProcess {
    pub fn spawn(config: PtyConfig) -> PtyResult<PtyProcessHandle> {
        // Create isolated workspace directory
        let jail_path = format!("/var/lib/web-terminal/jails/{}", session_id);
        std::fs::create_dir_all(&jail_path)?;

        // Set up minimal filesystem in jail
        setup_minimal_rootfs(&jail_path)?;  // /bin, /lib, /usr, /tmp, etc.

        let mut cmd = CommandBuilder::new(&config.shell.shell_path);

        let jail = jail_path.clone();
        cmd.pre_exec(move || {
            // ✅ FIX: chroot to isolated filesystem
            chroot(&jail)?;
            std::env::set_current_dir("/")?;
            Ok(())
        });

        let child = pair.slave.spawn_command(cmd)?;
        Ok(PtyProcessHandle { ... })
    }
}

fn setup_minimal_rootfs(jail_path: &str) -> Result<()> {
    // Copy essential binaries and libraries
    std::fs::create_dir_all(format!("{}/bin", jail_path))?;
    std::fs::create_dir_all(format!("{}/lib", jail_path))?;
    std::fs::create_dir_all(format!("{}/usr/bin", jail_path))?;
    std::fs::create_dir_all(format!("{}/tmp", jail_path))?;

    // Copy shell and basic utilities
    std::fs::copy("/bin/bash", format!("{}/bin/bash", jail_path))?;
    std::fs::copy("/bin/ls", format!("{}/bin/ls", jail_path))?;
    // ... copy required shared libraries

    Ok(())
}
```

**Alternative:** Use containers (Docker/Podman) or systemd-nspawn for stronger isolation.

#### 3. Enforce Resource Limits via cgroups ⚠️ CRITICAL

```rust
// Add to Cargo.toml
// [dependencies]
// cgroups-rs = "0.3"

// In src/pty/process.rs
use cgroups_rs::{Cgroup, CgroupPid, hierarchies::V2};
use cgroups_rs::cpu::CpuController;
use cgroups_rs::memory::MemController;

impl PtyProcess {
    pub fn spawn(config: PtyConfig) -> PtyResult<PtyProcessHandle> {
        // Create cgroup for this session
        let cg = Cgroup::new(hierarchies::auto(), &format!("web-terminal/{}", session_id))?;

        // ✅ FIX: Set CPU limits (50% of one core)
        let cpu_controller: &CpuController = cg.controller_of().unwrap();
        cpu_controller.set_cfs_quota(50_000)?;  // 50ms per 100ms period
        cpu_controller.set_cfs_period(100_000)?;

        // ✅ FIX: Set memory limits (512MB)
        let mem_controller: &MemController = cg.controller_of().unwrap();
        mem_controller.set_limit(512 * 1024 * 1024)?;
        mem_controller.set_soft_limit(256 * 1024 * 1024)?;

        // Spawn process
        let child = pair.slave.spawn_command(cmd)?;
        let pid = child.process_id().unwrap();

        // ✅ FIX: Add process to cgroup
        cg.add_task(CgroupPid::from(pid as u64))?;

        Ok(PtyProcessHandle { cgroup: Some(cg), ... })
    }
}
```

#### 4. Implement Process Count Limits ⚠️ HIGH

```rust
// In src/session/state.rs
impl Session {
    pub async fn can_spawn_process(&self, max_processes: usize) -> Result<bool> {
        let state = self.state.read().await;
        Ok(state.processes.len() < max_processes)
    }
}

// In src/pty/manager.rs
pub fn spawn(&self, config: Option<PtyConfig>, session: &Session) -> PtyResult<PtyProcessHandle> {
    // ✅ FIX: Check process limit before spawning
    if !session.can_spawn_process(self.default_config.max_processes).await? {
        return Err(PtyError::ProcessLimitExceeded);
    }

    let handle = PtyProcess::spawn(config)?;
    self.processes.insert(handle.id().to_string(), handle.clone());
    Ok(handle)
}
```

#### 5. Complete Cleanup Implementation ⚠️ HIGH

```rust
// In src/session/state.rs
pub async fn kill_all_processes(&self) -> Result<()> {
    let state = self.state.read().await;

    // ✅ FIX: Actually kill processes
    for (pid, _handle) in state.processes.iter() {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        if let Err(e) = kill(Pid::from_raw(*pid as i32), Signal::SIGTERM) {
            tracing::error!("Failed to kill process {}: {}", pid, e);
        }
    }

    tracing::info!("Killed {} processes for session {}", state.processes.len(), self.id);
    Ok(())
}

pub async fn cleanup_filesystem(&self) -> Result<()> {
    let state = self.state.read().await;

    // ✅ FIX: Actually clean up filesystem
    if state.working_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&state.working_dir) {
            tracing::error!("Failed to cleanup workspace {:?}: {}", state.working_dir, e);
        }
    }

    tracing::info!("Cleaned up filesystem for session {} at {:?}", self.id, state.working_dir);
    Ok(())
}
```

### 9.2 Long-Term Improvements (v1.1+)

#### 1. Container-Based Isolation

Consider using Docker/Podman containers for each session:
- Stronger isolation than chroot
- Built-in resource limits (cgroups v2)
- Network namespace isolation
- Easier to manage and audit

```rust
// Pseudo-code for container-based approach
use bollard::Docker;

pub async fn spawn_container_session(user_id: &str) -> Result<Container> {
    let docker = Docker::connect_with_socket_defaults()?;

    let config = ContainerConfig {
        image: "web-terminal-session:latest",
        env: vec![format!("USER_ID={}", user_id)],
        host_config: Some(HostConfig {
            memory: Some(512 * 1024 * 1024),  // 512MB
            cpu_quota: Some(50_000),  // 50% CPU
            network_mode: Some("none"),  // Network isolation
            ..Default::default()
        }),
        ..Default::default()
    };

    docker.create_container(None, config).await?
}
```

#### 2. Seccomp Filtering

Implement syscall filtering to block dangerous operations:

```rust
use seccomp_rs::{SeccompFilter, SeccompRule, SeccompAction};

fn apply_seccomp_filter() -> Result<()> {
    let mut filter = SeccompFilter::new(SeccompAction::Allow)?;

    // Block dangerous syscalls
    filter.add_rule(SeccompRule::new("execve", SeccompAction::Errno(libc::EPERM)))?;
    filter.add_rule(SeccompRule::new("ptrace", SeccompAction::Kill))?;
    filter.add_rule(SeccompRule::new("mount", SeccompAction::Kill))?;

    filter.load()?;
    Ok(())
}
```

#### 3. Network Isolation

Implement network namespaces or firewall rules:

```rust
use nix::sched::{unshare, CloneFlags};

cmd.pre_exec(move || {
    // Create new network namespace (requires CAP_NET_ADMIN)
    unshare(CloneFlags::CLONE_NEWNET)?;
    Ok(())
});
```

---

## 10. Compliance and Standards

### 10.1 CIS Benchmark Alignment

| Control | Status | Notes |
|---------|--------|-------|
| 5.1.1 Ensure permissions on /etc/passwd are configured | N/A | In-memory sessions |
| 5.2.1 Ensure sudo is installed | ❌ | Shell has full server privileges |
| 5.2.2 Ensure sudo commands use pty | ✅ | Using PTY |
| 5.3.1 Ensure password creation requirements are configured | N/A | No user passwords |
| 5.4.1 Ensure default user umask is configured | ⚠️ | Inherits server umask |

### 10.2 OWASP Top 10 Coverage

| OWASP Risk | Coverage | Notes |
|------------|----------|-------|
| A01:2021 – Broken Access Control | ❌ | No filesystem access control |
| A02:2021 – Cryptographic Failures | N/A | Not applicable |
| A03:2021 – Injection | ⚠️ | Command injection via shell |
| A04:2021 – Insecure Design | ❌ | Missing security boundaries |
| A05:2021 – Security Misconfiguration | ❌ | Privileged shell execution |

---

## 11. Conclusion

The web-terminal project implements **basic session-level isolation** with per-session PTYs and state management. However, it **lacks comprehensive sandbox security boundaries** required for multi-tenant production deployment.

### Critical Gaps:
1. **Shell runs with server privileges** → Users can access entire filesystem
2. **No chroot/container isolation** → Absolute path access is unrestricted
3. **No resource limits enforced** → DoS via CPU/memory/process exhaustion
4. **Privilege escalation possible** → No UID/GID dropping

### Recommendation:
**DO NOT deploy to production** without implementing:
1. Privilege dropping (UID/GID per session)
2. Filesystem jail (chroot or containers)
3. Resource limits (cgroups)
4. Process count enforcement

### Timeline:
- **Immediate (pre-v1.0):** Implement privilege dropping + resource limits
- **Short-term (v1.1):** Add chroot jails + complete cleanup implementation
- **Long-term (v2.0):** Migrate to container-based isolation

---

## 12. References

- [003-backend-spec.md](../spec-kit/003-backend-spec.md) - Backend Architecture
- [008-testing-spec.md](../spec-kit/008-testing-spec.md) - Testing Strategy
- [OWASP Container Security](https://owasp.org/www-community/vulnerabilities/Container_security)
- [Linux Namespaces Documentation](https://man7.org/linux/man-pages/man7/namespaces.7.html)
- [cgroups(7) - Linux manual page](https://man7.org/linux/man-pages/man7/cgroups.7.html)

---

**Audit Status:** ❌ FAILED
**Next Review Date:** After implementing critical fixes
**Approved for Production:** NO