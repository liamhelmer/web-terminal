# GitHub Actions Research: Rust CI/CD and Node.js/TypeScript Best Practices (2025)

**Research Date:** 2025-09-29
**Research Agent:** Claude Code Research Specialist
**Project:** Web-Terminal
**Status:** Complete

---

## Executive Summary

This comprehensive research report identifies the most widely supported and recommended GitHub Actions for Rust and Node.js/TypeScript projects as of 2025. Key findings emphasize migration from deprecated actions-rs tooling to actively maintained alternatives, adoption of v4 artifact actions for 10x performance improvements, and specialized Rust-specific caching solutions.

**Critical Updates for 2025:**
- actions-rs/* actions are **DEPRECATED** - migrate to dtolnay/rust-toolchain or actions-rust-lang/setup-rust-toolchain
- artifacts v3 actions **NO LONGER SUPPORTED** as of January 30, 2025 - upgrade to v4 required
- Swatinem/rust-cache v2 provides Rust-specific caching with automatic cleanup and cache corruption workarounds
- OIDC authentication recommended over long-lived secrets for cloud deployments

---

## Table of Contents

1. [Rust CI/CD Actions](#1-rust-cicd-actions)
2. [Node.js/TypeScript Frontend Actions](#2-nodejstypescript-frontend-actions)
3. [Cross-Platform Build Matrix](#3-cross-platform-build-matrix)
4. [Dependency Caching Strategies](#4-dependency-caching-strategies)
5. [Security Best Practices](#5-security-best-practices)
6. [Release and Artifacts](#6-release-and-artifacts)
7. [Recommended Workflow Examples](#7-recommended-workflow-examples)
8. [Performance Optimization](#8-performance-optimization)

---

## 1. Rust CI/CD Actions

### 1.1 Toolchain Installation (CRITICAL UPDATE)

#### ⚠️ DEPRECATED: actions-rs/toolchain

**Status:** No longer actively maintained
**Recommendation:** **DO NOT USE** - migrate to alternatives

#### ✅ RECOMMENDED: dtolnay/rust-toolchain

**Repository:** https://github.com/dtolnay/rust-toolchain
**Status:** Actively maintained by core Rust contributor
**Latest Version:** Use version tags directly (e.g., `@stable`, `@nightly`, `@1.75.0`)

**Key Features:**
- Concise and minimalist design
- Toolchain selection via action revision (e.g., `@nightly`, `@1.75.0`)
- All inputs optional
- Fast and reliable
- Zero dependencies

**Example Usage:**
```yaml
- uses: dtolnay/rust-toolchain@stable
- uses: dtolnay/rust-toolchain@nightly
- uses: dtolnay/rust-toolchain@1.75.0
```

**When to Use:**
- Projects prioritizing simplicity
- When you don't need built-in caching
- For minimal CI/CD setups

#### ✅ RECOMMENDED: actions-rust-lang/setup-rust-toolchain

**Repository:** https://github.com/actions-rust-lang/setup-rust-toolchain
**Status:** Actively maintained, extends dtolnay's approach
**Latest Version:** v1.x (check repository for latest)

**Key Features:**
- Built-in caching for Rust tools and build artifacts
- Problem matchers for cargo, clippy, and rustfmt
- Inspired by dtolnay/rust-toolchain with additional features
- Automatic cache management

**Example Usage:**
```yaml
- uses: actions-rust-lang/setup-rust-toolchain@v1
  with:
    toolchain: stable
    components: rustfmt, clippy
    cache: true
```

**When to Use:**
- Projects needing built-in caching
- When you want problem matchers for better error reporting
- For feature-rich CI/CD pipelines

### 1.2 Security Scanning

#### ✅ cargo-audit (Vulnerability Scanning)

**Repository:** https://github.com/rustsec/rustsec
**Purpose:** Audit Cargo.lock for security vulnerabilities via RustSec Advisory DB

**Recommended Actions:**

1. **actions-rust-lang/audit** (RECOMMENDED)
   - **Repository:** https://github.com/actions-rust-lang/audit
   - **Features:**
     - Creates summary with all vulnerabilities
     - Can automatically create issues for vulnerabilities
     - Configurable warning levels

   ```yaml
   - uses: actions-rust-lang/audit@v1
     with:
       denyWarnings: true
   ```

2. **actions-rs/audit-check** (Legacy but functional)
   - Still works but less feature-rich
   - Consider migrating to actions-rust-lang/audit

**Schedule Configuration:**
```yaml
on:
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight
  push:
    paths:
      - '**/Cargo.lock'
      - '**/Cargo.toml'
