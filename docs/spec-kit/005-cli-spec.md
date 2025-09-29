# Web-Terminal: CLI Interface Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

The web-terminal CLI provides command-line tools for server management, configuration, and diagnostics. This specification defines the command structure, usage patterns, and implementation details.

---

## Command Structure

```bash
web-terminal [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS] [ARGUMENTS]
```

---

## Global Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--config` | `-c` | Path to configuration file | `config.toml` |
| `--verbose` | `-v` | Enable verbose logging | `false` |
| `--quiet` | `-q` | Suppress non-error output | `false` |
| `--help` | `-h` | Display help information | N/A |
| `--version` | `-V` | Display version information | N/A |

---

## Commands

### 1. Server Management

#### `start` - Start the server

```bash
web-terminal start [OPTIONS]

OPTIONS:
  --port <PORT>         Server port (default: 8080)
  --host <HOST>         Server host (default: 0.0.0.0)
  --workers <NUM>       Number of worker threads (default: auto)
  --tls-cert <PATH>     TLS certificate file
  --tls-key <PATH>      TLS private key file
  --daemon              Run as background daemon

EXAMPLES:
  web-terminal start
  web-terminal start --port 3000
  web-terminal start --tls-cert cert.pem --tls-key key.pem
  web-terminal start --daemon
```

#### `stop` - Stop the server

```bash
web-terminal stop [OPTIONS]

OPTIONS:
  --force    Force stop without graceful shutdown
  --timeout  Graceful shutdown timeout in seconds (default: 30)

EXAMPLES:
  web-terminal stop
  web-terminal stop --force
```

#### `restart` - Restart the server

```bash
web-terminal restart [OPTIONS]

OPTIONS:
  --timeout  Graceful shutdown timeout (default: 30)

EXAMPLES:
  web-terminal restart
```

#### `status` - Check server status

```bash
web-terminal status [OPTIONS]

OPTIONS:
  --json     Output in JSON format
  --watch    Continuously watch status (Ctrl+C to exit)

EXAMPLES:
  web-terminal status
  web-terminal status --json
  web-terminal status --watch
```

---

### 2. Session Management

#### `sessions list` - List active sessions

```bash
web-terminal sessions list [OPTIONS]

OPTIONS:
  --user <USER_ID>      Filter by user ID
  --format <FORMAT>     Output format: table, json, csv (default: table)
  --sort <FIELD>        Sort by: id, user, created, activity (default: created)

EXAMPLES:
  web-terminal sessions list
  web-terminal sessions list --user alice
  web-terminal sessions list --format json
```

#### `sessions kill` - Terminate a session

```bash
web-terminal sessions kill <SESSION_ID> [OPTIONS]

OPTIONS:
  --force    Force kill without cleanup

EXAMPLES:
  web-terminal sessions kill abc123
  web-terminal sessions kill abc123 --force
```

#### `sessions cleanup` - Clean up expired sessions

```bash
web-terminal sessions cleanup [OPTIONS]

OPTIONS:
  --dry-run  Show what would be cleaned without cleaning
  --force    Clean all sessions regardless of status

EXAMPLES:
  web-terminal sessions cleanup
  web-terminal sessions cleanup --dry-run
```

---

### 3. Configuration

#### `config show` - Display configuration

```bash
web-terminal config show [OPTIONS]

OPTIONS:
  --section <NAME>    Show specific section only
  --format <FORMAT>   Output format: toml, json, yaml (default: toml)

EXAMPLES:
  web-terminal config show
  web-terminal config show --section server
  web-terminal config show --format json
```

#### `config set` - Set configuration value

```bash
web-terminal config set <KEY> <VALUE> [OPTIONS]

OPTIONS:
  --file <PATH>       Configuration file to modify (default: config.toml)

EXAMPLES:
  web-terminal config set server.port 3000
  web-terminal config set session.timeout 1800
```

#### `config validate` - Validate configuration

```bash
web-terminal config validate [OPTIONS]

OPTIONS:
  --file <PATH>       Configuration file to validate

EXAMPLES:
  web-terminal config validate
  web-terminal config validate --file prod-config.toml
```

