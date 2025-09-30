// Security and sandboxing module
// Per spec-kit/003-backend-spec.md section 2.6
// Per 011-authentication-spec.md section 2

pub mod auth;
pub mod authorization;
pub mod jwks_client;
pub mod jwt_validator;

// External JWT validation only - NO internal token generation
pub use auth::{Claims, JwtValidator, ValidatedToken, ValidationError};
pub use authorization::{
    AuthorizationError, AuthorizationService, Permission, PermissionRules, Role,
};
pub use jwks_client::{JwksClient, JwksError, JwksProvider, JsonWebKey};
pub use jwt_validator::{UserSignInContext};