```

#### ✅ cargo-deny (Comprehensive Dependency Linting)

**Repository:** https://github.com/EmbarkStudios/cargo-deny
**Action:** https://github.com/EmbarkStudios/cargo-deny-action
**Purpose:** Multi-faceted dependency checking

**Check Categories:**
1. **Advisories**: Security vulnerability scanning
2. **Licenses**: License compliance verification
3. **Bans**: Enforce/deny specific crates
4. **Sources**: Verify crate sources are trusted

**Example Usage:**
```yaml
- uses: EmbarkStudios/cargo-deny-action@v2
  with:
    arguments: --all-features
```

**When to Use:**
- Enterprise environments requiring license compliance
- Projects with strict dependency policies
- When you need more than security scanning (licenses, banned crates)

**Comparison: cargo-audit vs cargo-deny**
- **cargo-audit**: Focused exclusively on security vulnerabilities
- **cargo-deny**: Broader linting including security, licenses, bans, sources
- **Recommendation**: Use cargo-deny for comprehensive checks, or cargo-audit for security-only

---

## 2. Node.js/TypeScript Frontend Actions

### 2.1 Setup and Build

#### ✅ actions/setup-node

**Status:** Official GitHub action, actively maintained
**Latest Version:** v4.x
**Repository:** https://github.com/actions/setup-node

**Example Usage:**
```yaml
- uses: actions/setup-node@v4
  with:
    node-version: '20'
    cache: 'pnpm'  # or 'npm', 'yarn'
```

**Key Features:**
- Native caching support for npm, yarn, pnpm
- Automatic version resolution
- Matrix testing support

### 2.2 Vite Build Process

**Best Practice:** Use native npm/pnpm scripts

```yaml
- name: Install dependencies
  run: pnpm install --frozen-lockfile

- name: Build with Vite
  run: pnpm run build

- name: Type check
  run: pnpm run typecheck
```

**Vite-Specific Optimizations:**
- Vite CI automatically detects CI environment (no watch mode)
- Unified configuration with Vitest for testing
- ESM, TypeScript, JSX support out-of-box via esbuild

### 2.3 Testing with Vitest

**Repository:** https://github.com/vitest-dev/vitest
**Integration:** Unified configuration with Vite

**CI Configuration:**
```yaml
- name: Run unit tests
  run: pnpm run test

- name: Run tests with coverage
  run: pnpm run test -- --coverage
```

**Key Features for CI:**
- Automatic CI detection (runs once, no watch)
- Fast execution powered by Vite
- Native TypeScript support
- Compatible with Jest snapshots

### 2.4 E2E Testing with Playwright

**Repository:** https://github.com/microsoft/playwright
**Status:** Official Microsoft project, actively maintained

**Recommended Action: playwright-github-action**

```yaml
- name: Install Playwright browsers
  run: pnpm exec playwright install --with-deps

- name: Run Playwright tests
  run: pnpm exec playwright test

- name: Upload test results
  uses: actions/upload-artifact@v4
  if: always()
  with:
    name: playwright-report
    path: playwright-report/
```

**Matrix Testing:**
```yaml
strategy:
  matrix:
    project: [chromium, firefox, webkit]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Run tests
        run: pnpm exec playwright test --project=${{ matrix.project }}
