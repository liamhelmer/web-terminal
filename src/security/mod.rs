// Security and sandboxing module
// Per spec-kit/003-backend-spec.md section 2.6

pub mod auth;

pub use auth::{AuthService, AuthToken, Credentials};
