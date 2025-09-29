# Web-Terminal: Documentation Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

This document specifies documentation standards, structure, and maintenance practices for the web-terminal project.

**Documentation Goal:** Provide comprehensive, accessible documentation for all user personas and use cases.

---

## Documentation Structure

```
docs/
├── spec-kit/                    # Technical specifications (this section)
│   ├── 000-overview.md
│   ├── 001-requirements.md
│   ├── 002-architecture.md
│   ├── 003-backend-spec.md
│   ├── 004-frontend-spec.md
│   ├── 005-cli-spec.md
│   ├── 006-api-spec.md
│   ├── 007-websocket-spec.md
│   ├── 008-testing-spec.md
│   ├── 009-deployment-spec.md
│   └── 010-documentation-spec.md
├── user-guide/                  # End-user documentation
│   ├── getting-started.md
│   ├── basic-usage.md
│   ├── advanced-features.md
│   ├── troubleshooting.md
│   └── faq.md
├── admin-guide/                 # Administrator documentation
│   ├── installation.md
│   ├── configuration.md
│   ├── security.md
│   ├── monitoring.md
│   └── backup-recovery.md
├── developer-guide/             # Developer documentation
│   ├── setup.md
│   ├── architecture.md
│   ├── api-reference.md
│   ├── contributing.md
│   └── code-style.md
├── api/                         # API documentation
│   ├── rest-api.md
│   ├── websocket-protocol.md
│   └── openapi.yaml
└── examples/                    # Code examples
    ├── basic-usage/
    ├── custom-commands/
    └── integrations/
```

---

## Documentation Types

### 1. User Documentation

**Audience:** End users of the terminal

**Content:**
- Getting started guide
- Basic usage instructions
- Feature documentation
- Troubleshooting
- FAQ

**Format:** Markdown files in `/docs/user-guide/`

**Example Structure:**

```markdown
# Getting Started

## Introduction

Brief overview of what web-terminal is and what it can do.

## Prerequisites

- Modern web browser (Chrome 90+, Firefox 88+, Safari 14+)
- Internet connection
- Valid user account

## First Steps

1. Navigate to the terminal URL
2. Log in with your credentials
3. Execute your first command

## Basic Commands

### Navigation

- `cd <directory>` - Change directory
- `pwd` - Print working directory
- `ls` - List files

### File Operations

- `cat <file>` - View file contents
- `touch <file>` - Create empty file
- `rm <file>` - Remove file

## Next Steps

[Link to advanced features guide]
```

---

### 2. Administrator Documentation

**Audience:** System administrators deploying and managing web-terminal

**Content:**
- Installation instructions
- Configuration reference
- Security guidelines
- Monitoring setup
- Backup procedures
- Troubleshooting

**Format:** Markdown files in `/docs/admin-guide/`

**Example Structure:**

```markdown
# Installation Guide

## System Requirements

### Hardware Requirements

- CPU: 4+ cores recommended
- RAM: 8GB minimum, 16GB recommended
- Disk: 20GB+ available space
- Network: 1Gbps recommended

### Software Requirements

- Docker 24.x or later
- Docker Compose 2.x or later
- (Optional) Kubernetes 1.28+

## Installation Methods

### Docker Installation

1. Pull the image:
   ```bash
   docker pull web-terminal:latest
   ```

2. Create configuration:
   ```bash
   mkdir -p /etc/web-terminal
   cp config.example.toml /etc/web-terminal/config.toml
   ```

3. Start the container:
   ```bash
   docker run -d \
     -p 8080:8080 \
     -v /etc/web-terminal:/app/config \
     web-terminal:latest
   ```

### Docker Compose Installation

[Detailed steps]

### Kubernetes Installation

[Detailed steps]

## Post-Installation

1. Verify health: `curl http://localhost:8080/health`
2. Create admin user: `web-terminal users create admin --admin`
3. Configure monitoring [link]
4. Set up backups [link]
```

---

### 3. Developer Documentation

**Audience:** Developers contributing to or integrating with web-terminal

**Content:**
- Development setup
- Architecture overview
- Code organization
- API reference
- Contributing guidelines
- Code style guide

**Format:** Markdown files in `/docs/developer-guide/`

**Example Structure:**

```markdown
# Development Setup

## Prerequisites

### Required Tools

- Rust 1.75+
- Node.js 20+
- pnpm 8+
- Docker 24+

### Optional Tools

- rustfmt (code formatting)
- clippy (linting)
- cargo-watch (auto-rebuild)

## Clone Repository

```bash
git clone https://github.com/example/web-terminal.git
cd web-terminal
```

## Backend Setup

```bash
# Install Rust dependencies
cargo build

# Run tests
cargo test

# Start dev server
cargo run
```