```

**Integration with Vitest:**
- Component testing via Playwright + Vitest dev server
- Write component tests in Vitest, interact via Playwright
- Unified testing approach across unit and E2E

---

## 3. Cross-Platform Build Matrix

### 3.1 Recommended Build Matrix

```yaml
strategy:
  fail-fast: false
  matrix:
    include:
      # Linux targets
      - os: ubuntu-latest
        target: x86_64-unknown-linux-gnu
        use-cross: false
      - os: ubuntu-latest
        target: aarch64-unknown-linux-gnu
        use-cross: true

      # macOS targets
      - os: macos-latest
        target: x86_64-apple-darwin
        use-cross: false
      - os: macos-latest
        target: aarch64-apple-darwin
        use-cross: false

      # Windows targets
      - os: windows-latest
        target: x86_64-pc-windows-msvc
        use-cross: false
      - os: windows-latest
        target: aarch64-pc-windows-msvc
        use-cross: false
```

### 3.2 Cross-Compilation Tools

#### Option 1: cargo-cross

**Repository:** https://github.com/cross-rs/cross
**Action:** https://github.com/houseabsolute/actions-rust-cross

**When to Use:**
- Linux builds for non-x86 targets
- Complex system dependencies
- Docker-based cross-compilation

```yaml
- uses: houseabsolute/actions-rust-cross@v0
  with:
    command: build
    target: ${{ matrix.target }}
    args: --release
```

#### Option 2: Native Compilation

**When to Use:**
- macOS universal binaries (x86_64 + aarch64)
- Windows native builds
- Linux x86_64 builds

```yaml
- name: Add target
  run: rustup target add ${{ matrix.target }}

- name: Build
  run: cargo build --release --target ${{ matrix.target }}
```

### 3.3 Platform-Specific Considerations

**Linux:**
- Include `lsb_release --short --description` in cache key
- Different Ubuntu versions may have different library versions
- musl target for static linking: `x86_64-unknown-linux-musl`

**macOS:**
- Universal binaries: `universal-apple-darwin` target
- Code signing required for distribution
- Notarization for Gatekeeper

**Windows:**
- MSVC toolchain recommended over GNU
- Static CRT linking may be desired

---

## 4. Dependency Caching Strategies

### 4.1 Rust Caching: Swatinem/rust-cache (HIGHLY RECOMMENDED)

**Repository:** https://github.com/Swatinem/rust-cache
**Status:** Actively maintained, Rust-specific optimizations
**Latest Version:** v2.x

**Why Use Over actions/cache:**
- Built on top of actions/cache with Rust-specific tweaks
- Automatic cache cleaning (removes old binaries, unused dependencies)
- Smart lockfile-aware caching (only rebuilds changed dependencies)
- Workarounds for cargo#8603 / actions/cache#403 (macOS cache corruption)
- Automatic `CARGO_INCREMENTAL=0` to avoid incremental artifact waste

**What It Caches:**
- `~/.cargo/` directory (binaries, registry, cache, git dependencies)
- `./target/` directory (dependency build artifacts)

**Example Usage:**
```yaml
- uses: dtolnay/rust-toolchain@stable
- uses: Swatinem/rust-cache@v2
  with:
    # Optional: additional cache key
    key: "v1"
    # Optional: shared cache across jobs
    shared-key: "stable"
    # Optional: cache working directory
    workspaces: "backend"
```

**Performance Impact:**
- From testing: 255 dependency project reduced from 9.5 min → 4.5 min (~50% reduction)
- Most effective for projects with `Cargo.lock` file
- Less useful for libraries (always pulling latest deps)

### 4.2 Node.js Caching

**Recommendation:** Use `actions/setup-node` built-in caching

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: '20'
    cache: 'pnpm'  # Automatically caches ~/.pnpm-store
```

**Manual Caching (if needed):**
```yaml
- uses: actions/cache@v4
  with:
    path: ~/.pnpm-store
    key: ${{ runner.os }}-pnpm-${{ hashFiles('**/pnpm-lock.yaml') }}
    restore-keys: |
      ${{ runner.os }}-pnpm-
```

### 4.3 General Caching Best Practices

**Cache Retention:**
- GitHub removes cache entries not accessed in over 7 days
- Cannot modify existing cache contents
- Create new caches with new keys for updates

