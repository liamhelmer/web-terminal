// Integration tests entry point
// Per spec-kit/008-testing-spec.md

mod integration;

// Re-export tests for discoverability
pub use integration::*;