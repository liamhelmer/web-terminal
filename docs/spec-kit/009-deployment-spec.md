# Web-Terminal: Deployment Specification

**Version:** 1.0.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29

---

## Overview

This document specifies deployment strategies, packaging requirements, and operational procedures for the web-terminal application.

---

## Table of Contents

1. [Build Process](#build-process)
2. [Packaging](#packaging)
3. [Deployment Strategies](#deployment-strategies)
4. [Configuration Management](#configuration-management)
5. [Monitoring and Logging](#monitoring-and-logging)
6. [Backup and Recovery](#backup-and-recovery)
7. [Rollback Procedures](#rollback-procedures)

---

## Build Process

### Backend Build (Rust)

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Build with specific features
cargo build --release --features "tls,metrics"

# Cross-compilation
cargo build --release --target x86_64-unknown-linux-musl
```

#### Build Optimization

```toml
# Cargo.toml

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
panic = 'abort'
```

### Frontend Build (TypeScript)

```bash
# Install dependencies
pnpm install

# Development build
pnpm run build:dev

# Production build
pnpm run build

# Build with type checking
pnpm run build && pnpm run typecheck
```

#### Build Output

```
dist/
├── index.html
├── assets/
│   ├── index-[hash].js
│   ├── index-[hash].css
│   └── vendor-[hash].js
└── favicon.ico
```

---

## Packaging

### Docker Container

#### Dockerfile

```dockerfile
# Multi-stage build for minimal image size

# Stage 1: Build Rust backend
FROM rust:1.75-alpine AS rust-builder

WORKDIR /app

# Install dependencies
RUN apk add --no-cache musl-dev openssl-dev

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Build dependencies (cached layer)
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release

# Copy source and build
COPY src ./src
RUN cargo build --release

# Stage 2: Build frontend
FROM node:20-alpine AS frontend-builder

WORKDIR /app

# Install pnpm
RUN npm install -g pnpm

# Copy manifests
COPY frontend/package.json frontend/pnpm-lock.yaml ./

# Install dependencies
RUN pnpm install --frozen-lockfile

# Copy source and build
COPY frontend/ ./
RUN pnpm run build

# Stage 3: Runtime image
FROM alpine:latest

# Install runtime dependencies
RUN apk add --no-cache ca-certificates bash

# Create non-root user
RUN addgroup -g 1000 webterminal && \
    adduser -D -s /bin/bash -u 1000 -G webterminal webterminal

# Copy binaries
COPY --from=rust-builder /app/target/release/web-terminal /usr/local/bin/
COPY --from=frontend-builder /app/dist /app/static

# Set ownership
RUN chown -R webterminal:webterminal /app

# Switch to non-root user
USER webterminal

# Expose single port for HTTP, WebSocket, and static assets
EXPOSE 8080

# Health check (uses same port as all other traffic)
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD wget --no-verbose --tries=1 --spider http://localhost:8080/health || exit 1

# Run application
CMD ["web-terminal", "start"]
```

#### Build and Push

```bash
# Build image
docker build -t web-terminal:1.0.0 .

# Tag for registry
docker tag web-terminal:1.0.0 registry.example.com/web-terminal:1.0.0
docker tag web-terminal:1.0.0 registry.example.com/web-terminal:latest

# Push to registry
docker push registry.example.com/web-terminal:1.0.0
docker push registry.example.com/web-terminal:latest
```

### Docker Compose

```yaml
# docker-compose.yml

version: '3.8'

services:
  web-terminal:
    image: web-terminal:latest
    container_name: web-terminal
    restart: unless-stopped
    # Single port for all traffic (HTTP, WebSocket, static assets)
    ports:
      - "8080:8080"
    volumes:
      - ./config:/app/config:ro
      - workspaces:/app/data/workspaces
      - logs:/app/data/logs
    environment:
      - WEB_TERMINAL_CONFIG=/app/config/config.toml
      - WEB_TERMINAL_LOG_LEVEL=info
      - WEB_TERMINAL_PORT=8080  # Single port configuration
      - RUST_BACKTRACE=1

      # Authentication (Optional - uncomment to enable)
      # - AUTH_ENABLED=true
      # - AUTH_JWKS_PROVIDERS=[{"name":"backstage","url":"https://backstage.example.com/.well-known/jwks.json","issuer":"https://backstage.example.com"}]
      # - AUTH_ALLOWED_USERS=["user:default/admin"]
      # - AUTH_ALLOWED_GROUPS=["group:default/platform-team"]
      # - AUTH_JWKS_CACHE_TTL=3600
      # - AUTH_AUDIT_LOG=true
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:8080/health"]
      interval: 30s
      timeout: 3s
      retries: 3
      start_period: 10s
    networks:
      - web-terminal-net

  # Optional: Prometheus for metrics (separate service, separate port)
  prometheus:
    image: prom/prometheus:latest
    container_name: prometheus
    restart: unless-stopped
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    networks:
      - web-terminal-net

  # Optional: Grafana for visualization
  grafana:
    image: grafana/grafana:latest
    container_name: grafana
    restart: unless-stopped
    ports:
      - "3000:3000"
    volumes:
      - grafana-data:/var/lib/grafana
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    networks:
      - web-terminal-net

volumes:
  workspaces:
  logs:
  prometheus-data:
  grafana-data:

networks:
  web-terminal-net:
    driver: bridge
```

---

## Deployment Strategies

### 1. Single Server Deployment

**Use Case:** Development, testing, small teams

```bash
# Pull latest image
docker pull web-terminal:latest

# Stop existing container
docker-compose down

# Start new container
docker-compose up -d

# Verify deployment
docker-compose ps
docker-compose logs -f web-terminal
```

### 2. Kubernetes Deployment

**Use Case:** Production, high availability, scaling

#### Deployment Manifest

```yaml
# k8s/deployment.yaml

apiVersion: apps/v1
kind: Deployment
metadata:
  name: web-terminal
  namespace: default
  labels:
    app: web-terminal
spec:
  replicas: 3
  selector:
    matchLabels:
      app: web-terminal
  template:
    metadata:
      labels:
        app: web-terminal
    spec:
      containers:
      - name: web-terminal
        image: registry.example.com/web-terminal:1.0.0
        ports:
        - containerPort: 8080
          name: http
        env:
        - name: WEB_TERMINAL_PORT
          value: "8080"
        - name: WEB_TERMINAL_LOG_LEVEL
          value: "info"

        # Authentication (Optional)
        - name: AUTH_ENABLED
          value: "true"
        - name: AUTH_JWKS_PROVIDERS
          valueFrom:
            configMapKeyRef:
              name: web-terminal-config
              key: jwks-providers
        - name: AUTH_ALLOWED_USERS
          valueFrom:
            configMapKeyRef:
              name: web-terminal-config
              key: allowed-users
        - name: AUTH_ALLOWED_GROUPS
          valueFrom:
            configMapKeyRef:
              name: web-terminal-config
              key: allowed-groups
        - name: AUTH_JWKS_CACHE_TTL
          value: "3600"
        resources:
          requests:
            memory: "256Mi"
            cpu: "250m"
          limits:
            memory: "1Gi"
            cpu: "1000m"
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 10
          timeoutSeconds: 3
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 5
          timeoutSeconds: 3
          failureThreshold: 3
        volumeMounts:
        - name: config
          mountPath: /app/config
          readOnly: true
        - name: workspaces
          mountPath: /app/data/workspaces
      volumes:
      - name: config
        configMap:
          name: web-terminal-config
      - name: workspaces
        persistentVolumeClaim:
          claimName: workspaces-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: web-terminal
  namespace: default
spec:
  selector:
    app: web-terminal
  type: LoadBalancer
  ports:
  - protocol: TCP
    port: 80
    targetPort: 8080
    name: http
---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: workspaces-pvc
spec:
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 100Gi
```

#### Deploy to Kubernetes

```bash
# Create namespace
kubectl create namespace web-terminal

# Create secrets
kubectl create secret generic web-terminal-secrets \
  --from-literal=jwt-secret=$(openssl rand -base64 32) \
  -n web-terminal

# Create config map (basic)
kubectl create configmap web-terminal-config \
  --from-file=config.toml \
  -n web-terminal

# Create config map with authentication settings
kubectl create configmap web-terminal-config \
  --from-file=config.toml \
  --from-literal=jwks-providers='[{"name":"backstage","url":"https://backstage.example.com/.well-known/jwks.json","issuer":"https://backstage.example.com"}]' \
  --from-literal=allowed-users='["user:default/admin"]' \
  --from-literal=allowed-groups='["group:default/platform-team"]' \
  -n web-terminal

# Apply deployment
kubectl apply -f k8s/deployment.yaml -n web-terminal

# Verify deployment
kubectl get pods -n web-terminal
kubectl get svc -n web-terminal

# View logs
kubectl logs -f deployment/web-terminal -n web-terminal
```

### 3. Cloud Platform Deployment

#### AWS ECS

```json
{
  "family": "web-terminal",
  "taskRoleArn": "arn:aws:iam::123456789012:role/ecsTaskRole",
  "executionRoleArn": "arn:aws:iam::123456789012:role/ecsTaskExecutionRole",
  "networkMode": "awsvpc",
  "containerDefinitions": [
    {
      "name": "web-terminal",
      "image": "123456789012.dkr.ecr.us-east-1.amazonaws.com/web-terminal:latest",
      "cpu": 512,
      "memory": 1024,
      "essential": true,
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "WEB_TERMINAL_PORT",
          "value": "8080"
        }
      ],
      "secrets": [
        {
          "name": "WEB_TERMINAL_JWT_SECRET",
          "valueFrom": "arn:aws:secretsmanager:us-east-1:123456789012:secret:jwt-secret"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/web-terminal",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
}
```

---

## Configuration Management

### Environment-Specific Configs

```
config/
├── config.toml           # Default config
├── dev.toml             # Development overrides
├── staging.toml         # Staging overrides
├── production.toml      # Production overrides
└── auth.yaml            # Authentication configuration (optional)
```

### Authentication Configuration File

Create `config/auth.yaml` for authentication settings:

```yaml
# config/auth.yaml

authentication:
  # Enable authentication (default: false)
  enabled: true

  # JWKS providers
  jwks_providers:
    - name: backstage
      url: https://backstage.example.com/.well-known/jwks.json
      issuer: https://backstage.example.com
      cache_ttl: 3600  # 1 hour

    # Example: Additional provider (Auth0)
    # - name: auth0
    #   url: https://tenant.auth0.com/.well-known/jwks.json
    #   issuer: https://tenant.auth0.com/
    #   cache_ttl: 3600

  # Authorization
  authorization:
    # Allowed users (Backstage entity format)
    allowed_users:
      - "user:default/admin"
      - "user:default/platform-ops"

    # Allowed groups (any member can access)
    allowed_groups:
      - "group:default/platform-team"
      - "group:default/sre-team"

    # Explicit deny list (takes precedence)
    deny_users: []
    deny_groups:
      - "group:default/contractors"

  # Audit logging
  audit:
    enabled: true
    log_file: /app/data/logs/auth-audit.log
    log_level: info
    include_claims: false  # Don't log full JWT claims for privacy
```

### Configuration Loading

```bash
# Development
WEB_TERMINAL_CONFIG=config/dev.toml web-terminal start

# Staging
WEB_TERMINAL_CONFIG=config/staging.toml web-terminal start

# Production
WEB_TERMINAL_CONFIG=config/production.toml web-terminal start

# With authentication config
WEB_TERMINAL_CONFIG=config/production.toml \
WEB_TERMINAL_AUTH_CONFIG=config/auth.yaml \
web-terminal start
```

### Authentication Deployment Considerations

#### JWKS Endpoint Accessibility

**CRITICAL:** The JWKS endpoint must be accessible from the web-terminal server:

```bash
# Test JWKS endpoint accessibility from your deployment environment
curl -v https://backstage.example.com/.well-known/jwks.json

# Expected response: HTTP 200 with JSON containing "keys" array
# If this fails, authentication will not work
```

**Common Issues:**
- **DNS Resolution**: Ensure the server can resolve the JWKS hostname
- **Firewall Rules**: JWKS endpoint must be reachable from the container/pod
- **TLS Certificates**: Server must trust the JWKS endpoint's TLS certificate
- **Network Policies**: Kubernetes NetworkPolicies must allow egress to JWKS endpoint

**Docker Considerations:**
- JWKS URL must be accessible from inside the container
- Use host network if JWKS is on localhost: `docker run --network=host`
- For Docker Compose, ensure network connectivity between services

**Kubernetes Considerations:**
- Add NetworkPolicy allowing egress to JWKS endpoint
- Ensure CoreDNS can resolve external domains
- Configure proxy if required for external access

#### Environment Variables for Authentication

```bash
# Enable authentication
export AUTH_ENABLED=true

# JWKS providers (JSON format)
export AUTH_JWKS_PROVIDERS='[
  {
    "name": "backstage",
    "url": "https://backstage.example.com/.well-known/jwks.json",
    "issuer": "https://backstage.example.com"
  }
]'

# Allowed users (Backstage entity format)
export AUTH_ALLOWED_USERS='["user:default/admin","user:default/platform-ops"]'

# Allowed groups (any member can access)
export AUTH_ALLOWED_GROUPS='["group:default/platform-team","group:default/sre-team"]'

# JWKS cache TTL (seconds, default: 3600)
export AUTH_JWKS_CACHE_TTL=3600

# Enable audit logging (default: true if AUTH_ENABLED=true)
export AUTH_AUDIT_LOG=true
export AUTH_AUDIT_LOG_FILE=/app/data/logs/auth-audit.log
```

### Secrets Management

#### Using Environment Variables

```bash
# No shared secrets required for JWKS authentication
# JWKS uses public key cryptography (no secrets to manage)

# Example: Generate random secret if needed for other purposes
export WEB_TERMINAL_SESSION_SECRET=$(openssl rand -base64 32)
```

#### Using HashiCorp Vault

```bash
# Store secret
vault kv put secret/web-terminal/jwt-secret value="<secret>"

# Retrieve secret
export WEB_TERMINAL_JWT_SECRET=$(vault kv get -field=value secret/web-terminal/jwt-secret)
```

#### Using AWS Secrets Manager

```bash
# Store secret
aws secretsmanager create-secret \
  --name web-terminal/jwt-secret \
  --secret-string "<secret>"

# Retrieve secret
export WEB_TERMINAL_JWT_SECRET=$(aws secretsmanager get-secret-value \
  --secret-id web-terminal/jwt-secret \
  --query SecretString \
  --output text)
```

---

## Monitoring and Logging

### Prometheus Metrics

```yaml
# prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'web-terminal'
    static_configs:
      # Single port serves metrics endpoint along with all other traffic
      - targets: ['web-terminal:8080']
    metrics_path: /api/v1/metrics
```

### Grafana Dashboards

Import dashboard from `monitoring/grafana/dashboard.json`

Key Metrics:
- Active sessions
- Command execution rate
- WebSocket connections
- Resource usage (CPU, memory, disk)
- Error rates
- Latency (p50, p95, p99)

### Logging

#### Structured JSON Logging

```json
{
  "timestamp": "2025-09-29T10:00:00Z",
  "level": "info",
  "target": "web_terminal::session",
  "message": "Session created",
  "session_id": "abc123",
  "user_id": "user456"
}
```

#### Log Aggregation (ELK Stack)

```yaml
# filebeat.yml

filebeat.inputs:
  - type: container
    paths:
      - '/var/lib/docker/containers/*/*.log'

output.elasticsearch:
  hosts: ["elasticsearch:9200"]

setup.kibana:
  host: "kibana:5601"
```

---

## Backup and Recovery

### Backup Strategy

#### What to Backup

1. **User Workspaces** (important)
   - Location: `/app/data/workspaces`
   - Frequency: Daily
   - Retention: 30 days

2. **Configuration** (critical)
   - Location: `/app/config`
   - Frequency: On change
   - Retention: Indefinite

3. **Application Logs**
   - Location: `/app/data/logs`
   - Frequency: Daily
   - Retention: 90 days

#### Backup Script

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR=/backups
DATE=$(date +%Y%m%d_%H%M%S)

# Backup workspaces
tar -czf $BACKUP_DIR/workspaces_$DATE.tar.gz /app/data/workspaces

# Backup config
tar -czf $BACKUP_DIR/config_$DATE.tar.gz /app/config

# Backup logs (compressed)
tar -czf $BACKUP_DIR/logs_$DATE.tar.gz /app/data/logs

# Upload to S3 (optional)
aws s3 cp $BACKUP_DIR/ s3://web-terminal-backups/$DATE/ --recursive

# Cleanup old backups (keep last 30 days)
find $BACKUP_DIR -type f -mtime +30 -delete
```

### Recovery Procedures

```bash
# Restore workspaces
tar -xzf workspaces_20250929_100000.tar.gz -C /

# Restore config
tar -xzf config_20250929_100000.tar.gz -C /

# Restart application
docker-compose restart web-terminal
```

---

## Rollback Procedures

### Docker Rollback

```bash
# List previous versions
docker images web-terminal

# Stop current version
docker-compose down

# Update docker-compose.yml to previous version
sed -i 's/web-terminal:1.0.1/web-terminal:1.0.0/' docker-compose.yml

# Start previous version
docker-compose up -d
```

### Kubernetes Rollback

```bash
# View deployment history
kubectl rollout history deployment/web-terminal -n web-terminal

# Rollback to previous version
kubectl rollout undo deployment/web-terminal -n web-terminal

# Rollback to specific revision
kubectl rollout undo deployment/web-terminal --to-revision=2 -n web-terminal

# Verify rollback
kubectl rollout status deployment/web-terminal -n web-terminal
```

---

## CI/CD Pipeline

### ⚠️ CRITICAL: GitHub Actions CI/CD is MANDATORY

**Per spec-kit requirements updated 2025-09-29:**
- **GitHub Actions workflows are REQUIRED for all deployments**
- **All CI checks MUST pass before deployment**
- **Security scans are BLOCKING** (no deployments with critical vulnerabilities)
- **Automated releases REQUIRED for version tags**

### CI/CD Architecture (2025 Best Practices)

The web-terminal project uses **automated GitHub Actions workflows** for the entire deployment pipeline:

#### Workflow Overview

1. **`ci-rust.yml`** - Rust backend CI (tests, security, coverage) - **BLOCKING**
2. **`ci-frontend.yml`** - Frontend CI (tests, lint, typecheck) - **BLOCKING**
3. **`ci-integration.yml`** - Full integration tests - **BLOCKING**
4. **`security.yml`** - Security scanning (daily + PR) - **BLOCKING**
5. **`release.yml`** - Automated releases (on version tags)
6. **`deploy.yml`** - Deployment automation (staging → production)

**Deployment Gating:** Deployments are ONLY allowed when ALL CI workflows pass ✅

### GitHub Actions - Complete Deployment Pipeline

**Location:** `.github/workflows/`

#### 1. Continuous Integration (Pre-Deployment)

**Required Workflows (MUST pass before deployment):**

```yaml
# ci-rust.yml - Rust Backend CI
- Test all features
- Run clippy (linting)
- Format check (rustfmt)
- Security audit (cargo audit, cargo deny)
- Code coverage (>80% required)

# ci-frontend.yml - Frontend CI
- ESLint (linting)
- TypeScript type checking
- Vitest unit tests
- Playwright E2E tests
- Code coverage (>75% required)

# ci-integration.yml - Integration Tests
- Build backend + frontend
- Start server on port 8080 (single-port)
- Run full E2E test suite
- Docker build verification

# security.yml - Security Scanning
- cargo audit (Rust vulnerabilities)
- pnpm audit (Node vulnerabilities)
- OWASP ZAP baseline scan
- Trivy container scanning
```

**Failure Policy:** If ANY of these workflows fail, deployment is BLOCKED ❌

#### 2. Automated Release Workflow

**File:** `.github/workflows/release.yml`

**Trigger:** Version tags matching `v*` (e.g., `v1.0.0`)

**Process:**
1. ✅ Verify ALL CI workflows passed
2. Build cross-platform binaries (Linux x86_64/ARM64, macOS universal, Windows)
3. Create GitHub Release with changelog
4. Upload binary artifacts to release
5. Build multi-arch Docker images (amd64, arm64)
6. Push images to GitHub Container Registry (ghcr.io)
7. Optionally trigger deployment to staging

**Key Features:**
- Uses `taiki-e/upload-rust-binary-action@v1` for cross-platform builds
- OIDC authentication for secure cloud deployments (no long-lived secrets)
- Automatic changelog generation from git tags
- Multi-architecture Docker builds using buildx

**Example Release Workflow:**

```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

env:
  REGISTRY: ghcr.io
  IMAGE_NAME: ${{ github.repository }}

jobs:
  # Ensure all CI checks passed before release
  verify-ci:
    runs-on: ubuntu-latest
    steps:
      - name: Check required workflows
        uses: lewagon/wait-on-check-action@v1.3.1
        with:
          ref: ${{ github.ref }}
          check-name: 'test'  # From ci-rust.yml
          repo-token: ${{ secrets.GITHUB_TOKEN }}

  # Build cross-platform binaries
  release-binaries:
    needs: verify-ci
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
          - os: macos-latest
            target: universal-apple-darwin  # Universal binary (x86_64 + aarch64)
          - os: windows-latest
            target: x86_64-pc-windows-msvc

    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Build and upload binaries
        uses: taiki-e/upload-rust-binary-action@v1
        with:
          bin: web-terminal
          target: ${{ matrix.target }}
          tar: unix
          zip: windows
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}

  # Build and push Docker images
  release-docker:
    needs: verify-ci
    runs-on: ubuntu-latest
    permissions:
      contents: read
      packages: write
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Log in to Container Registry
        uses: docker/login-action@v3
        with:
          registry: ${{ env.REGISTRY }}
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Extract metadata
        id: meta
        uses: docker/metadata-action@v5
        with:
          images: ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}
          tags: |
            type=semver,pattern={{version}}
            type=semver,pattern={{major}}.{{minor}}
            type=semver,pattern={{major}}
            type=sha

      - name: Build and push multi-arch image
        uses: docker/build-push-action@v5
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          tags: ${{ steps.meta.outputs.tags }}
          labels: ${{ steps.meta.outputs.labels }}
          cache-from: type=gha
          cache-to: type=gha,mode=max

  # Create GitHub Release
  create-release:
    needs: [release-binaries, release-docker]
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Generate changelog
        id: changelog
        run: |
          # Generate changelog from git commits since last tag
          PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
          if [ -z "$PREV_TAG" ]; then
            CHANGELOG=$(git log --pretty=format:"- %s (%h)" --reverse)
          else
            CHANGELOG=$(git log ${PREV_TAG}..HEAD --pretty=format:"- %s (%h)" --reverse)
          fi
          echo "changelog<<EOF" >> $GITHUB_OUTPUT
          echo "$CHANGELOG" >> $GITHUB_OUTPUT
          echo "EOF" >> $GITHUB_OUTPUT

      - name: Create Release
        uses: softprops/action-gh-release@v1
        with:
          body: |
            ## What's Changed
            ${{ steps.changelog.outputs.changelog }}

            ## Docker Image
            ```
            docker pull ${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.ref_name }}
            ```

            ## Installation
            Download the appropriate binary for your platform from the Assets section below.
          draft: false
          prerelease: false
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

#### 3. Deployment Workflow

**File:** `.github/workflows/deploy.yml`

**Triggers:**
- Automatic: After successful release creation
- Manual: `workflow_dispatch` for emergency deployments

**Environments:**
- `staging` - Automatic deployment after release
- `production` - Requires manual approval

**Process:**
1. ✅ Verify all CI workflows passed
2. Deploy to staging environment
3. Run smoke tests on staging
4. Wait for manual approval (production only)
5. Deploy to production
6. Run smoke tests on production
7. Monitor deployment health

**Example Deployment Workflow:**

```yaml
name: Deploy

on:
  workflow_run:
    workflows: ["Release"]
    types:
      - completed
  workflow_dispatch:
    inputs:
      environment:
        description: 'Environment to deploy to'
        required: true
        type: choice
        options:
          - staging
          - production
      version:
        description: 'Docker image tag to deploy'
        required: true

jobs:
  deploy-staging:
    if: github.event.workflow_run.conclusion == 'success' || github.event_name == 'workflow_dispatch'
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - uses: actions/checkout@v4

      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG_STAGING }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig

      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/web-terminal \
            web-terminal=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.ref_name }} \
            -n staging

      - name: Wait for rollout
        run: |
          kubectl rollout status deployment/web-terminal -n staging --timeout=5m

      - name: Smoke tests
        run: |
          # Test single-port endpoint (8080)
          curl -f http://staging.example.com:8080/health || exit 1
          curl -f http://staging.example.com:8080/ || exit 1

  deploy-production:
    needs: deploy-staging
    runs-on: ubuntu-latest
    environment: production  # Requires manual approval
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - uses: actions/checkout@v4

      - name: Configure kubectl
        run: |
          echo "${{ secrets.KUBECONFIG_PRODUCTION }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig

      - name: Deploy to Kubernetes
        run: |
          kubectl set image deployment/web-terminal \
            web-terminal=${{ env.REGISTRY }}/${{ env.IMAGE_NAME }}:${{ github.ref_name }} \
            -n production

      - name: Wait for rollout
        run: |
          kubectl rollout status deployment/web-terminal -n production --timeout=5m

      - name: Smoke tests
        run: |
          # Test single-port endpoint (8080)
          curl -f https://web-terminal.example.com/health || exit 1
          curl -f https://web-terminal.example.com/ || exit 1

      - name: Notify stakeholders
        if: success()
        run: |
          # Send notification (Slack, email, etc.)
          echo "Production deployment successful: ${{ github.ref_name }}"
```

#### 4. Cloud Deployment with OIDC (No Long-Lived Secrets)

**Example: AWS ECS Deployment**

```yaml
name: Deploy to AWS ECS

on:
  workflow_run:
    workflows: ["Release"]
    types:
      - completed

permissions:
  id-token: write  # Required for OIDC
  contents: read

jobs:
  deploy-ecs:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Configure AWS credentials (OIDC)
        uses: aws-actions/configure-aws-credentials@v4
        with:
          role-to-assume: arn:aws:iam::123456789012:role/GitHubActionsRole
          aws-region: us-east-1

      - name: Login to Amazon ECR
        id: login-ecr
        uses: aws-actions/amazon-ecr-login@v2

      - name: Build and push to ECR
        env:
          ECR_REGISTRY: ${{ steps.login-ecr.outputs.registry }}
          ECR_REPOSITORY: web-terminal
          IMAGE_TAG: ${{ github.sha }}
        run: |
          docker build -t $ECR_REGISTRY/$ECR_REPOSITORY:$IMAGE_TAG .
          docker push $ECR_REGISTRY/$ECR_REPOSITORY:$IMAGE_TAG

      - name: Update ECS service
        run: |
          aws ecs update-service \
            --cluster web-terminal-cluster \
            --service web-terminal-service \
            --force-new-deployment
```

### Security Best Practices

1. **No Long-Lived Secrets:**
   - Use OIDC for cloud deployments (AWS, Azure, GCP)
   - Tokens auto-rotated, valid only minutes
   - Eliminates credential theft risk

2. **Workflow Security:**
   - Pin actions to commit SHA (supply chain security)
   - Require PR approval for `.github/workflows/` changes
   - Use Dependabot for action updates
   - Separate environments with approval gates

3. **Container Security:**
   - Multi-stage Docker builds (minimal attack surface)
   - Non-root user in containers
   - Trivy scanning in CI
   - Regular base image updates via Dependabot

### Performance Targets

Per spec-kit/008-testing-spec.md:
- ✅ **Total CI/CD pipeline: < 5 minutes** (parallel execution)
- ✅ **Build time: < 2 minutes** (with caching)
- ✅ **Deployment time: < 5 minutes** (automated rollout)
- ✅ **Rollback time: < 2 minutes** (automated)

### Monitoring and Rollback

**Automatic Rollback Triggers:**
- Health check failures after deployment
- Error rate spike (>5% increase)
- Latency degradation (>50ms p95)
- Critical log errors

**Manual Rollback:**
```bash
# Kubernetes rollback (automated via GitHub Actions)
kubectl rollout undo deployment/web-terminal -n production

# View rollout history
kubectl rollout history deployment/web-terminal -n production
```

---

## Deployment Checklist

### Pre-Deployment (Automated via GitHub Actions)

**🚨 MANDATORY: ALL GitHub Actions workflows MUST pass before deployment:**

- [x] **`ci-rust.yml` passed** ✅ (tests, clippy, fmt, audit, coverage)
- [x] **`ci-frontend.yml` passed** ✅ (lint, typecheck, tests, coverage)
- [x] **`ci-integration.yml` passed** ✅ (full E2E tests with real server)
- [x] **`security.yml` passed** ✅ (cargo audit, npm audit, OWASP ZAP, Trivy)
- [ ] Code review approved (manual gate)
- [ ] Performance benchmarks passed (enforced in CI)
- [ ] Documentation updated
- [ ] Changelog updated (auto-generated by release workflow)
- [ ] Backup completed (automated)
- [ ] Maintenance window scheduled (if required)

**Authentication Configuration (if enabled):**
- [ ] JWKS provider URLs configured and accessible
- [ ] Allowed users/groups configured
- [ ] JWKS endpoint tested (curl https://backstage.example.com/.well-known/jwks.json)
- [ ] JWT token validation tested
- [ ] Audit logging configured and verified
- [ ] Authentication config mounted in Docker volume

**Failure Policy:** Deployment is BLOCKED ❌ if ANY workflow fails. No exceptions.

### Deployment (Automated via GitHub Actions)

**Process (fully automated by `release.yml` and `deploy.yml`):**

- [x] Build and tag Docker image (multi-arch: amd64, arm64)
- [x] Push image to GitHub Container Registry (ghcr.io)
- [x] Build cross-platform binaries (Linux, macOS universal, Windows)
- [x] Create GitHub Release with artifacts
- [x] Deploy to staging environment (automated)
- [x] Run smoke tests on staging (automated health checks)
- [ ] Manual approval gate for production (GitHub Environment protection)
- [x] Deploy to production (automated after approval)
- [x] Monitor deployment progress (Kubernetes rollout status)
- [x] Verify health checks passing (automated curl tests)

**Manual Steps:**
1. Create git tag: `git tag -a v1.0.0 -m "Release v1.0.0"`
2. Push tag: `git push origin v1.0.0`
3. GitHub Actions handles everything else automatically
4. Approve production deployment when ready (GitHub UI)

### Post-Deployment (Automated Monitoring)

**Automated Checks:**
- [x] Run smoke tests on production (health endpoint, main page)
- [x] Monitor error rates (Prometheus alerts)
- [x] Monitor performance metrics (Grafana dashboards)
- [x] Check logs for errors (structured logging to ELK stack)
- [x] Verify health checks passing (Kubernetes liveness/readiness probes)

**Manual Verification:**
- [ ] Verify user-facing functionality (manual QA)
- [ ] Update status page (if applicable)
- [ ] Notify stakeholders (automated via workflow success)
- [ ] Document any issues

**Rollback Procedure (if needed):**
```bash
# Automated rollback via GitHub Actions
kubectl rollout undo deployment/web-terminal -n production

# Or trigger rollback workflow
gh workflow run deploy.yml -f environment=production -f version=v1.0.0
```

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial deployment specification |