**Cache Keys:**
- Include OS: `${{ runner.os }}`
- Include dependency hash: `${{ hashFiles('**/Cargo.lock') }}`
- Include compiler version if relevant
- Version your keys: `v1-`, `v2-` for invalidation

---

## 5. Security Best Practices

### 5.1 OIDC Authentication (RECOMMENDED for Cloud Deployments)

**What is OIDC?**
OpenID Connect allows GitHub Actions to authenticate with cloud providers without storing long-lived credentials.

**Benefits:**
- No long-lived secrets in GitHub
- Tokens automatically rotated
- Valid only for minutes
- Eliminates risk of stolen credentials

**Example: Azure Deployment**
```yaml
permissions:
  id-token: write
  contents: read

steps:
  - uses: azure/login@v1
    with:
      client-id: ${{ secrets.AZURE_CLIENT_ID }}
      tenant-id: ${{ secrets.AZURE_TENANT_ID }}
      subscription-id: ${{ secrets.AZURE_SUBSCRIPTION_ID }}

  - name: Deploy
    run: az webapp deploy ...
```

**When to Use:**
- AWS deployments (via aws-actions/configure-aws-credentials)
- Azure deployments
- GCP deployments
- Any cloud provider supporting OIDC

### 5.2 Secret Management

**Best Practices:**

1. **Use GitHub Environments**
   - Separate secrets by environment (dev, staging, prod)
   - Configure protection rules
   - Require approvals for production

2. **Fine-Grained Tokens**
   - Use fine-grained PATs instead of classic tokens
   - Limit permissions to minimum required
   - Set expiration dates

3. **Secret Rotation**
   - Rotate secrets regularly (quarterly minimum)
   - Use OIDC where possible to eliminate static secrets
   - Audit secret usage

4. **Never Log Secrets**
   - GitHub automatically masks registered secrets
   - Be careful with base64 encoding (can bypass masking)
   - Review logs before making workflows public

### 5.3 Security Threats (2025 Context)

**GhostAction Campaign (September 2025):**
- 327 GitHub users affected across 817 repositories
- 3,325 secrets exfiltrated (PyPI, npm, DockerHub tokens)
- Malicious workflow injection technique

**Mitigation Strategies:**
1. **Code Review Workflow Changes:**
   - Require PR approval for `.github/workflows/` changes
   - Use CODEOWNERS for workflow files
   - Enable branch protection

2. **Restrict Workflow Permissions:**
   ```yaml
   permissions:
     contents: read  # Default to read-only
   ```

3. **Use Dependency Pinning:**
   - Pin action versions to specific SHA (not tags)
   - Use Dependabot to update pinned versions
   ```yaml
   - uses: actions/checkout@8e5e7e5ab8b370d6c329ec480221332ada57f0ab  # v4.0.0
   ```

4. **Enable Security Features:**
   - Require signed commits
   - Enable secret scanning
   - Use GitHub Advanced Security if available

---

## 6. Release and Artifacts

### 6.1 Artifact Actions (CRITICAL UPDATE)

**⚠️ BREAKING CHANGE: v3 Deprecated**

As of **January 30, 2025**, v3 of `actions/upload-artifact` and `actions/download-artifact` are **NO LONGER SUPPORTED**.

**Migration Required:** Upgrade to v4

#### ✅ actions/upload-artifact@v4

**Key Improvements:**
- Up to **10x performance improvement**
- Uploads up to **90% faster** in worst-case scenarios
- Immediate availability in UI and REST API
- Returns Artifact ID after upload

**Breaking Changes:**
1. **Cannot upload to same Artifact multiple times**
   - Solution: Use different names or upload once
2. **500 Artifact limit per job**
3. **Hidden files excluded by default** (v4.4+)
   - Override with `include-hidden-files: true`

**Example Usage:**
```yaml
- uses: actions/upload-artifact@v4
  with:
    name: rust-binary-${{ matrix.target }}
    path: target/${{ matrix.target }}/release/web-terminal
    if-no-files-found: error
    retention-days: 7
```

#### ✅ actions/download-artifact@v4

```yaml
- uses: actions/download-artifact@v4
  with:
    name: rust-binary-x86_64-unknown-linux-gnu
    path: ./dist/
```