## Frontend Setup

```bash
cd frontend

# Install dependencies
pnpm install

# Start dev server
pnpm run dev
```

## Running Full Stack

```bash
# Terminal 1: Backend
cargo run

# Terminal 2: Frontend
cd frontend && pnpm run dev

# Access at http://localhost:3000
```

## Development Workflow

1. Create feature branch: `git checkout -b feature/my-feature`
2. Make changes
3. Run tests: `cargo test && pnpm test`
4. Format code: `cargo fmt && pnpm run format`
5. Lint code: `cargo clippy && pnpm run lint`
6. Commit: `git commit -m "feat: add my feature"`
7. Push and create PR
```

---

### 4. API Documentation

**Audience:** Developers integrating with the API

**Content:**
- REST API endpoints
- WebSocket protocol
- Authentication
- Request/response examples
- Error codes

**Format:**
- OpenAPI 3.0 specification (`docs/api/openapi.yaml`)
- Generated HTML documentation
- Markdown reference (`docs/api/rest-api.md`)

**Example (OpenAPI):**

```yaml
openapi: 3.0.0
info:
  title: Web-Terminal API
  version: 1.0.0
  description: REST API for web-terminal session management

servers:
  - url: http://localhost:8080/api/v1
    description: Development server

paths:
  /sessions:
    post:
      summary: Create new terminal session
      tags:
        - Sessions
      security:
        - bearerAuth: []
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateSessionRequest'
      responses:
        '201':
          description: Session created successfully
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Session'
        '401':
          $ref: '#/components/responses/Unauthorized'

components:
  schemas:
    Session:
      type: object
      properties:
        id:
          type: string
          format: uuid
        user_id:
          type: string
        created_at:
          type: string
          format: date-time
        state:
          $ref: '#/components/schemas/SessionState'

  securitySchemes:
    bearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
```

---

### 5. Code Documentation

**Audience:** Developers working on the codebase

**Content:** Inline documentation using doc comments

**Format:**
- Rust: `///` and `//!` doc comments
- TypeScript: JSDoc comments

**Example (Rust):**

```rust
/// Session manager handles lifecycle of terminal sessions.
///
/// The `SessionManager` is responsible for:
/// - Creating new sessions
/// - Tracking active sessions
/// - Enforcing session limits
/// - Cleaning up expired sessions
///
/// # Examples
///
/// ```
/// use web_terminal::SessionManager;
///
/// let manager = SessionManager::new();
/// let session = manager.create_session(user_id).await?;
/// ```
pub struct SessionManager {
    sessions: DashMap<SessionId, Arc<Session>>,
    config: SessionConfig,
}

impl SessionManager {
    /// Creates a new session for the given user.
    ///
    /// # Arguments
    ///
    /// * `user_id` - The ID of the user creating the session
    ///
    /// # Returns
    ///
    /// Returns `Ok(Session)` if successful, or `Err(Error)` if:
    /// - User has reached session limit
    /// - Workspace creation fails
    /// - Resource allocation fails
    ///
    /// # Examples
    ///
    /// ```
    /// let session = manager.create_session(UserId::new("user123")).await?;
    /// println!("Created session: {}", session.id);
    /// ```
    pub async fn create_session(&self, user_id: UserId) -> Result<Session> {
        // Implementation
    }
}
```

**Example (TypeScript):**

```typescript
/**
 * Terminal manager provides high-level terminal operations.
 *
 * @example
 * ```typescript
 * const terminal = new Terminal(container, config);
 * terminal.open();
 * terminal.write('Hello World');
 * ```
 */
export class Terminal {
  /**
   * Creates a new Terminal instance.
   *
   * @param container - HTML element to render terminal into
   * @param config - Terminal configuration options
   */
  constructor(container: HTMLElement, config: TerminalConfig) {
    // Implementation
  }

