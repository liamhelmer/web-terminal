// CORS middleware configuration
// Per spec-kit/002-architecture.md Layer 1: Network Security

use actix_cors::Cors;
use actix_web::http::header;

/// CORS configuration
/// Per spec-kit/002-architecture.md: CORS policy for cross-origin requests
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins (e.g., ["https://example.com", "http://localhost:3000"])
    /// Use "*" to allow all origins (NOT recommended for production)
    pub allowed_origins: Vec<String>,

    /// Allowed HTTP methods
    pub allowed_methods: Vec<String>,

    /// Allowed headers
    pub allowed_headers: Vec<String>,

    /// Max age for preflight cache (seconds)
    pub max_age: usize,

    /// Allow credentials (cookies, authorization headers)
    pub supports_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["Authorization".to_string(), "Content-Type".to_string()],
            max_age: 3600,
            supports_credentials: true,
        }
    }
}

impl CorsConfig {
    /// Build Actix-Web CORS middleware from configuration
    /// Per spec-kit/002-architecture.md: Restrict cross-origin requests
    pub fn build(&self) -> Cors {
        let mut cors = Cors::default();

        // Configure allowed origins
        if self.allowed_origins.contains(&"*".to_string()) {
            cors = cors.allow_any_origin();
            tracing::warn!("CORS: Allowing all origins (*). NOT recommended for production!");
        } else {
            for origin in &self.allowed_origins {
                cors = cors.allowed_origin(origin);
            }
            tracing::info!("CORS: Allowed origins: {:?}", self.allowed_origins);
        }

        // Configure allowed methods
        for method in &self.allowed_methods {
            cors = cors.allowed_methods(vec![method.as_str()]);
        }
        tracing::info!("CORS: Allowed methods: {:?}", self.allowed_methods);

        // Configure allowed headers
        for header_name in &self.allowed_headers {
            cors = cors.allowed_header(header_name.as_str());
        }
        tracing::info!("CORS: Allowed headers: {:?}", self.allowed_headers);

        // Expose headers that client can access
        cors = cors.expose_headers(vec![
            header::CONTENT_TYPE.as_str(),
            header::AUTHORIZATION.as_str(),
        ]);

        // Set max age for preflight cache
        cors = cors.max_age(self.max_age);

        // Support credentials (cookies, authorization headers)
        if self.supports_credentials {
            cors = cors.supports_credentials();
            tracing::info!("CORS: Credentials support enabled");
        }

        cors
    }

    /// Production-ready CORS configuration
    /// Per spec-kit/009-deployment-spec.md: Security best practices
    pub fn production(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins,
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec!["Authorization".to_string(), "Content-Type".to_string()],
            max_age: 3600,
            supports_credentials: true,
        }
    }

    /// Development CORS configuration (permissive)
    pub fn development() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
                "HEAD".to_string(),
                "PATCH".to_string(),
            ],
            allowed_headers: vec!["*".to_string()],
            max_age: 3600,
            supports_credentials: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_config_default() {
        let config = CorsConfig::default();
        assert_eq!(config.allowed_origins, vec!["*"]);
        assert_eq!(config.max_age, 3600);
        assert!(config.supports_credentials);
    }

    #[test]
    fn test_cors_config_production() {
        let origins = vec!["https://example.com".to_string()];
        let config = CorsConfig::production(origins.clone());
        assert_eq!(config.allowed_origins, origins);
        assert!(config.supports_credentials);
    }

    #[test]
    fn test_cors_config_development() {
        let config = CorsConfig::development();
        assert_eq!(config.allowed_origins, vec!["*"]);
    }

    #[test]
    fn test_cors_build() {
        let config = CorsConfig::default();
        let _cors = config.build();
        // If this compiles and runs, CORS middleware is built successfully
    }
}
