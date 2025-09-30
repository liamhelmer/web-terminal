// JWT authentication middleware (JWKS-based)
// Per spec-kit/011-authentication-spec.md section "Authentication Flow"
// Per spec-kit/003-backend-spec.md section 4.1

use std::future::{ready, Ready};
use std::sync::Arc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;

use crate::security::jwt_validator::{Claims, JwtValidator, ValidationError};
use crate::session::UserId;

/// User context extracted from validated JWT
/// Per spec-kit/011-authentication-spec.md: "Authentication Flow"
#[derive(Debug, Clone)]
pub struct UserContext {
    /// User identifier from JWT sub claim
    pub user_id: UserId,
    /// User email (if available from claims)
    pub email: Option<String>,
    /// Group memberships from Backstage ent claim
    pub groups: Vec<String>,
    /// Identity provider (from iss claim)
    pub provider: String,
    /// Raw JWT claims for additional data
    pub claims: Claims,
}

impl UserContext {
    /// Create from validated JWT claims
    /// Per spec-kit/011-authentication-spec.md: Claims extraction
    pub fn from_claims(claims: Claims, provider: String) -> Self {
        let user_id = UserId::new(claims.sub.clone());

        // Extract groups from Backstage ent claim
        let groups = claims
            .ent
            .clone()
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.starts_with("group:"))
            .collect();

        // Extract email from claims
        let email = claims.email.clone();

        Self {
            user_id,
            email,
            groups,
            provider,
            claims,
        }
    }

    /// Extract UserContext from actix request extensions
    pub fn from_request(req: &actix_web::HttpRequest) -> Option<Self> {
        req.extensions().get::<UserContext>().cloned()
    }
}

/// JWT authentication middleware using JWKS validation
/// Per spec-kit/011-authentication-spec.md: "HTTP Request Authentication"
#[derive(Clone)]
pub struct JwtAuthMiddleware {
    validator: Arc<JwtValidator>,
}

impl JwtAuthMiddleware {
    /// Create new JWT authentication middleware with JWKS validator
    pub fn new(validator: Arc<JwtValidator>) -> Self {
        Self { validator }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddlewareService<S>;
    type Future = Ready<std::result::Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddlewareService {
            service: Arc::new(service),
            validator: self.validator.clone(),
        }))
    }
}

pub struct JwtAuthMiddlewareService<S> {
    service: Arc<S>,
    validator: Arc<JwtValidator>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let validator = self.validator.clone();
        let service = self.service.clone();

        Box::pin(async move {
            // Extract Authorization header
            let auth_header = req
                .headers()
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.strip_prefix("Bearer "))
                .ok_or_else(|| {
                    actix_web::error::ErrorUnauthorized("Missing or invalid Authorization header")
                })?;

            // Validate JWT token using JWKS
            let validated = validator.validate(auth_header).await.map_err(|e| {
                tracing::warn!("JWT validation failed: {}", e);
                actix_web::error::ErrorUnauthorized(format!("Invalid token: {}", e))
            })?;

            // Create user context
            let user_context = UserContext::from_claims(validated.claims, validated.provider);

            tracing::debug!(
                "Authenticated user: {} (provider: {})",
                user_context.user_id,
                user_context.provider
            );

            // Attach user context to request
            req.extensions_mut().insert(user_context);

            // Continue to next service
            service.call(req).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_context_from_claims() {
        let claims = Claims {
            sub: "user:default/testuser".to_string(),
            iss: "https://example.com".to_string(),
            aud: Some("web-terminal".to_string()),
            exp: 1234567890,
            nbf: None,
            iat: Some(1234567800),
            email: Some("test@example.com".to_string()),
            groups: Some(vec!["group:default/admins".to_string()]),
            ent: Some(vec![
                "group:default/admins".to_string(),
                "user:default/testuser".to_string(),
            ]),
            additional: Default::default(),
        };

        let user_ctx = UserContext::from_claims(claims, "backstage".to_string());

        assert_eq!(user_ctx.user_id.as_ref(), "user:default/testuser");
        assert_eq!(user_ctx.email, Some("test@example.com".to_string()));
        assert_eq!(user_ctx.groups, vec!["group:default/admins"]);
        assert_eq!(user_ctx.provider, "backstage");
    }
}