### 6.2 Release Creation

#### ✅ taiki-e/upload-rust-binary-action (HIGHLY RECOMMENDED)

**Repository:** https://github.com/taiki-e/upload-rust-binary-action
**Status:** Actively maintained (releases in 2025: v1.27.0, v1.26.0, v1.25.0)
**Purpose:** Build and upload cross-platform Rust binaries to GitHub Releases

**Key Features:**
- Automatic cross-compilation for multiple targets
- Universal macOS binaries (`universal-apple-darwin`)
- Automatic asset packaging (tar.gz, zip)
- Direct upload to GitHub Releases
- Integrates with `setup-cross-toolchain-action`

**Example Usage:**
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
          - target: aarch64-unknown-linux-gnu
          - target: x86_64-apple-darwin
          - target: aarch64-apple-darwin
          - target: universal-apple-darwin  # Universal binary
          - target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: web-terminal
          target: ${{ matrix.target }}
          tar: gzip
          zip: windows
          token: ${{ secrets.GITHUB_TOKEN }}
```

**When to Use:**
- Automated release workflow
- Cross-platform binary distribution
- When you need universal macOS binaries
- Simplify release process

#### Alternative: Manual Release Creation

```yaml
- uses: softprops/action-gh-release@v1
  if: startsWith(github.ref, 'refs/tags/')
  with:
    files: |
      target/release/web-terminal-*
      LICENSE
      README.md
    generate_release_notes: true
```

---

## 7. Recommended Workflow Examples

### 7.1 Complete Rust CI/CD Workflow

```yaml
name: Rust CI/CD

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt

      - uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt -- --check

      - name: Clippy
        run: cargo clippy -- -D warnings

      - name: Run tests
        run: cargo test --all-features

      - name: Run doc tests
        run: cargo test --doc

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions-rust-lang/audit@v1
        with:
          denyWarnings: true

  build:
    name: Build
    strategy:
      fail-fast: false
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: macos-latest
            target: x86_64-apple-darwin
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - uses: Swatinem/rust-cache@v2
        with:
          key: ${{ matrix.target }}

      - name: Build release
        run: cargo build --release --target ${{ matrix.target }}

      - uses: actions/upload-artifact@v4
        with:
          name: binary-${{ matrix.target }}
          path: target/${{ matrix.target }}/release/web-terminal${{ matrix.os == 'windows-latest' && '.exe' || '' }}
```

### 7.2 Complete Node.js/TypeScript Workflow

```yaml
name: Frontend CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Lint
        run: pnpm run lint

      - name: Type check
        run: pnpm run typecheck

      - name: Unit tests
        run: pnpm run test -- --coverage

      - name: Build
        run: pnpm run build

      - uses: actions/upload-artifact@v4
        with:
          name: dist
          path: dist/

  e2e:
    name: E2E Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'

      - name: Install dependencies
        run: pnpm install --frozen-lockfile

      - name: Install Playwright browsers
        run: pnpm exec playwright install --with-deps

      - name: Run Playwright tests
        run: pnpm exec playwright test

      - uses: actions/upload-artifact@v4
        if: always()
        with:
          name: playwright-report
          path: playwright-report/
          retention-days: 7
```

### 7.3 Combined Monorepo Workflow

```yaml
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  backend:
    name: Backend (Rust)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: backend
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: backend

      - run: cargo test --all-features
      - run: cargo build --release

  frontend:
    name: Frontend (TypeScript)
    runs-on: ubuntu-latest
    defaults:
      run:
        working-directory: frontend
    steps:
      - uses: actions/checkout@v4

      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
          cache-dependency-path: frontend/pnpm-lock.yaml

      - run: pnpm install --frozen-lockfile
      - run: pnpm run test
      - run: pnpm run build

  integration:
    name: Integration Tests
    needs: [backend, frontend]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # Start backend server
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - name: Start backend
        run: cargo run --release &
        working-directory: backend

      # Run E2E tests
      - uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'pnpm'
          cache-dependency-path: frontend/pnpm-lock.yaml

      - run: pnpm install --frozen-lockfile
        working-directory: frontend

      - run: pnpm exec playwright install --with-deps
        working-directory: frontend

      - run: pnpm exec playwright test
        working-directory: frontend