  /**
   * Writes text to the terminal.
   *
   * @param data - Text to write
   *
   * @example
   * ```typescript
   * terminal.write('Hello World\r\n');
   * terminal.write('\x1b[31mError\x1b[0m\r\n'); // Red text
   * ```
   */
  write(data: string): void {
    // Implementation
  }
}
```

---

## Documentation Standards

### Markdown Style Guide

#### Headers

```markdown
# H1 - Document Title (only one per file)
## H2 - Major Section
### H3 - Subsection
#### H4 - Minor Section
```

#### Code Blocks

Use fenced code blocks with language identifier:

```markdown
```rust
fn main() {
    println!("Hello, world!");
}
```
```

#### Links

```markdown
[Link text](https://example.com)
[Internal link](./other-doc.md)
[Section link](#section-name)
```

#### Lists

```markdown
- Unordered list item
- Another item
  - Nested item

1. Ordered list item
2. Another item
   1. Nested item
```

#### Tables

```markdown
| Column 1 | Column 2 | Column 3 |
|----------|----------|----------|
| Value 1  | Value 2  | Value 3  |
```

#### Admonitions

```markdown
> **Note:** This is a note.

> **Warning:** This is a warning.

> **Tip:** This is a tip.
```

---

### Version Control

#### Document Metadata

Every document should include metadata header:

```markdown
# Document Title

**Version:** 1.0.0
**Status:** Draft | Review | Approved | Deprecated
**Author:** Name
**Last Updated:** YYYY-MM-DD
**References:** Links to related docs

---
```

#### Version History

Include version history table at end of document:

```markdown
## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | John Doe | Initial version |
| 1.1.0 | 2025-10-15 | Jane Smith | Added section on X |
```

---

### Documentation Review Process

1. **Draft**: Author creates initial document
2. **Peer Review**: 2+ team members review
3. **Technical Review**: Technical lead reviews
4. **Approval**: Document owner approves
5. **Publication**: Merge to main branch

#### Review Checklist

- [ ] Accurate and up-to-date information
- [ ] Clear and concise writing
- [ ] Proper formatting and structure
- [ ] Working code examples
- [ ] Valid links
- [ ] Proper grammar and spelling
- [ ] Consistent terminology
- [ ] Appropriate level of detail for audience

---

## Documentation Maintenance

### Regular Updates

- **Quarterly Review**: Review all docs for accuracy
- **Release Updates**: Update docs with each release
- **Issue Tracking**: Track doc issues in GitHub
- **User Feedback**: Incorporate user feedback

### Git Workflow for Documentation

**CRITICAL: After completing any major milestone, push changes to git and verify CI/CD status.**

#### When to Commit and Push

Push documentation changes in these scenarios:

1. **Major Milestone Completion**:
   - Complete module implementation
   - Feature completion with passing tests
   - Architecture changes
   - API modifications
   - Significant refactoring

2. **Spec-Kit Updates**:
   - Requirements changes
   - Architecture decision records (ADRs)
   - API specification updates
   - Testing strategy updates
   - Deployment procedure changes

3. **Documentation Updates**:
   - User guide additions
   - API documentation updates
   - Developer guide changes
   - Example code additions

#### Git Commit Workflow

```bash
# 1. Stage all changes
git add .

# 2. Commit with descriptive message
git commit -m "feat: implement PTY spawning and management

- Add portable-pty integration
- Implement async I/O streaming
- Add process lifecycle management
- Add 7 integration tests (6 passing)
- Update spec-kit with PTY architecture"

# 3. Push to remote
git push origin main  # or current branch
```

#### GitHub Actions Validation

**MANDATORY: After pushing, verify CI/CD status:**

1. **Check Workflow Status**:
   - Navigate to repository's Actions tab
   - Verify all workflows pass (green checkmarks)
   - Review any warnings in workflow logs

2. **Common CI/CD Checks**:
   - ✅ Build: `cargo build --release`
   - ✅ Tests: `cargo test --all`
   - ✅ Linting: `cargo clippy -- -D warnings`
   - ✅ Formatting: `cargo fmt -- --check`
   - ✅ Documentation: `cargo doc --no-deps`
   - ✅ Frontend: `pnpm build && pnpm test`

3. **Fix Issues if Applicable**:
   ```bash
   # If tests fail
   cargo test  # Fix failing tests
   git commit -am "fix: resolve failing tests"
   git push

   # If formatting fails
   cargo fmt
   git commit -am "style: apply rustfmt"
   git push

   # If linting fails
   cargo clippy --fix --allow-dirty
   git commit -am "fix: resolve clippy warnings"
   git push

   # If build fails
   # Fix compilation errors
   git commit -am "fix: resolve compilation errors"
   git push
   ```

4. **Verify Success**:
   - All GitHub Actions checks show green checkmarks
   - No workflow warnings requiring attention
   - Documentation builds successfully
   - Tests pass in CI environment

#### Commit Message Guidelines

Use conventional commit format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting)
- `refactor`: Code refactoring
- `test`: Test additions or changes
- `perf`: Performance improvements
- `build`: Build system changes
- `ci`: CI/CD configuration changes
- `chore`: Maintenance tasks

**Examples:**
```bash
feat(pty): implement PTY spawning and lifecycle management
fix(websocket): resolve connection timeout issues
docs(spec-kit): update architecture with ADR-000
test(pty): add integration tests for I/O streaming
refactor(session): simplify session manager API
perf(protocol): optimize message serialization
```

#### Pre-Push Checklist

Before pushing major changes, verify:

- [ ] All tests pass locally: `cargo test --all`
- [ ] Code is formatted: `cargo fmt --check`
- [ ] No linting errors: `cargo clippy -- -D warnings`
- [ ] Documentation builds: `cargo doc --no-deps`
- [ ] Frontend builds: `pnpm build` (if applicable)
- [ ] Commit messages follow convention
- [ ] Spec-kit updated if architecture changed
- [ ] CHANGELOG.md updated (for releases)

#### Post-Push Validation

After pushing, **ALWAYS**:

1. Check GitHub Actions status within 5 minutes
2. Review workflow logs for warnings
3. Fix any CI/CD issues immediately
4. Verify deployment status if applicable
5. Update issue/PR status

#### Branch Protection

Production branches enforce:
- ✅ Required status checks must pass
- ✅ Require branches to be up to date
- ✅ Require code review approval
- ✅ No force pushes allowed
- ✅ No deletions allowed

### Deprecation Policy

When deprecating features:

1. Mark feature as deprecated in docs
2. Provide migration guide
3. Set deprecation timeline
4. Remove after grace period

Example:

```markdown
## Feature Name (Deprecated)

> **Deprecated:** This feature is deprecated as of v2.0.0 and will be removed in v3.0.0.
> Please use [New Feature](./new-feature.md) instead.
>
> [Migration Guide](./migration-guide.md)
```

---

## Documentation Tools

### Generation Tools

#### Rust Documentation

```bash
# Generate API docs
cargo doc --no-deps --open

# Generate with private items
cargo doc --no-deps --document-private-items
```

#### TypeScript Documentation

```bash
# Generate TypeDoc documentation
pnpm run docs

# Output to docs/api/typescript/
```

#### OpenAPI Documentation

```bash
# Generate HTML documentation
npx redoc-cli bundle docs/api/openapi.yaml -o docs/api/index.html

# Start interactive docs server
npx redoc-cli serve docs/api/openapi.yaml
```

### Validation Tools

```bash
# Markdown linting
npm install -g markdownlint-cli
markdownlint docs/**/*.md

# Link checking
npm install -g markdown-link-check
find docs -name "*.md" -exec markdown-link-check {} \;

# Spell checking
npm install -g cspell
cspell "docs/**/*.md"
```

---

## Documentation Hosting

### Static Site Generation

Use a static site generator for public documentation:

#### MkDocs (Recommended)

```yaml
# mkdocs.yml

site_name: Web-Terminal Documentation
theme:
  name: material
  features:
    - navigation.tabs
    - navigation.sections
    - toc.integrate
    - search.suggest

nav:
  - Home: index.md
  - User Guide:
    - Getting Started: user-guide/getting-started.md
    - Basic Usage: user-guide/basic-usage.md
  - Admin Guide:
    - Installation: admin-guide/installation.md
    - Configuration: admin-guide/configuration.md
  - Developer Guide:
    - Setup: developer-guide/setup.md
    - Architecture: developer-guide/architecture.md
  - API Reference:
    - REST API: api/rest-api.md
    - WebSocket: api/websocket-protocol.md

markdown_extensions:
  - admonition
  - codehilite
  - toc:
      permalink: true
```

```bash
# Build docs
mkdocs build

# Serve locally
mkdocs serve

# Deploy to GitHub Pages
mkdocs gh-deploy
```

---

## Documentation Metrics

Track documentation quality with metrics:

### Coverage Metrics

- **API Coverage**: % of API endpoints documented
- **Code Coverage**: % of public APIs with doc comments
- **Example Coverage**: % of features with examples

### Quality Metrics

- **Readability Score**: Flesch reading ease
- **Link Health**: % of broken links
- **Freshness**: Average age of last update
- **User Satisfaction**: Doc feedback ratings

### Tracking

```bash
# Count documented vs undocumented APIs
grep -r "/// " src/ | wc -l

# Check for broken links
find docs -name "*.md" -exec markdown-link-check {} \;

# Generate documentation coverage report
cargo doc --no-deps 2>&1 | grep "warning: missing documentation"
```

---

## Examples Repository

Maintain separate examples repository:

```
examples/
├── basic-terminal-usage/
│   ├── README.md
│   ├── src/
│   └── package.json
├── custom-commands/
│   ├── README.md
│   └── src/
├── rest-api-integration/
│   ├── README.md
│   └── src/
└── websocket-client/
    ├── README.md
    └── src/
```

Each example includes:
- README with description and instructions
- Complete working code
- Dependencies list
- Expected output

---

## Internationalization (Future)

For multi-language support:

```
docs/
├── en/          # English (default)
├── es/          # Spanish
├── fr/          # French
└── de/          # German
```

Translation guidelines:
- Maintain consistent terminology
- Translate UI strings separately
- Keep code examples in English
- Use native date/number formats

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial documentation specification |