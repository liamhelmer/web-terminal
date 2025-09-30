# Web-Terminal: Rust Backend Specification

**Version:** 1.1.0
**Status:** Draft
**Author:** Liam Helmer
**Last Updated:** 2025-09-29
**References:** [002-architecture.md](./002-architecture.md), [011-authentication-spec.md](./011-authentication-spec.md)

---

## Table of Contents

1. [Module Structure](#module-structure)
2. [Core Components](#core-components)
3. [Authentication & Authorization](#authentication--authorization)
4. [API Endpoints](#api-endpoints)
5. [Data Models](#data-models)
6. [Error Handling](#error-handling)
7. [Configuration](#configuration)
8. [Performance Optimizations](#performance-optimizations)

---

## Module Structure

```
src/
├── main.rs                    # Application entry point
├── lib.rs                     # Library root
├── config/
│   ├── mod.rs
│   ├── server.rs             # Server configuration
│   ├── security.rs           # Security configuration
│   └── jwks.rs               # JWKS provider configuration
├── server/
│   ├── mod.rs
│   ├── http.rs               # HTTP server setup
│   ├── websocket.rs          # WebSocket handler
│   └── middleware.rs         # Auth, logging middleware
├── session/
│   ├── mod.rs
│   ├── manager.rs            # Session lifecycle
│   ├── state.rs              # Session state
│   └── registry.rs           # Session registry
├── execution/
│   ├── mod.rs
│   ├── executor.rs           # Command execution
│   ├── process.rs            # Process management
│   └── environment.rs        # Environment variables
├── filesystem/
│   ├── mod.rs
│   ├── vfs.rs                # Virtual file system
│   ├── quota.rs              # Storage quotas
│   └── operations.rs         # File operations
├── security/
│   ├── mod.rs
│   ├── auth.rs               # JWT verification & authentication
│   ├── jwks.rs               # JWKS client and cache
│   ├── claims.rs             # Claims extraction and validation
│   ├── authz.rs              # Authorization logic
│   ├── sandbox.rs            # Sandboxing
│   ├── limits.rs             # Resource limits
│   └── validator.rs          # Input validation
├── protocol/
│   ├── mod.rs
│   ├── messages.rs           # Message types
│   └── codec.rs              # Serialization
├── monitoring/
│   ├── mod.rs
│   ├── metrics.rs            # Prometheus metrics
│   └── logging.rs            # Structured logging
└── error.rs                   # Error types
```

---

## Core Components

### 1. Server Module

#### 1.1 HTTP Server (src/server/http.rs)

```rust
use actix_web::{web, App, HttpServer};
use rustls::ServerConfig;

pub struct Server {
    config: ServerConfig,
    sessions: Arc<SessionManager>,
    auth: Arc<AuthService>,
}

impl Server {
    pub async fn new(config: ServerConfig) -> Result<Self> {
        let sessions = Arc::new(SessionManager::new());
        let auth = Arc::new(AuthService::new());

        Ok(Self {
            config,
            sessions,
            auth,
        })
    }

    pub async fn run(self) -> Result<()> {
        HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(self.sessions.clone()))
                .app_data(web::Data::new(self.auth.clone()))
                .wrap(middleware::Logger::default())
                .wrap(middleware::JwtAuth::new(self.auth.clone()))
                .wrap(middleware::RateLimit::default())
                .service(routes::health)
                .service(routes::websocket)
                .service(actix_files::Files::new("/", "./static"))
        })
        .bind((self.config.host.as_str(), self.config.port))?
        .run()
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,
    pub max_connections: usize,
    pub worker_threads: usize,
}
```

#### 1.2 WebSocket Handler (src/server/websocket.rs)

```rust
use actix::{Actor, StreamHandler};
use actix_web_actors::ws;

pub struct WebSocketSession {
    id: SessionId,
    manager: Arc<SessionManager>,
    executor: Arc<CommandExecutor>,
    last_heartbeat: Instant,
}

impl Actor for WebSocketSession {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);
        tracing::info!("WebSocket session started: {}", self.id);
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        tracing::info!("WebSocket session stopped: {}", self.id);
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebSocketSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                self.handle_command(text, ctx);
            }
            Ok(ws::Message::Binary(bin)) => {
                self.handle_binary(bin, ctx);
            }
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl WebSocketSession {
    fn handle_command(&mut self, text: String, ctx: &mut ws::WebsocketContext<Self>) {
        let msg: ClientMessage = match serde_json::from_str(&text) {
            Ok(m) => m,
            Err(e) => {
                tracing::error!("Failed to parse message: {}", e);
                return;
            }
        };

        match msg {
            ClientMessage::Command { data } => {
                self.execute_command(data, ctx);
            }
            ClientMessage::Resize { cols, rows } => {
                self.resize_terminal(cols, rows);
            }
            ClientMessage::Signal { signal } => {
                self.send_signal(signal);
            }
        }
    }

    fn execute_command(&mut self, cmd: String, ctx: &mut ws::WebsocketContext<Self>) {
        let executor = self.executor.clone();
        let session_id = self.id.clone();

        let fut = async move {
            let result = executor.execute(session_id, cmd).await;
            match result {
                Ok(output) => ServerMessage::Output { data: output },
                Err(e) => ServerMessage::Error { message: e.to_string() },
            }
        };

        let fut = actix::fut::wrap_future(fut);
        ctx.spawn(fut.map(|msg, _act, ctx| {
            ctx.text(serde_json::to_string(&msg).unwrap());
        }));
    }

    fn heartbeat(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > Duration::from_secs(30) {
                tracing::warn!("WebSocket heartbeat timeout");
                ctx.stop();
                return;
            }
            ctx.ping(b"");
        });
    }
}
```

---

### 2. Session Module

#### 2.1 Session Manager (src/session/manager.rs)

```rust
use dashmap::DashMap;
use tokio::sync::RwLock;

pub struct SessionManager {
    sessions: DashMap<SessionId, Arc<Session>>,
    user_sessions: DashMap<UserId, Vec<SessionId>>,
    config: SessionConfig,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
            user_sessions: DashMap::new(),
            config: SessionConfig::default(),
        }
    }

    pub async fn create_session(&self, user_id: UserId) -> Result<Session> {
        // Check user session limit
        if let Some(sessions) = self.user_sessions.get(&user_id) {
            if sessions.len() >= self.config.max_sessions_per_user {
                return Err(Error::SessionLimitExceeded);
            }
        }

        let session = Session::new(user_id, self.config.clone())?;
        let session_id = session.id.clone();

        // Store session
        self.sessions.insert(session_id.clone(), Arc::new(session.clone()));

        // Track user sessions
        self.user_sessions
            .entry(user_id)
            .or_insert_with(Vec::new)
            .push(session_id);

        tracing::info!("Created session {} for user {}", session.id, user_id);
        Ok(session)
    }

    pub async fn get_session(&self, session_id: &SessionId) -> Result<Arc<Session>> {
        self.sessions
            .get(session_id)
            .map(|s| s.clone())
            .ok_or(Error::SessionNotFound)
    }

    pub async fn destroy_session(&self, session_id: &SessionId) -> Result<()> {
        if let Some((_, session)) = self.sessions.remove(session_id) {
            // Kill all processes
            session.kill_all_processes().await?;

            // Clean up file system
            session.cleanup_filesystem().await?;

            // Remove from user sessions
            if let Some(mut user_sessions) = self.user_sessions.get_mut(&session.user_id) {
                user_sessions.retain(|id| id != session_id);
            }

            tracing::info!("Destroyed session {}", session_id);
            Ok(())
        } else {
            Err(Error::SessionNotFound)
        }
    }

    pub async fn cleanup_expired_sessions(&self) -> Result<usize> {
        let now = Instant::now();
        let mut expired = Vec::new();

        for entry in self.sessions.iter() {
            let session = entry.value();
            if now.duration_since(session.last_activity) > self.config.timeout {
                expired.push(entry.key().clone());
            }
        }

        let count = expired.len();
        for session_id in expired {
            self.destroy_session(&session_id).await?;
        }

        tracing::info!("Cleaned up {} expired sessions", count);
        Ok(count)
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub timeout: Duration,
    pub max_sessions_per_user: usize,
    pub workspace_quota: u64,
    pub max_processes: usize,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30 * 60), // 30 minutes
            max_sessions_per_user: 10,
            workspace_quota: 1024 * 1024 * 1024, // 1GB
            max_processes: 10,
        }
    }
}
```

#### 2.2 Session State (src/session/state.rs)

```rust
pub struct Session {
    pub id: SessionId,
    pub user_id: UserId,
    pub created_at: Instant,
    pub last_activity: Instant,
    state: RwLock<SessionState>,
}

#[derive(Debug, Clone)]
pub struct SessionState {
    pub working_dir: PathBuf,
    pub environment: HashMap<String, String>,
    pub command_history: Vec<String>,
    pub processes: HashMap<ProcessId, ProcessHandle>,
}

impl Session {
    pub fn new(user_id: UserId, config: SessionConfig) -> Result<Self> {
        let id = SessionId::generate();
        let working_dir = PathBuf::from(format!("/workspace/{}", id));

        // Create workspace directory
        std::fs::create_dir_all(&working_dir)?;

        let state = SessionState {
            working_dir,
            environment: Self::default_environment(),
            command_history: Vec::new(),
            processes: HashMap::new(),
        };

        Ok(Self {
            id,
            user_id,
            created_at: Instant::now(),
            last_activity: Instant::now(),
            state: RwLock::new(state),
        })
    }

    pub async fn update_working_dir(&self, path: PathBuf) -> Result<()> {
        let mut state = self.state.write().await;

        // Validate path is within workspace
        if !path.starts_with(&state.working_dir) {
            return Err(Error::InvalidPath);
        }

        state.working_dir = path;
        Ok(())
    }

    pub async fn add_to_history(&self, command: String) {
        let mut state = self.state.write().await;
        state.command_history.push(command);

        // Limit history size
        if state.command_history.len() > 1000 {
            state.command_history.remove(0);
        }
    }

    pub async fn get_history(&self) -> Vec<String> {
        let state = self.state.read().await;
        state.command_history.clone()
    }

    fn default_environment() -> HashMap<String, String> {
        let mut env = HashMap::new();
        env.insert("PATH".to_string(), "/usr/bin:/bin".to_string());
        env.insert("HOME".to_string(), "/workspace".to_string());
        env.insert("SHELL".to_string(), "/bin/bash".to_string());
        env
    }
}
```

---

### 3. Execution Module

#### 3.1 Command Executor (src/execution/executor.rs)

```rust
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

pub struct CommandExecutor {
    sandbox: Arc<SandboxManager>,
    limits: Arc<ResourceLimiter>,
}

impl CommandExecutor {
    pub fn new(sandbox: Arc<SandboxManager>, limits: Arc<ResourceLimiter>) -> Self {
        Self { sandbox, limits }
    }

    pub async fn execute(&self, session_id: SessionId, cmd: String) -> Result<CommandOutput> {
        // Validate command
        self.sandbox.validate_command(&cmd)?;

        // Check resource limits
        self.limits.check_limit(&session_id, Resource::Processes)?;

        // Parse command
        let parts = shell_words::split(&cmd)?;
        if parts.is_empty() {
            return Err(Error::EmptyCommand);
        }

        let program = &parts[0];
        let args = &parts[1..];

        // Create process
        let mut child = Command::new(program)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Stream output
        let stdout = child.stdout.take().unwrap();
        let stderr = child.stderr.take().unwrap();

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        let mut stdout_lines = stdout_reader.lines();
        let mut stderr_lines = stderr_reader.lines();

        let mut output = CommandOutput::new();

        // Read output
        loop {
            tokio::select! {
                line = stdout_lines.next_line() => {
                    match line? {
                        Some(line) => output.stdout.push_str(&line),
                        None => break,
                    }
                }
                line = stderr_lines.next_line() => {
                    match line? {
                        Some(line) => output.stderr.push_str(&line),
                        None => {},
                    }
                }
            }
        }

        // Wait for completion
        let status = child.wait().await?;
        output.exit_code = status.code().unwrap_or(-1);

        Ok(output)
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl CommandOutput {
    fn new() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }
    }
}
```

---

### 4. Security Module

**See [011-authentication-spec.md](./011-authentication-spec.md) for complete authentication architecture.**

#### 4.1 JWKS Client (src/security/jwks.rs)

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use jsonwebtoken::jwk::JwkSet;

/// JWKS client for fetching and caching public keys from multiple providers
pub struct JwksClient {
    client: Client,
    cache: Arc<RwLock<JwksCache>>,
    providers: Vec<JwksProvider>,
}

#[derive(Debug, Clone)]
pub struct JwksProvider {
    pub name: String,
    pub jwks_uri: String,
    pub issuer: String,
    pub audience: Option<String>,
}

#[derive(Debug)]
struct JwksCache {
    keys: HashMap<String, CachedKey>,
}

#[derive(Debug, Clone)]
struct CachedKey {
    jwk: Jwk,
    cached_at: Instant,
    expires_at: Option<Instant>,
}

impl JwksClient {
    pub fn new(providers: Vec<JwksProvider>) -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            cache: Arc::new(RwLock::new(JwksCache {
                keys: HashMap::new(),
            })),
            providers,
        }
    }

    /// Fetch JWKS from all configured providers
    pub async fn refresh_all(&self) -> Result<()> {
        for provider in &self.providers {
            if let Err(e) = self.refresh_provider(provider).await {
                tracing::error!("Failed to refresh JWKS for {}: {}", provider.name, e);
            }
        }
        Ok(())
    }

    /// Fetch JWKS from specific provider
    async fn refresh_provider(&self, provider: &JwksProvider) -> Result<()> {
        let response = self.client.get(&provider.jwks_uri).send().await?;
        let jwks: JwkSet = response.json().await?;

        let mut cache = self.cache.write().await;
        let now = Instant::now();

        for jwk in jwks.keys {
            if let Some(kid) = &jwk.common.key_id {
                cache.keys.insert(
                    kid.clone(),
                    CachedKey {
                        jwk,
                        cached_at: now,
                        expires_at: Some(now + Duration::from_secs(3600)), // 1 hour cache
                    },
                );
            }
        }

        tracing::info!("Refreshed JWKS for provider: {}", provider.name);
        Ok(())
    }

    /// Get signing key by kid
    pub async fn get_key(&self, kid: &str) -> Result<DecodingKey> {
        let cache = self.cache.read().await;

        if let Some(cached) = cache.keys.get(kid) {
            // Check if expired
            if let Some(expires_at) = cached.expires_at {
                if Instant::now() > expires_at {
                    drop(cache);
                    self.refresh_all().await?;
                    return self.get_key(kid).await;
                }
            }

            return DecodingKey::from_jwk(&cached.jwk)
                .map_err(|e| Error::JwksError(e.to_string()));
        }

        Err(Error::KeyNotFound(kid.to_string()))
    }

    /// Get provider config by issuer
    pub fn get_provider(&self, issuer: &str) -> Option<&JwksProvider> {
        self.providers.iter().find(|p| p.issuer == issuer)
    }
}
```

#### 4.2 JWT Authentication Service (src/security/auth.rs)

```rust
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use crate::security::claims::BackstageClaims;
use crate::security::jwks::JwksClient;

pub struct AuthService {
    jwks_client: Arc<JwksClient>,
    validation_config: ValidationConfig,
}

#[derive(Debug, Clone)]
pub struct ValidationConfig {
    pub allowed_issuers: Vec<String>,
    pub required_audience: Option<String>,
    pub algorithms: Vec<Algorithm>,
    pub validate_exp: bool,
    pub validate_nbf: bool,
    pub leeway: u64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            allowed_issuers: Vec::new(),
            required_audience: None,
            algorithms: vec![Algorithm::RS256, Algorithm::ES256],
            validate_exp: true,
            validate_nbf: true,
            leeway: 60, // 60 seconds
        }
    }
}

impl AuthService {
    pub fn new(jwks_client: Arc<JwksClient>, config: ValidationConfig) -> Self {
        Self {
            jwks_client,
            validation_config: config,
        }
    }

    /// Verify and decode JWT token
    pub async fn verify_token(&self, token: &str) -> Result<BackstageClaims> {
        // Decode header to get kid and algorithm
        let header = decode_header(token)?;

        let kid = header
            .kid
            .ok_or_else(|| Error::InvalidToken("Missing kid in token header".to_string()))?;

        // Get decoding key from JWKS
        let decoding_key = self.jwks_client.get_key(&kid).await?;

        // Configure validation
        let mut validation = Validation::new(
            header.alg
        );

        validation.set_issuer(&self.validation_config.allowed_issuers);

        if let Some(aud) = &self.validation_config.required_audience {
            validation.set_audience(&[aud]);
        }

        validation.validate_exp = self.validation_config.validate_exp;
        validation.validate_nbf = self.validation_config.validate_nbf;
        validation.leeway = self.validation_config.leeway;

        // Decode and verify token
        let token_data = decode::<BackstageClaims>(
            token,
            &decoding_key,
            &validation,
        )?;

        // Verify issuer is in provider list
        let issuer = token_data.claims.iss.as_deref()
            .ok_or_else(|| Error::InvalidToken("Missing issuer".to_string()))?;

        if !self.validation_config.allowed_issuers.contains(&issuer.to_string()) {
            return Err(Error::InvalidToken(format!("Untrusted issuer: {}", issuer)));
        }

        Ok(token_data.claims)
    }

    /// Extract claims from request
    pub async fn authenticate_request(
        &self,
        auth_header: Option<&str>,
    ) -> Result<BackstageClaims> {
        let token = auth_header
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(Error::MissingAuthHeader)?;

        self.verify_token(token).await
    }
}
```

#### 4.3 Claims Extraction (src/security/claims.rs)

```rust
use serde::{Deserialize, Serialize};

/// Backstage-compatible JWT claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackstageClaims {
    // Standard JWT claims
    pub sub: String,           // Subject: "user:default/username"
    pub iss: Option<String>,   // Issuer
    pub aud: Option<Vec<String>>, // Audience
    pub exp: usize,            // Expiration time
    pub iat: usize,            // Issued at
    pub nbf: Option<usize>,    // Not before

    // Backstage-specific claims
    pub ent: Vec<String>,      // Entity references (ownership)

    // Optional claims
    pub email: Option<String>,
    pub name: Option<String>,
    pub groups: Option<Vec<String>>,
}

impl BackstageClaims {
    /// Extract user ID from subject claim
    pub fn user_id(&self) -> Result<String> {
        // Parse "user:default/username" format
        if let Some(user_ref) = self.sub.strip_prefix("user:") {
            if let Some((_namespace, username)) = user_ref.split_once('/') {
                return Ok(username.to_string());
            }
        }

        // Fallback to full subject
        Ok(self.sub.clone())
    }

    /// Get owned entity references
    pub fn owned_entities(&self) -> &[String] {
        &self.ent
    }

    /// Check if user is in group
    pub fn is_in_group(&self, group: &str) -> bool {
        self.groups
            .as_ref()
            .map(|g| g.iter().any(|gn| gn == group))
            .unwrap_or(false)
    }

    /// Get email or None
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}
```

#### 4.4 Authorization Service (src/security/authz.rs)

```rust
use crate::security::claims::BackstageClaims;

pub struct AuthorizationService {
    rules: Vec<AuthzRule>,
}

#[derive(Debug, Clone)]
pub struct AuthzRule {
    pub resource: String,
    pub action: String,
    pub allowed_groups: Vec<String>,
    pub allowed_users: Vec<String>,
}

impl AuthorizationService {
    pub fn new(rules: Vec<AuthzRule>) -> Self {
        Self { rules }
    }

    /// Check if user is authorized for action on resource
    pub fn authorize(
        &self,
        claims: &BackstageClaims,
        resource: &str,
        action: &str,
    ) -> Result<bool> {
        let user_id = claims.user_id()?;

        for rule in &self.rules {
            if rule.resource == resource && rule.action == action {
                // Check user whitelist
                if rule.allowed_users.contains(&user_id) {
                    return Ok(true);
                }

                // Check group membership
                if let Some(groups) = &claims.groups {
                    for group in groups {
                        if rule.allowed_groups.contains(group) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        Ok(false)
    }

    /// Check session creation authorization
    pub fn can_create_session(&self, claims: &BackstageClaims) -> Result<bool> {
        self.authorize(claims, "terminal", "create_session")
    }

    /// Check command execution authorization
    pub fn can_execute_command(&self, claims: &BackstageClaims) -> Result<bool> {
        self.authorize(claims, "terminal", "execute_command")
    }
}

/// Default authorization rules
pub fn default_authz_rules() -> Vec<AuthzRule> {
    vec![
        AuthzRule {
            resource: "terminal".to_string(),
            action: "create_session".to_string(),
            allowed_groups: vec!["developers".to_string(), "admins".to_string()],
            allowed_users: vec![],
        },
        AuthzRule {
            resource: "terminal".to_string(),
            action: "execute_command".to_string(),
            allowed_groups: vec!["developers".to_string(), "admins".to_string()],
            allowed_users: vec![],
        },
    ]
}
```

#### 4.5 JWT Authentication Middleware (src/server/middleware.rs)

```rust
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

pub struct JwtAuth {
    auth_service: Arc<AuthService>,
}

impl JwtAuth {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service,
            auth_service: self.auth_service.clone(),
        }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: S,
    auth_service: Arc<AuthService>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Skip auth for health endpoint
        if req.path() == "/health" {
            let fut = self.service.call(req);
            return Box::pin(async move { fut.await });
        }

        let auth_service = self.auth_service.clone();
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string());

        Box::pin(async move {
            match auth_header {
                Some(header) => {
                    // Verify token
                    match auth_service.authenticate_request(Some(&header)).await {
                        Ok(claims) => {
                            // Store claims in request extensions
                            req.extensions_mut().insert(claims);

                            // Continue with request
                            let fut = self.service.call(req);
                            fut.await
                        }
                        Err(e) => {
                            tracing::warn!("Authentication failed: {}", e);
                            Err(actix_web::error::ErrorUnauthorized("Invalid token"))
                        }
                    }
                }
                None => Err(actix_web::error::ErrorUnauthorized("Missing authorization header")),
            }
        })
    }
}

/// Extract claims from request extensions
pub fn extract_claims(req: &HttpRequest) -> Result<BackstageClaims> {
    req.extensions()
        .get::<BackstageClaims>()
        .cloned()
        .ok_or(Error::Unauthorized)
}
```

---

## Authentication & Authorization

### Overview

The backend implements JWT-based authentication with JWKS (JSON Web Key Set) support for secure token verification. This enables integration with enterprise identity providers like Backstage, Keycloak, Auth0, etc.

### Key Features

1. **JWKS Support**: Fetches and caches public keys from multiple identity providers
2. **Multi-Provider**: Supports multiple JWT issuers simultaneously
3. **Backstage Compatible**: Parses Backstage-specific claims (entity references, ownership)
4. **Role-Based Authorization**: Group and user-based access control
5. **Automatic Key Rotation**: Refreshes JWKS keys with expiration handling
6. **Middleware Integration**: Transparent authentication for all protected routes

### Authentication Flow

```
1. Client sends request with JWT in Authorization header
   ↓
2. JwtAuthMiddleware extracts token
   ↓
3. AuthService decodes token header to get kid (key ID)
   ↓
4. JwksClient retrieves corresponding public key (cached or fetched)
   ↓
5. Token is verified using public key
   ↓
6. Claims are extracted and validated
   ↓
7. AuthorizationService checks permissions
   ↓
8. Request proceeds with user context in extensions
```

### Configuration Example

```rust
// Configure JWKS providers
let providers = vec![
    JwksProvider {
        name: "backstage".to_string(),
        jwks_uri: "https://backstage.example.com/.well-known/jwks.json".to_string(),
        issuer: "https://backstage.example.com".to_string(),
        audience: Some("web-terminal".to_string()),
    },
];

// Initialize JWKS client
let jwks_client = Arc::new(JwksClient::new(providers));

// Configure validation
let validation_config = ValidationConfig {
    allowed_issuers: vec!["https://backstage.example.com".to_string()],
    required_audience: Some("web-terminal".to_string()),
    algorithms: vec![Algorithm::RS256, Algorithm::ES256],
    validate_exp: true,
    validate_nbf: true,
    leeway: 60,
};

// Create auth service
let auth_service = Arc::new(AuthService::new(jwks_client.clone(), validation_config));

// Configure authorization rules
let authz_rules = default_authz_rules();
let authz_service = Arc::new(AuthorizationService::new(authz_rules));

// Background task to refresh JWKS
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        if let Err(e) = jwks_client.refresh_all().await {
            tracing::error!("Failed to refresh JWKS: {}", e);
        }
    }
});
```

### Usage in Handlers

```rust
use actix_web::{web, HttpRequest, HttpResponse};
use crate::security::claims::BackstageClaims;
use crate::server::middleware::extract_claims;

async fn create_session(
    req: HttpRequest,
    session_manager: web::Data<Arc<SessionManager>>,
    authz: web::Data<Arc<AuthorizationService>>,
) -> Result<HttpResponse> {
    // Extract claims from middleware
    let claims = extract_claims(&req)?;

    // Check authorization
    if !authz.can_create_session(&claims)? {
        return Err(Error::Unauthorized);
    }

    // Get user ID
    let user_id = claims.user_id()?;

    // Create session
    let session = session_manager.create_session(user_id).await?;

    Ok(HttpResponse::Ok().json(session))
}
```

---

## Data Models

### Protocol Messages

```rust
// src/protocol/messages.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Command {
        data: String,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Signal {
        signal: Signal,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Output {
        data: String,
    },
    Error {
        message: String,
    },
    ProcessExited {
        exit_code: i32,
    },
    ConnectionStatus {
        status: ConnectionStatus,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Signal {
    SIGINT = 2,
    SIGTERM = 15,
    SIGKILL = 9,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    Reconnecting,
}
```

---

## Error Handling

```rust
// src/error.rs

#[derive(Debug, thiserror::Error)]
pub enum Error {
    // Session errors
    #[error("Session not found")]
    SessionNotFound,

    #[error("Session limit exceeded")]
    SessionLimitExceeded,

    // Command execution errors
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    #[error("Command not allowed: {0}")]
    CommandNotAllowed(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),

    // Authentication errors
    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Missing authorization header")]
    MissingAuthHeader,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Unauthorized")]
    Unauthorized,

    // JWKS errors
    #[error("JWKS error: {0}")]
    JwksError(String),

    #[error("Key not found: {0}")]
    KeyNotFound(String),

    #[error("Failed to fetch JWKS: {0}")]
    JwksFetchError(String),

    // Authorization errors
    #[error("Authorization denied for resource '{0}' action '{1}'")]
    AuthorizationDenied(String, String),

    #[error("Forbidden")]
    Forbidden,

    // Claims errors
    #[error("Invalid claims: {0}")]
    InvalidClaims(String),

    #[error("Missing required claim: {0}")]
    MissingClaim(String),

    // External errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

// HTTP status code mapping for API responses
impl Error {
    pub fn status_code(&self) -> actix_web::http::StatusCode {
        use actix_web::http::StatusCode;

        match self {
            Error::SessionNotFound => StatusCode::NOT_FOUND,
            Error::SessionLimitExceeded => StatusCode::TOO_MANY_REQUESTS,
            Error::InvalidCommand(_) => StatusCode::BAD_REQUEST,
            Error::CommandNotAllowed(_) => StatusCode::FORBIDDEN,
            Error::ResourceLimitExceeded(_) => StatusCode::TOO_MANY_REQUESTS,
            Error::AuthenticationFailed => StatusCode::UNAUTHORIZED,
            Error::MissingAuthHeader => StatusCode::UNAUTHORIZED,
            Error::InvalidToken(_) => StatusCode::UNAUTHORIZED,
            Error::Unauthorized => StatusCode::UNAUTHORIZED,
            Error::AuthorizationDenied(_, _) => StatusCode::FORBIDDEN,
            Error::Forbidden => StatusCode::FORBIDDEN,
            Error::InvalidClaims(_) => StatusCode::UNAUTHORIZED,
            Error::MissingClaim(_) => StatusCode::UNAUTHORIZED,
            Error::JwksError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::KeyNotFound(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::JwksFetchError(_) => StatusCode::SERVICE_UNAVAILABLE,
            Error::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Serialization(_) => StatusCode::BAD_REQUEST,
            Error::Jwt(_) => StatusCode::UNAUTHORIZED,
            Error::Http(_) => StatusCode::BAD_GATEWAY,
        }
    }
}
```

---

## Configuration

```rust
// src/config/server.rs

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub security: SecurityConfig,
    pub jwks: JwksConfig,
    pub authorization: AuthorizationConfig,
    pub logging: LoggingConfig,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            server: ServerConfig::from_env()?,
            session: SessionConfig::default(),
            security: SecurityConfig::from_env()?,
            jwks: JwksConfig::from_env()?,
            authorization: AuthorizationConfig::default(),
            logging: LoggingConfig::default(),
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,

    #[serde(default = "default_port")]
    pub port: u16,

    pub tls_cert: Option<PathBuf>,
    pub tls_key: Option<PathBuf>,

    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_max_connections() -> usize {
    10000
}
```

### JWKS Configuration (src/config/jwks.rs)

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwksConfig {
    pub providers: Vec<JwksProviderConfig>,

    #[serde(default = "default_refresh_interval")]
    pub refresh_interval_secs: u64,

    #[serde(default = "default_cache_ttl")]
    pub cache_ttl_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct JwksProviderConfig {
    pub name: String,
    pub jwks_uri: String,
    pub issuer: String,
    pub audience: Option<String>,
}

impl JwksConfig {
    pub fn from_env() -> Result<Self> {
        // Try to load from environment variable (JSON format)
        if let Ok(config_json) = std::env::var("JWKS_CONFIG") {
            return serde_json::from_str(&config_json)
                .map_err(|e| Error::InvalidConfig(format!("Invalid JWKS_CONFIG: {}", e)));
        }

        // Fallback to individual environment variables
        let provider = JwksProviderConfig {
            name: std::env::var("JWKS_PROVIDER_NAME")
                .unwrap_or_else(|_| "default".to_string()),
            jwks_uri: std::env::var("JWKS_URI")
                .map_err(|_| Error::InvalidConfig("JWKS_URI not set".to_string()))?,
            issuer: std::env::var("JWKS_ISSUER")
                .map_err(|_| Error::InvalidConfig("JWKS_ISSUER not set".to_string()))?,
            audience: std::env::var("JWKS_AUDIENCE").ok(),
        };

        Ok(Self {
            providers: vec![provider],
            refresh_interval_secs: default_refresh_interval(),
            cache_ttl_secs: default_cache_ttl(),
        })
    }
}

fn default_refresh_interval() -> u64 {
    3600 // 1 hour
}

fn default_cache_ttl() -> u64 {
    3600 // 1 hour
}
```

### Security Configuration (src/config/security.rs)

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct SecurityConfig {
    // JWT validation settings
    pub jwt: JwtValidationConfig,

    // Sandbox settings
    pub sandbox: SandboxConfig,

    // Rate limiting
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JwtValidationConfig {
    #[serde(default = "default_allowed_algorithms")]
    pub allowed_algorithms: Vec<String>,

    pub allowed_issuers: Vec<String>,

    pub required_audience: Option<String>,

    #[serde(default = "default_validate_exp")]
    pub validate_exp: bool,

    #[serde(default = "default_validate_nbf")]
    pub validate_nbf: bool,

    #[serde(default = "default_leeway")]
    pub leeway_secs: u64,
}

fn default_allowed_algorithms() -> Vec<String> {
    vec!["RS256".to_string(), "ES256".to_string()]
}

fn default_validate_exp() -> bool {
    true
}

fn default_validate_nbf() -> bool {
    true
}

fn default_leeway() -> u64 {
    60
}

impl SecurityConfig {
    pub fn from_env() -> Result<Self> {
        // Load from environment or config file
        Ok(Self {
            jwt: JwtValidationConfig {
                allowed_algorithms: default_allowed_algorithms(),
                allowed_issuers: std::env::var("JWT_ALLOWED_ISSUERS")
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| s.to_string())
                    .collect(),
                required_audience: std::env::var("JWT_REQUIRED_AUDIENCE").ok(),
                validate_exp: default_validate_exp(),
                validate_nbf: default_validate_nbf(),
                leeway_secs: default_leeway(),
            },
            sandbox: SandboxConfig::default(),
            rate_limit: RateLimitConfig::default(),
        })
    }
}
```

### Authorization Configuration

```rust
// src/config/authz.rs

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationConfig {
    pub rules: Vec<AuthzRuleConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthzRuleConfig {
    pub resource: String,
    pub action: String,
    pub allowed_groups: Vec<String>,
    pub allowed_users: Vec<String>,
}

impl Default for AuthorizationConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                AuthzRuleConfig {
                    resource: "terminal".to_string(),
                    action: "create_session".to_string(),
                    allowed_groups: vec!["developers".to_string(), "admins".to_string()],
                    allowed_users: vec![],
                },
                AuthzRuleConfig {
                    resource: "terminal".to_string(),
                    action: "execute_command".to_string(),
                    allowed_groups: vec!["developers".to_string(), "admins".to_string()],
                    allowed_users: vec![],
                },
            ],
        }
    }
}

impl AuthorizationConfig {
    pub fn from_env() -> Result<Self> {
        if let Ok(config_json) = std::env::var("AUTHZ_CONFIG") {
            return serde_json::from_str(&config_json)
                .map_err(|e| Error::InvalidConfig(format!("Invalid AUTHZ_CONFIG: {}", e)));
        }

        Ok(Self::default())
    }
}
```

### Environment Variables

```bash
# Server
HOST=0.0.0.0
PORT=8080

# JWKS Configuration
JWKS_URI=https://backstage.example.com/.well-known/jwks.json
JWKS_ISSUER=https://backstage.example.com
JWKS_AUDIENCE=web-terminal
JWKS_PROVIDER_NAME=backstage

# Or use JSON for multiple providers:
JWKS_CONFIG='{"providers":[{"name":"backstage","jwks_uri":"...","issuer":"...","audience":"..."}]}'

# JWT Validation
JWT_ALLOWED_ISSUERS=https://backstage.example.com,https://auth.example.com
JWT_REQUIRED_AUDIENCE=web-terminal

# Authorization
AUTHZ_CONFIG='{"rules":[{"resource":"terminal","action":"create_session","allowed_groups":["developers","admins"],"allowed_users":[]}]}'
```

---

## Performance Optimizations

### 1. Async I/O Throughout
- All I/O operations use Tokio async runtime
- No blocking operations in hot paths
- Efficient use of system threads

### 2. Zero-Copy Where Possible
- Use `Bytes` for buffer management
- Avoid unnecessary string allocations
- Stream output instead of buffering

### 3. Connection Pooling
- Reuse WebSocket connections
- Pool process executors
- Cache frequently accessed data

### 4. Memory Management
- Use `Arc` for shared data
- Minimize cloning with references
- Drop resources eagerly

---

## Dependencies

The following Rust dependencies are required for JWT/JWKS authentication:

```toml
[dependencies]
# Core server
actix-web = "4.x"
actix-web-actors = "4.x"
actix-files = "0.6"
tokio = { version = "1.x", features = ["full"] }

# JWT and JWKS
jsonwebtoken = "9.x"
reqwest = { version = "0.11.x", features = ["json"] }

# Serialization
serde = { version = "1.x", features = ["derive"] }
serde_json = "1.x"

# Data structures
dashmap = "5.x"

# Error handling
thiserror = "1.x"
anyhow = "1.x"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Utilities
chrono = "0.4"
uuid = { version = "1.x", features = ["v4", "serde"] }
```

### Key Dependencies

1. **jsonwebtoken (9.x)**: JWT token verification and validation
   - RS256, ES256 algorithm support
   - JWKS integration via `jwk` module
   - Claims validation

2. **reqwest (0.11.x)**: HTTP client for JWKS fetching
   - Async support with Tokio
   - JSON deserialization
   - Timeout handling

3. **actix-web (4.x)**: Web framework
   - Middleware support
   - Request extensions for claims storage
   - WebSocket support

4. **dashmap (5.x)**: Concurrent hash map for JWKS cache
   - Lock-free reads
   - Thread-safe writes

---

## Version History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0.0 | 2025-09-29 | Liam Helmer | Initial backend specification |
| 1.1.0 | 2025-09-29 | Liam Helmer | Added JWT/JWKS authentication, authorization, and claims extraction |