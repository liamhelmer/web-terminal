// Authentication service (EXTERNAL JWT VALIDATION ONLY)
// Per spec-kit/011-authentication-spec.md: External JWT providers only
// NO TOKEN GENERATION - tokens are issued by external identity providers

// This file is intentionally minimal - all JWT validation logic
// is in jwt_validator.rs which validates tokens from external providers

// Re-export the main authentication types from jwt_validator
pub use crate::security::jwt_validator::{Claims, JwtValidator, ValidatedToken, ValidationError};

// Note: This project does NOT generate or sign JWT tokens.
// All authentication is handled by external identity providers:
// - Backstage
// - Auth0
// - Okta
// - Keycloak
// - Custom OIDC providers
//
// The server only VALIDATES tokens using JWKS (public keys from external providers)
