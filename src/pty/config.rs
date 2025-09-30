// PTY configuration
// Per spec-kit/003-backend-spec.md

use std::collections::HashMap;
use std::path::PathBuf;

/// PTY configuration for spawning and managing pseudo-terminals
#[derive(Debug, Clone)]
pub struct PtyConfig {
    /// Initial terminal size (columns)
    pub cols: u16,

    /// Initial terminal size (rows)
    pub rows: u16,

    /// Working directory for the PTY process
    pub working_dir: PathBuf,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Shell configuration
    pub shell: ShellConfig,

    /// Maximum output buffer size (bytes)
    pub max_buffer_size: usize,

    /// Read timeout in milliseconds
    pub read_timeout_ms: u64,
}

impl Default for PtyConfig {
    fn default() -> Self {
        let mut env = HashMap::new();
        env.insert("TERM".to_string(), "xterm-256color".to_string());
        env.insert("COLORTERM".to_string(), "truecolor".to_string());

        Self {
            cols: 80,
            rows: 24,
            working_dir: PathBuf::from("/workspace"),
            env,
            shell: ShellConfig::default(),
            max_buffer_size: 1024 * 1024, // 1MB
            read_timeout_ms: 10,          // 10ms for real-time streaming
        }
    }
}

/// Shell configuration for PTY
#[derive(Debug, Clone)]
pub struct ShellConfig {
    /// Path to shell executable
    pub shell_path: PathBuf,

    /// Shell arguments
    pub args: Vec<String>,

    /// Whether to run shell as login shell
    pub login_shell: bool,
}

impl Default for ShellConfig {
    fn default() -> Self {
        // Default to bash as login shell
        Self {
            shell_path: PathBuf::from("/bin/bash"),
            args: vec!["--login".to_string()],
            login_shell: true,
        }
    }
}

impl ShellConfig {
    /// Create shell config for bash
    pub fn bash() -> Self {
        Self {
            shell_path: PathBuf::from("/bin/bash"),
            args: vec!["--login".to_string()],
            login_shell: true,
        }
    }

    /// Create shell config for zsh
    pub fn zsh() -> Self {
        Self {
            shell_path: PathBuf::from("/bin/zsh"),
            args: vec!["-l".to_string()],
            login_shell: true,
        }
    }

    /// Create shell config for sh
    pub fn sh() -> Self {
        Self {
            shell_path: PathBuf::from("/bin/sh"),
            args: vec![],
            login_shell: false,
        }
    }
}