```

---

## 8. Performance Optimization

### 8.1 Build Time Optimization

**1. Aggressive Caching**
```yaml
# Cache Rust dependencies
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "stable"  # Share across jobs

# Cache Node dependencies
- uses: actions/setup-node@v4
  with:
    cache: 'pnpm'
```

**2. Dependency Pre-compilation**
```yaml
# Pre-compile dependencies in separate step
- name: Build dependencies
  run: cargo build --release --locked

- name: Build project
  run: cargo build --release
```

**3. Incremental Compilation (Controlled)**
```yaml
# Enable for development builds only
- name: Dev build
  run: cargo build
  env:
    CARGO_INCREMENTAL: 1

# Disable for release builds (Swatinem/rust-cache does this automatically)
- name: Release build
  run: cargo build --release
  env:
    CARGO_INCREMENTAL: 0
```

**4. Parallel Jobs**
```yaml
# Run tests and builds in parallel
jobs:
  test:
    # Fast feedback
  build:
    # Longer build time
  security:
    # Can run independently
```

### 8.2 Cost Optimization

**1. Conditional Job Execution**
```yaml
# Only run expensive jobs on main branch
jobs:
  expensive-test:
    if: github.ref == 'refs/heads/main'
```

**2. Path Filtering**
```yaml
on:
  push:
    paths:
      - 'backend/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
      - '.github/workflows/backend.yml'
```

**3. Artifact Retention**
```yaml
- uses: actions/upload-artifact@v4
  with:
    name: build
    path: dist/
    retention-days: 7  # Default is 90 days
```

**4. Self-Hosted Runners (Large Projects)**
- Use for frequently run workflows
- Amortize hardware cost across many builds
- Leverage persistent caching

### 8.3 Latency Optimization

**1. Use Latest Runner Images**
```yaml
runs-on: ubuntu-latest  # Always gets latest Ubuntu LTS
```

**2. Minimize Checkout Depth**
```yaml
- uses: actions/checkout@v4
  with:
    fetch-depth: 1  # Shallow clone
```

**3. Matrix Parallelization**
```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
  # All platforms build simultaneously
