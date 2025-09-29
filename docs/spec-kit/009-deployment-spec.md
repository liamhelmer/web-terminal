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
      - sessions:/app/data/sessions
      - workspaces:/app/data/workspaces
      - logs:/app/data/logs
    environment:
      - WEB_TERMINAL_CONFIG=/app/config/config.toml
      - WEB_TERMINAL_LOG_LEVEL=info
      - WEB_TERMINAL_PORT=8080  # Single port configuration
      - RUST_BACKTRACE=1
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
  sessions:
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
        - name: WEB_TERMINAL_JWT_SECRET
          valueFrom:
            secretKeyRef:
              name: web-terminal-secrets
              key: jwt-secret
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
        - name: sessions
          mountPath: /app/data/sessions
        - name: workspaces
          mountPath: /app/data/workspaces
      volumes:
      - name: config
        configMap:
          name: web-terminal-config
      - name: sessions
        persistentVolumeClaim:
          claimName: sessions-pvc
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
  name: sessions-pvc
spec:
  accessModes:
    - ReadWriteMany
  resources:
    requests:
      storage: 10Gi
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

# Create config map
kubectl create configmap web-terminal-config \
  --from-file=config.toml \
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
└── production.toml      # Production overrides
```

### Configuration Loading

```bash
# Development
WEB_TERMINAL_CONFIG=config/dev.toml web-terminal start

# Staging
WEB_TERMINAL_CONFIG=config/staging.toml web-terminal start

# Production
WEB_TERMINAL_CONFIG=config/production.toml web-terminal start
```

### Secrets Management

#### Using Environment Variables

```bash
# Never commit secrets to git
export WEB_TERMINAL_JWT_SECRET=$(openssl rand -base64 32)
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

1. **Session Data** (optional, ephemeral)
   - Location: `/app/data/sessions`
   - Frequency: Daily
   - Retention: 7 days

2. **User Workspaces** (important)
   - Location: `/app/data/workspaces`
   - Frequency: Daily
   - Retention: 30 days

3. **Configuration** (critical)
   - Location: `/app/config`
   - Frequency: On change
   - Retention: Indefinite

4. **Application Logs**
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

### GitHub Actions

```yaml
# .github/workflows/deploy.yml

name: Deploy

on:
  push:
    branches:
      - main
    tags:
      - 'v*'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Build Docker image
        run: docker build -t web-terminal:${{ github.sha }} .

      - name: Tag image
        run: |
          docker tag web-terminal:${{ github.sha }} registry.example.com/web-terminal:${{ github.sha }}
          docker tag web-terminal:${{ github.sha }} registry.example.com/web-terminal:latest

      - name: Push to registry
        run: |
          echo "${{ secrets.DOCKER_PASSWORD }}" | docker login -u "${{ secrets.DOCKER_USERNAME }}" --password-stdin registry.example.com
          docker push registry.example.com/web-terminal:${{ github.sha }}
          docker push registry.example.com/web-terminal:latest

  deploy-staging:
    needs: build
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - name: Deploy to staging
        run: |
          kubectl set image deployment/web-terminal web-terminal=registry.example.com/web-terminal:${{ github.sha }} -n staging

  deploy-production:
    needs: deploy-staging
    runs-on: ubuntu-latest
    environment: production
    if: startsWith(github.ref, 'refs/tags/v')
    steps:
      - name: Deploy to production
        run: |
          kubectl set image deployment/web-terminal web-terminal=registry.example.com/web-terminal:${{ github.sha }} -n production
```

---

## Deployment Checklist

### Pre-Deployment

- [ ] All tests passing
- [ ] Code review approved
- [ ] Security scan completed
- [ ] Performance benchmarks passed
- [ ] Documentation updated
- [ ] Changelog updated
- [ ] Backup completed
- [ ] Maintenance window scheduled (if required)

### Deployment

- [ ] Build and tag Docker image
- [ ] Push image to registry
- [ ] Update configuration (if needed)
- [ ] Deploy to staging environment
- [ ] Run smoke tests on staging
- [ ] Deploy to production
- [ ] Monitor deployment progress
- [ ] Verify health checks passing

### Post-Deployment

- [ ] Run smoke tests on production
- [ ] Monitor error rates
- [ ] Monitor performance metrics
- [ ] Check logs for errors
- [ ] Verify user-facing functionality
- [ ] Update status page
- [ ] Notify stakeholders
- [ ] Document any issues

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial deployment specification |