---

### 4. User Management

#### `users list` - List users

```bash
web-terminal users list [OPTIONS]

OPTIONS:
  --format <FORMAT>   Output format: table, json, csv
  --active            Show only active users

EXAMPLES:
  web-terminal users list
  web-terminal users list --active
```

#### `users create` - Create user

```bash
web-terminal users create <USERNAME> [OPTIONS]

OPTIONS:
  --email <EMAIL>     User email address
  --admin             Grant admin privileges
  --password <PASS>   Set password (interactive if omitted)

EXAMPLES:
  web-terminal users create alice --email alice@example.com
  web-terminal users create admin --admin
```

#### `users delete` - Delete user

```bash
web-terminal users delete <USERNAME> [OPTIONS]

OPTIONS:
  --force    Skip confirmation prompt

EXAMPLES:
  web-terminal users delete alice
  web-terminal users delete alice --force
```

---

### 5. Diagnostics

#### `logs` - View server logs

```bash
web-terminal logs [OPTIONS]

OPTIONS:
  --follow, -f        Follow log output
  --tail <N>          Show last N lines (default: 100)
  --level <LEVEL>     Filter by level: error, warn, info, debug
  --session <ID>      Filter by session ID
  --json              Output in JSON format

EXAMPLES:
  web-terminal logs
  web-terminal logs --follow
  web-terminal logs --tail 500 --level error
  web-terminal logs --session abc123
```

#### `metrics` - Display metrics

```bash
web-terminal metrics [OPTIONS]

OPTIONS:
  --watch             Continuously update metrics
  --interval <SEC>    Update interval in seconds (default: 5)
  --format <FORMAT>   Output format: human, json, prometheus

EXAMPLES:
  web-terminal metrics
  web-terminal metrics --watch
  web-terminal metrics --format prometheus
```

#### `health` - Health check

```bash
web-terminal health [OPTIONS]

OPTIONS:
  --verbose           Show detailed health information
  --json              Output in JSON format

EXAMPLES:
  web-terminal health
  web-terminal health --verbose --json
```

---

## Configuration File Format

### config.toml

```toml
[server]
host = "0.0.0.0"
port = 8080
workers = 4
max_connections = 10000

[server.tls]
enabled = false
cert_file = "/path/to/cert.pem"
key_file = "/path/to/key.pem"

[session]
timeout_seconds = 1800
max_per_user = 10
workspace_quota_bytes = 1073741824  # 1GB

[security]
jwt_secret = "your-secret-key"
allowed_commands = []  # Empty = allow all
blocked_commands = ["rm -rf /", "dd if=/dev/zero"]

[limits]
max_processes_per_session = 10
max_memory_per_session_bytes = 536870912  # 512MB
max_cpu_percent = 50

[logging]
level = "info"
format = "json"
output = "stdout"

[logging.file]
enabled = true
path = "/var/log/web-terminal/app.log"
max_size_mb = 100
max_backups = 10
```

---

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `WEB_TERMINAL_CONFIG` | Path to config file | `config.toml` |
| `WEB_TERMINAL_PORT` | Server port | `8080` |
| `WEB_TERMINAL_HOST` | Server host | `0.0.0.0` |
| `WEB_TERMINAL_LOG_LEVEL` | Log level | `info` |
| `WEB_TERMINAL_JWT_SECRET` | JWT signing secret | (required) |
| `WEB_TERMINAL_TLS_CERT` | TLS certificate path | (optional) |
| `WEB_TERMINAL_TLS_KEY` | TLS key path | (optional) |

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Configuration error |
| 3 | Authentication error |
| 4 | Permission denied |
| 5 | Resource not found |
| 10 | Server already running |
| 11 | Server not running |
| 12 | Cannot connect to server |

---

## Shell Completion

### Generate Completion Scripts

```bash
# Bash
web-terminal completions bash > /etc/bash_completion.d/web-terminal

# Zsh
web-terminal completions zsh > ~/.zsh/completion/_web-terminal

# Fish
web-terminal completions fish > ~/.config/fish/completions/web-terminal.fish
```

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial CLI specification |