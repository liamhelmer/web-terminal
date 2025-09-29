# Build and Development Guide

This document provides instructions for building, testing, and deploying the web-terminal project.

## Prerequisites

- **Rust 1.75+**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js 20+**: `brew install node` (or use nvm)
- **pnpm 8+**: `npm install -g pnpm@8.15.0`
- **cargo-watch** (for development): `cargo install cargo-watch`
- **Docker** (for containerization): `brew install docker`
- **GitHub CLI** (for CI/CD): `brew install gh`

## Quick Start

### 1. Clone and Setup

```bash
git clone https://github.com/liamhelmer/web-terminal.git
cd web-terminal

# Install dependencies
pnpm install --prefix frontend
cargo check
```

### 2. Configuration

```bash
# Copy example environment file
cp .env.example .env

# Edit .env and set required values
# Minimum required: WEB_TERMINAL_JWT_SECRET
```

### 3. Development Mode

**Option A: Use helper script (recommended)**
```bash
./scripts/dev.sh
```

**Option B: Manual (separate terminals)**
```bash
# Terminal 1: Backend
cargo watch -x run

# Terminal 2: Frontend
cd frontend && pnpm run dev
```

Access:
- Backend: http://localhost:8080
- Frontend dev server: http://localhost:3000 (proxies to backend)

## Build Commands

### Using xtask (Recommended)

```bash
# Build everything (backend + frontend)
cargo xtask build

# Build release version
cargo xtask build --release

# Run all tests
cargo xtask test

# Run CI checks (fmt, clippy, test)
cargo xtask ci

# Format code
cargo xtask fmt

# Run linters
cargo xtask lint

# Clean build artifacts
cargo xtask clean

# Build Docker image
cargo xtask docker --tag web-terminal:latest
```

### Manual Build

**Backend:**
```bash
cargo build                # Debug build
cargo build --release      # Release build (optimized)
```

**Frontend:**
```bash
cd frontend
pnpm install
pnpm run build             # Production build → dist/
```

**Combined (production):**
```bash
# Frontend build
cd frontend && pnpm run build && cd ..

# Backend build (includes frontend in release)
cargo build --release

# Binary location: target/release/web-terminal
```

## Testing

### Unit Tests

```bash
# Rust unit tests
cargo test --lib

# Frontend unit tests
cd frontend && pnpm run test
```

### Integration Tests

```bash
# Run integration tests
cargo test --test "*"
```

### E2E Tests

```bash
# Playwright E2E tests
cd frontend && pnpm run test:e2e

# E2E with UI
pnpm run test:e2e:ui

# E2E debug mode
pnpm run test:e2e:debug
```

### Coverage

```bash
# Backend coverage (requires tarpaulin)
cargo install cargo-tarpaulin
cargo tarpaulin --out Html

# Frontend coverage
cd frontend && pnpm run test:coverage
```

## Docker

### Build Image

**Single-arch (local development):**
```bash
docker build -t web-terminal:latest .
```

**Multi-arch (production):**
```bash
# Requires Docker buildx
./scripts/docker-build.sh --multi-arch --tag v1.0.0
```

### Run Container

```bash
docker run -p 8080:8080 \
  -e WEB_TERMINAL_JWT_SECRET=your-secret \
  web-terminal:latest
```

### Docker Compose

```bash
docker-compose up -d
```

## GitHub Actions CI/CD

### Setup

```bash
# Configure GitHub secrets and environments
./scripts/setup-repo-secrets.sh
```

This script:
1. Generates JWT secret
2. Stores it in GitHub repository secrets
3. Creates local `.env` file
4. Provides instructions for manual environment setup

### Manual Environment Setup

1. **Create GitHub Environments:**
   - Go to: https://github.com/YOUR_ORG/web-terminal/settings/environments
   - Create `staging` environment
   - Create `production` environment (enable required reviewers)

2. **Add Environment Secrets:**

   **Staging:**
   - `STAGING_URL`: `http://staging.example.com:8080`
   - `KUBECONFIG_STAGING`: Base64-encoded kubeconfig (if using K8s)

   **Production:**
   - `PRODUCTION_URL`: `https://web-terminal.example.com`
   - `KUBECONFIG_PRODUCTION`: Base64-encoded kubeconfig (if using K8s)

3. **Configure Branch Protection:**
   - Protect `main` branch
   - Require all CI checks to pass
   - Require pull request reviews

### Workflow Triggers

- **Push to main:** Runs CI checks
- **Pull request:** Runs CI + integration tests
- **Version tag (v*):** Triggers release workflow
- **Daily:** Security scanning

### Release Process

```bash
# Create and push version tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0

# GitHub Actions automatically:
# 1. Builds cross-platform binaries
# 2. Creates GitHub Release
# 3. Builds multi-arch Docker images
# 4. Deploys to staging
# 5. Waits for approval for production
```

## Deployment

### Kubernetes

```bash
# Deploy to staging
./scripts/deploy.sh staging v1.0.0

# Deploy to production (requires approval)
./scripts/deploy.sh production v1.0.0
```

### Docker Compose

```bash
export IMAGE_TAG=v1.0.0
docker-compose up -d
```

### Manual

```bash
# Build release binary
cargo build --release

# Run with production config
WEB_TERMINAL_CONFIG=config/production.toml \
./target/release/web-terminal start
```

## Development Workflow

### 1. Feature Development

```bash
# Create feature branch
git checkout -b feature/my-feature

# Start development mode
./scripts/dev.sh

# Make changes...

# Run tests
cargo xtask test

# Format and lint
cargo xtask fmt
cargo xtask lint

# Commit changes
git add .
git commit -m "feat: add my feature"
git push origin feature/my-feature

# Create pull request
gh pr create
```

### 2. Code Review

Pull requests trigger:
- ✅ Rust CI (tests, clippy, fmt, security)
- ✅ Frontend CI (tests, lint, typecheck)
- ✅ Integration tests
- ✅ Security scanning

All checks must pass before merge.

### 3. Release

```bash
# Update version in Cargo.toml
# Update CHANGELOG.md

# Commit version bump
git add Cargo.toml CHANGELOG.md
git commit -m "chore: bump version to v1.0.0"

# Create and push tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin main --tags

# GitHub Actions handles the rest
```

## Troubleshooting

### Build Fails: Frontend Dependencies

```bash
cd frontend
rm -rf node_modules pnpm-lock.yaml
pnpm install
```

### Build Fails: Rust Dependencies

```bash
cargo clean
rm Cargo.lock
cargo build
```

### Docker Build Fails

```bash
# Clear Docker cache
docker builder prune -a

# Rebuild
docker build --no-cache -t web-terminal:latest .
```

### Tests Fail: Port Already in Use

```bash
# Kill processes on port 8080
lsof -ti:8080 | xargs kill -9
```

## Performance Optimization

### Release Build

```bash
# Maximum optimization (slower build, faster runtime)
cargo build --release

# Profile-guided optimization (advanced)
cargo pgo build
```

### Frontend Bundle Size

```bash
cd frontend

# Analyze bundle
pnpm run build
npx vite-bundle-visualizer

# Optimize
# - Enable code splitting in vite.config.ts
# - Use dynamic imports for large dependencies
# - Enable Terser minification
```

## References

- [Spec-Kit Documentation](./spec-kit/)
- [API Documentation](./API.md)
- [Deployment Specification](./spec-kit/009-deployment-spec.md)
- [Testing Specification](./spec-kit/008-testing-spec.md)