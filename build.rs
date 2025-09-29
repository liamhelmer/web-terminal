// Build script for web-terminal
// Per spec-kit/003-backend-spec.md and spec-kit/009-deployment-spec.md
//
// This script:
// 1. Builds frontend assets during Rust compilation
// 2. Embeds static assets into binary (optional)
// 3. Generates build metadata

use std::env;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=frontend/");
    println!("cargo:rerun-if-changed=build.rs");

    // Get build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    println!("cargo:warning=Building with profile: {}", profile);

    // Build frontend if in release mode
    if profile == "release" {
        build_frontend();
    } else {
        println!("cargo:warning=Skipping frontend build in debug mode (use 'pnpm run build' manually)");
    }

    // Generate build metadata
    generate_build_metadata();
}

fn build_frontend() {
    println!("cargo:warning=Building frontend...");

    // Check if pnpm is installed
    let pnpm_check = Command::new("pnpm")
        .arg("--version")
        .output();

    if pnpm_check.is_err() {
        println!("cargo:warning=pnpm not found, skipping frontend build");
        println!("cargo:warning=Install pnpm: npm install -g pnpm@8.15.0");
        return;
    }

    // Install frontend dependencies
    let install = Command::new("pnpm")
        .arg("install")
        .arg("--frozen-lockfile")
        .current_dir("frontend")
        .status()
        .expect("Failed to install frontend dependencies");

    if !install.success() {
        panic!("Frontend dependency installation failed");
    }

    // Build frontend
    let build = Command::new("pnpm")
        .arg("run")
        .arg("build")
        .current_dir("frontend")
        .status()
        .expect("Failed to build frontend");

    if !build.success() {
        panic!("Frontend build failed");
    }

    println!("cargo:warning=Frontend build completed successfully");
}

fn generate_build_metadata() {
    // Git commit hash
    if let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
    {
        if output.status.success() {
            let commit = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_COMMIT={}", commit);
        }
    }

    // Git branch
    if let Ok(output) = Command::new("git")
        .args(&["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
    {
        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_BRANCH={}", branch);
        }
    }

    // Build timestamp
    let timestamp = chrono::Utc::now().to_rfc3339();
    println!("cargo:rustc-env=BUILD_TIMESTAMP={}", timestamp);

    // Build profile
    let profile = env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
}