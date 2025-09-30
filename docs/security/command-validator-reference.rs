// Reference Implementation: Command Validator
// Per Layer 3 Security Audit - Input Validation
//
// This module provides comprehensive command validation to prevent
// command injection attacks. Should be integrated into src/security/

use std::collections::HashSet;
use crate::error::{Error, Result};

/// Command validator with configurable security policies
pub struct CommandValidator {
    /// Allowed commands (whitelist). If empty, all commands are allowed (blacklist mode)
    whitelist: Option<HashSet<String>>,

    /// Blocked commands (blacklist)
    blacklist: HashSet<String>,

    /// Dangerous shell metacharacters to block
    dangerous_chars: Vec<char>,

    /// Maximum command length
    max_length: usize,

    /// Whether to allow command chaining
    allow_chaining: bool,

    /// Whether to allow redirection
    allow_redirection: bool,

    /// Whether to allow pipes
    allow_pipes: bool,
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self {
            whitelist: None, // No whitelist = accept all commands (if not blacklisted)
            blacklist: Self::default_blacklist(),
            dangerous_chars: Self::dangerous_metacharacters(),
            max_length: 4096, // 4KB max command length
            allow_chaining: false,
            allow_redirection: false,
            allow_pipes: true, // Pipes are common in terminal usage
        }
    }
}

impl CommandValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a strict validator (whitelist mode)
    pub fn strict() -> Self {
        Self {
            whitelist: Some(Self::default_whitelist()),
            blacklist: Self::default_blacklist(),
            dangerous_chars: Self::dangerous_metacharacters(),
            max_length: 4096,
            allow_chaining: false,
            allow_redirection: false,
            allow_pipes: true,
        }
    }

    /// Validate a command string
    pub fn validate(&self, cmd: &str) -> Result<ValidatedCommand> {
        // 1. Check length
        if cmd.len() > self.max_length {
            return Err(Error::InvalidCommand(
                format!("Command too long: {} bytes (max: {})", cmd.len(), self.max_length)
            ));
        }

        // 2. Check for null bytes
        if cmd.contains('\0') {
            return Err(Error::InvalidCommand(
                "Command contains null byte".to_string()
            ));
        }

        // 3. Trim whitespace
        let trimmed = cmd.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidCommand("Empty command".to_string()));
        }

        // 4. Check for command chaining
        if !self.allow_chaining {
            if trimmed.contains(';') || trimmed.contains("&&") || trimmed.contains("||") {
                return Err(Error::InvalidCommand(
                    "Command chaining not allowed (; && ||)".to_string()
                ));
            }
        }

        // 5. Check for command substitution
        if trimmed.contains("$(") || trimmed.contains('`') {
            return Err(Error::InvalidCommand(
                "Command substitution not allowed ($() or ``)".to_string()
            ));
        }

        // 6. Check for background execution
        if trimmed.ends_with('&') && !trimmed.ends_with("&&") {
            return Err(Error::InvalidCommand(
                "Background execution not allowed (&)".to_string()
            ));
        }

        // 7. Check for redirection
        if !self.allow_redirection {
            if trimmed.contains('>') || trimmed.contains('<') {
                return Err(Error::InvalidCommand(
                    "Redirection not allowed (> <)".to_string()
                ));
            }
        }

        // 8. Check for pipes
        if !self.allow_pipes && trimmed.contains('|') {
            return Err(Error::InvalidCommand(
                "Pipes not allowed (|)".to_string()
            ));
        }

        // 9. Check for other dangerous characters
        for dangerous_char in &self.dangerous_chars {
            if trimmed.contains(*dangerous_char) {
                return Err(Error::InvalidCommand(
                    format!("Dangerous character not allowed: {}", dangerous_char)
                ));
            }
        }

        // 10. Parse command using shell_words (safe parsing)
        let parts = shell_words::split(trimmed)
            .map_err(|e| Error::InvalidCommand(
                format!("Failed to parse command: {}", e)
            ))?;

        if parts.is_empty() {
            return Err(Error::InvalidCommand("Empty command after parsing".to_string()));
        }

        let program = parts[0].clone();
        let args = parts[1..].to_vec();

        // 11. Check blacklist
        if self.blacklist.contains(&program) {
            return Err(Error::InvalidCommand(
                format!("Command is blacklisted: {}", program)
            ));
        }

        // 12. Check whitelist (if enabled)
        if let Some(ref whitelist) = self.whitelist {
            if !whitelist.contains(&program) {
                return Err(Error::InvalidCommand(
                    format!("Command not in whitelist: {}", program)
                ));
            }
        }

        // 13. Validate arguments don't contain suspicious patterns
        for arg in &args {
            if arg.contains("..") {
                return Err(Error::InvalidCommand(
                    "Arguments containing '..' are not allowed".to_string()
                ));
            }
        }

        Ok(ValidatedCommand {
            original: cmd.to_string(),
            program,
            args,
        })
    }

    /// Default whitelist of safe commands
    fn default_whitelist() -> HashSet<String> {
        let safe_commands = vec![
            // File operations
            "ls", "cat", "head", "tail", "less", "more",
            "pwd", "cd", "mkdir", "rmdir", "touch",
            "cp", "mv", "rm",

            // Text processing
            "echo", "grep", "sed", "awk", "cut", "sort", "uniq", "wc",

            // System info
            "whoami", "id", "date", "uname", "hostname",
            "ps", "top", "df", "du",

            // Network (limited)
            "ping", "curl", "wget", "nc", "telnet",

            // Development
            "git", "make", "cargo", "npm", "node", "python", "python3",
            "ruby", "perl", "bash", "sh",

            // Utilities
            "find", "which", "whereis", "file", "stat",
            "tar", "gzip", "gunzip", "zip", "unzip",
        ];

        safe_commands.into_iter().map(String::from).collect()
    }

    /// Default blacklist of dangerous commands
    fn default_blacklist() -> HashSet<String> {
        let dangerous_commands = vec![
            // System modification
            "reboot", "shutdown", "halt", "poweroff", "init",

            // User/privilege management
            "su", "sudo", "passwd", "useradd", "userdel", "usermod",
            "groupadd", "groupdel", "groupmod",

            // Package management (can install backdoors)
            "apt", "apt-get", "yum", "dnf", "pacman", "zypper",

            // Filesystem dangerous operations
            "mkfs", "fdisk", "parted", "mount", "umount",

            // Kernel/system
            "insmod", "rmmod", "modprobe", "sysctl",

            // Dangerous network tools
            "nc", "netcat", "telnet", // Can establish reverse shells

            // Compiler (can compile exploits)
            "gcc", "g++", "cc", "clang",
        ];

        dangerous_commands.into_iter().map(String::from).collect()
    }

    /// Dangerous shell metacharacters
    fn dangerous_metacharacters() -> Vec<char> {
        vec![
            // Command substitution
            // '$' is allowed in variables but checked separately
            // '`' is checked separately

            // Newline/carriage return (can inject commands)
            '\n', '\r',

            // Null byte (truncation attack)
            '\0',
        ]
    }

    /// Configure the validator
    pub fn with_whitelist(mut self, whitelist: HashSet<String>) -> Self {
        self.whitelist = Some(whitelist);
        self
    }

    pub fn with_blacklist(mut self, blacklist: HashSet<String>) -> Self {
        self.blacklist = blacklist;
        self
    }

    pub fn allow_chaining(mut self) -> Self {
        self.allow_chaining = true;
        self
    }

    pub fn allow_redirection(mut self) -> Self {
        self.allow_redirection = true;
        self
    }

    pub fn deny_pipes(mut self) -> Self {
        self.allow_pipes = false;
        self
    }

    pub fn max_length(mut self, length: usize) -> Self {
        self.max_length = length;
        self
    }
}