```

---

## 9. Web-Terminal Specific Recommendations

Based on the spec-kit analysis (000-overview.md, 009-deployment-spec.md), here are tailored recommendations:

### 9.1 Architecture Alignment

**Single-Port Constraint:**
- No special GitHub Actions considerations
- All services accessible through one port in production
- Health checks use same port (port 8080)

**Relative Paths Only:**
- Ensure workspace artifacts use relative paths
- No absolute path references in built artifacts

### 9.2 Recommended Workflow Structure

```
.github/workflows/
├── ci.yml                  # Main CI (test + build)
├── release.yml             # Release automation
├── security.yml            # Daily security scans
└── deploy.yml              # Deployment (manual trigger)
```

### 9.3 Specific Action Recommendations

**Backend (Rust):**
- ✅ dtolnay/rust-toolchain@stable
- ✅ Swatinem/rust-cache@v2
- ✅ actions-rust-lang/audit@v1 (daily cron)
- ✅ EmbarkStudios/cargo-deny-action@v2 (license compliance)

**Frontend (TypeScript/Vite):**
- ✅ actions/setup-node@v4 (Node 20)
- ✅ Built-in pnpm caching
- ✅ Vitest for unit tests
- ✅ Playwright for E2E tests

**Release:**
- ✅ taiki-e/upload-rust-binary-action@v1
- Targets: Linux (x86_64, aarch64), macOS (universal), Windows (x86_64)
- Include frontend build in release assets

**Deployment:**
- ✅ Docker multi-stage build per 009-deployment-spec.md
- ✅ Health check on single port (8080)
- ✅ Kubernetes manifest support

### 9.4 Performance Targets

Based on spec (000-overview.md success criteria):

**CI/CD Pipeline Duration:** < 5 minutes
- Achieved via:
  - Aggressive caching (Swatinem/rust-cache)
  - Parallel job execution
  - Optimized Docker layer caching

**Build Time:** < 2 minutes
- Separate dependency build from project build
- Use release profile optimizations

**Test Coverage:** > 80%
- Enforce in CI with coverage reporting
- Use codecov or similar for tracking

---

## 10. Migration Checklist

### 10.1 Immediate Actions Required

- [ ] **Replace actions-rs/toolchain** → dtolnay/rust-toolchain or actions-rust-lang/setup-rust-toolchain
- [ ] **Upgrade actions/upload-artifact** to v4
- [ ] **Upgrade actions/download-artifact** to v4
- [ ] **Upgrade actions/cache** to v4 (or use Swatinem/rust-cache@v2)
- [ ] **Review artifact retention** (reduce from 90 to 7-30 days if possible)

### 10.2 Recommended Enhancements

- [ ] **Add Swatinem/rust-cache@v2** for Rust caching
- [ ] **Add security scanning** (cargo-audit or cargo-deny)
- [ ] **Enable OIDC** for cloud deployments (eliminate static secrets)
- [ ] **Add taiki-e/upload-rust-binary-action** for release automation
- [ ] **Pin action versions** to commit SHA (security)
- [ ] **Add Dependabot** for action version updates

### 10.3 Security Hardening

- [ ] **Restrict workflow permissions** (default to read-only)
- [ ] **Enable branch protection** for `.github/workflows/`
- [ ] **Use CODEOWNERS** for workflow files
- [ ] **Enable secret scanning** (if not already enabled)
- [ ] **Rotate existing secrets** (or migrate to OIDC)
- [ ] **Review workflow logs** for sensitive data exposure

---

## 11. Reference Links

### Official Documentation
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [actions/checkout](https://github.com/actions/checkout)
- [actions/cache](https://github.com/actions/cache)
- [actions/upload-artifact](https://github.com/actions/upload-artifact)

### Rust Actions
- [dtolnay/rust-toolchain](https://github.com/dtolnay/rust-toolchain)
- [actions-rust-lang/setup-rust-toolchain](https://github.com/actions-rust-lang/setup-rust-toolchain)
- [Swatinem/rust-cache](https://github.com/Swatinem/rust-cache)
- [actions-rust-lang/audit](https://github.com/actions-rust-lang/audit)
- [EmbarkStudios/cargo-deny](https://github.com/EmbarkStudios/cargo-deny)
- [taiki-e/upload-rust-binary-action](https://github.com/taiki-e/upload-rust-binary-action)

### Node.js Actions
- [actions/setup-node](https://github.com/actions/setup-node)
- [Vitest](https://vitest.dev/)
- [Playwright](https://playwright.dev/)

### Security Resources
- [Security Hardening for GitHub Actions](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)
- [OIDC with GitHub Actions](https://docs.github.com/en/actions/deployment/security-hardening-your-deployments/about-security-hardening-with-openid-connect)
- [RustSec Advisory Database](https://rustsec.org/)

---

## 12. Conclusion

The GitHub Actions ecosystem for Rust and Node.js/TypeScript has matured significantly as of 2025, with clear migration paths away from deprecated tooling and substantial performance improvements in core actions.

**Key Takeaways:**

1. **Migration is Mandatory:** actions-rs and artifacts v3 are deprecated/unsupported
2. **Performance Gains Available:** v4 artifacts offer 10x improvement, Rust-specific caching offers 50%+ build time reduction
3. **Security is Paramount:** OIDC adoption, secret rotation, and vulnerability scanning are essential
4. **Ecosystem is Active:** Regular updates and improvements across all major actions

**For Web-Terminal Project:**
The recommended actions align well with the project's architecture (single-port deployment, Rust backend, TypeScript frontend) and performance targets (< 5 minute CI/CD, < 2 minute builds). Immediate implementation of the migration checklist will ensure a robust, performant, and secure CI/CD pipeline.

---

**Research Complete**
**Next Steps:** Implement recommended workflows in `.github/workflows/` directory per web-terminal spec-kit requirements.