/// Validated command ready for execution
pub struct ValidatedCommand {
    /// Original command string (for logging)
    pub original: String,

    /// Program/executable name
    pub program: String,

    /// Parsed arguments (safe - no shell interpretation)
    pub args: Vec<String>,
}

impl ValidatedCommand {
    /// Get the full command as a string (for display)
    pub fn display(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }

    /// Convert to tokio::process::Command (safe execution)
    pub fn to_tokio_command(&self) -> tokio::process::Command {
        let mut cmd = tokio::process::Command::new(&self.program);
        cmd.args(&self.args);
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_simple_command() {
        let validator = CommandValidator::new();
        let result = validator.validate("ls -la");
        assert!(result.is_ok());

        let cmd = result.unwrap();
        assert_eq!(cmd.program, "ls");
        assert_eq!(cmd.args, vec!["-la"]);
    }

    #[test]
    fn test_command_injection_blocked() {
        let validator = CommandValidator::new();

        let malicious_commands = vec![
            "ls; rm -rf /",
            "cat file && whoami",
            "echo test || cat /etc/passwd",
            "ls $(whoami)",
            "cat `whoami`",
        ];

        for cmd in malicious_commands {
            let result = validator.validate(cmd);
            assert!(result.is_err(), "Should block: {}", cmd);
        }
    }

    #[test]
    fn test_command_substitution_blocked() {
        let validator = CommandValidator::new();

        let result = validator.validate("echo $(whoami)");
        assert!(result.is_err());

        let result = validator.validate("cat `find /`");
        assert!(result.is_err());
    }

    #[test]
    fn test_background_execution_blocked() {
        let validator = CommandValidator::new();

        let result = validator.validate("nc -l 1337 &");
        assert!(result.is_err());

        // But && should be caught by chaining check
        let result = validator.validate("ls && pwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_redirection_blocked_by_default() {
        let validator = CommandValidator::new();

        let result = validator.validate("cat /etc/passwd > /tmp/stolen");
        assert!(result.is_err());

        let result = validator.validate("cat < /etc/shadow");
        assert!(result.is_err());
    }

    #[test]
    fn test_redirection_allowed_when_enabled() {
        let validator = CommandValidator::new().allow_redirection();

        let result = validator.validate("echo test > file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pipes_allowed_by_default() {
        let validator = CommandValidator::new();

        let result = validator.validate("ls | grep test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_whitelist_mode() {
        let mut whitelist = HashSet::new();
        whitelist.insert("ls".to_string());
        whitelist.insert("pwd".to_string());

        let validator = CommandValidator::new().with_whitelist(whitelist);

        // Whitelisted command should pass
        let result = validator.validate("ls -la");
        assert!(result.is_ok());

        // Non-whitelisted command should fail
        let result = validator.validate("cat /etc/passwd");
        assert!(result.is_err());
    }

    #[test]
    fn test_blacklist_mode() {
        let mut blacklist = HashSet::new();
        blacklist.insert("rm".to_string());
        blacklist.insert("sudo".to_string());

        let validator = CommandValidator::new().with_blacklist(blacklist);

        // Non-blacklisted command should pass
        let result = validator.validate("ls -la");
        assert!(result.is_ok());

        // Blacklisted command should fail
        let result = validator.validate("rm -rf /");
        assert!(result.is_err());

        let result = validator.validate("sudo su");
        assert!(result.is_err());
    }

    #[test]
    fn test_command_length_limit() {
        let validator = CommandValidator::new().max_length(100);

        // Short command should pass
        let result = validator.validate("ls");
        assert!(result.is_ok());

        // Long command should fail
        let long_cmd = "A".repeat(101);
        let result = validator.validate(&long_cmd);
        assert!(result.is_err());
    }

    #[test]
    fn test_null_byte_blocked() {
        let validator = CommandValidator::new();

        let result = validator.validate("cat /etc/passwd\0.txt");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal_in_args() {
        let validator = CommandValidator::new();

        let result = validator.validate("cat ../../etc/passwd");
        assert!(result.is_err());

        let result = validator.validate("cd ../../../root");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_command() {
        let validator = CommandValidator::new();

        let result = validator.validate("");
        assert!(result.is_err());

        let result = validator.validate("   ");
        assert!(result.is_err());
    }

    #[test]
    fn test_validated_command_display() {
        let validator = CommandValidator::new();
        let result = validator.validate("ls -la /tmp").unwrap();

        assert_eq!(result.display(), "ls -la /tmp");
    }
}

// ===== INTEGRATION EXAMPLE =====

#[cfg(test)]
mod integration_example {
    use super::*;

    /// Example: Integrating with command executor
    pub async fn execute_validated_command(
        cmd: &str,
        validator: &CommandValidator,
    ) -> Result<String> {
        // 1. Validate command
        let validated = validator.validate(cmd)?;

        tracing::info!("Executing validated command: {}", validated.display());

        // 2. Create tokio process (NO SHELL INTERPRETATION)
        let mut process = validated.to_tokio_command();

        // 3. Set up safe execution environment
        process
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        // 4. Execute
        let output = process.output().await
            .map_err(|e| Error::ProcessSpawnFailed(e.to_string()))?;

        // 5. Return output
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

// ===== USAGE EXAMPLE =====

/*
// In src/execution/executor.rs

use crate::security::CommandValidator;

pub struct CommandExecutor {
    validator: CommandValidator,
    // ... other fields
}

impl CommandExecutor {
    pub fn new() -> Self {
        Self {
            validator: CommandValidator::strict(), // or new() for permissive mode
            // ... other fields
        }
    }

    pub async fn execute(&self, cmd: &str) -> Result<CommandOutput> {
        // Validate command BEFORE execution
        let validated = self.validator.validate(cmd)?;

        // Execute safely (no shell interpretation)
        let mut process = validated.to_tokio_command();
        process.stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = process.output().await?;

        Ok(CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        })
    }
}
*/