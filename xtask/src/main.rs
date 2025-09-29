// xtask - Task runner for web-terminal
// Per spec-kit/009-deployment-spec.md
//
// Usage: cargo xtask <command>
// Available commands: build, test, ci, clean, docker

use clap::{Parser, Subcommand};
use std::process::{Command, Stdio};
use anyhow::{Result, Context};

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Task runner for web-terminal project")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build the entire project (backend + frontend)
    Build {
        /// Build in release mode
        #[arg(long)]
        release: bool,
    },

    /// Run all tests (unit + integration + e2e)
    Test {
        /// Run only unit tests
        #[arg(long)]
        unit: bool,

        /// Run only integration tests
        #[arg(long)]
        integration: bool,

        /// Run only e2e tests
        #[arg(long)]
        e2e: bool,
    },

    /// Run CI checks (fmt, clippy, test, coverage)
    Ci,

    /// Clean build artifacts
    Clean,

    /// Build Docker image
    Docker {
        /// Docker image tag
        #[arg(short, long, default_value = "web-terminal:latest")]
        tag: String,

        /// Build multi-arch images (amd64, arm64)
        #[arg(long)]
        multi_arch: bool,
    },

    /// Development mode (watch and rebuild)
    Dev,

    /// Format code (rust + typescript)
    Fmt,

    /// Run linters (clippy + eslint)
    Lint {
        /// Fix lint errors automatically
        #[arg(long)]
        fix: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { release } => build(release),
        Commands::Test { unit, integration, e2e } => test(unit, integration, e2e),
        Commands::Ci => ci(),
        Commands::Clean => clean(),
        Commands::Docker { tag, multi_arch } => docker(&tag, multi_arch),
        Commands::Dev => dev(),
        Commands::Fmt => fmt(),
        Commands::Lint { fix } => lint(fix),
    }
}

fn build(release: bool) -> Result<()> {
    println!("🔨 Building web-terminal...");

    // Build frontend first
    println!("📦 Building frontend...");
    run_command("pnpm", &["install"], Some("frontend"))?;
    run_command("pnpm", &["run", "build"], Some("frontend"))?;

    // Build backend
    println!("🦀 Building backend...");
    let mut args = vec!["build"];
    if release {
        args.push("--release");
    }
    run_command("cargo", &args, None)?;

    println!("✅ Build complete!");
    Ok(())
}

fn test(unit: bool, integration: bool, e2e: bool) -> Result<()> {
    println!("🧪 Running tests...");

    // If no flags, run all tests
    let run_all = !unit && !integration && !e2e;

    if run_all || unit {
        println!("📋 Running Rust unit tests...");
        run_command("cargo", &["test", "--lib"], None)?;

        println!("📋 Running frontend unit tests...");
        run_command("pnpm", &["run", "test"], Some("frontend"))?;
    }

    if run_all || integration {
        println!("🔗 Running integration tests...");
        run_command("cargo", &["test", "--test", "*"], None)?;
    }

    if run_all || e2e {
        println!("🌐 Running E2E tests...");
        run_command("pnpm", &["run", "test:e2e"], Some("frontend"))?;
    }

    println!("✅ All tests passed!");
    Ok(())
}

fn ci() -> Result<()> {
    println!("🚀 Running CI checks...");

    // Format check
    println!("📝 Checking formatting...");
    run_command("cargo", &["fmt", "--", "--check"], None)?;
    run_command("pnpm", &["run", "format:check"], Some("frontend"))?;

    // Linting
    println!("🔍 Running linters...");
    run_command("cargo", &["clippy", "--all-targets", "--all-features", "--", "-D", "warnings"], None)?;
    run_command("pnpm", &["run", "lint"], Some("frontend"))?;

    // Type checking
    println!("🔎 Type checking...");
    run_command("pnpm", &["run", "typecheck"], Some("frontend"))?;

    // Tests
    println!("🧪 Running tests...");
    test(false, false, false)?;

    // Security audit
    println!("🔒 Security audit...");
    run_command("cargo", &["audit"], None).ok(); // Don't fail on advisories

    println!("✅ All CI checks passed!");
    Ok(())
}

fn clean() -> Result<()> {
    println!("🧹 Cleaning build artifacts...");

    run_command("cargo", &["clean"], None)?;
    run_command("rm", &["-rf", "frontend/dist"], None)?;
    run_command("rm", &["-rf", "frontend/node_modules"], None)?;

    println!("✅ Clean complete!");
    Ok(())
}

fn docker(tag: &str, multi_arch: bool) -> Result<()> {
    println!("🐳 Building Docker image: {}", tag);

    let mut args = vec!["build", "-t", tag, "."];

    if multi_arch {
        println!("📦 Building multi-arch image (amd64, arm64)...");
        args = vec![
            "buildx", "build",
            "--platform", "linux/amd64,linux/arm64",
            "-t", tag,
            "--push",
            ".",
        ];
    }

    run_command("docker", &args, None)?;

    println!("✅ Docker image built: {}", tag);
    Ok(())
}

fn dev() -> Result<()> {
    println!("👨‍💻 Starting development mode...");
    println!("Backend: cargo watch -x run");
    println!("Frontend: pnpm run dev");
    println!("\nRun these commands in separate terminals:");
    println!("  Terminal 1: cargo watch -x run");
    println!("  Terminal 2: cd frontend && pnpm run dev");

    Ok(())
}

fn fmt() -> Result<()> {
    println!("📝 Formatting code...");

    run_command("cargo", &["fmt"], None)?;
    run_command("pnpm", &["run", "format"], Some("frontend"))?;

    println!("✅ Code formatted!");
    Ok(())
}

fn lint(fix: bool) -> Result<()> {
    println!("🔍 Running linters...");

    let mut cargo_args = vec!["clippy", "--all-targets", "--all-features"];
    if fix {
        cargo_args.push("--fix");
        cargo_args.push("--allow-dirty");
    }
    run_command("cargo", &cargo_args, None)?;

    let mut frontend_args = vec!["run", "lint"];
    if fix {
        frontend_args = vec!["run", "lint:fix"];
    }
    run_command("pnpm", &frontend_args, Some("frontend"))?;

    println!("✅ Linting complete!");
    Ok(())
}

fn run_command(cmd: &str, args: &[&str], working_dir: Option<&str>) -> Result<()> {
    let mut command = Command::new(cmd);
    command.args(args);

    if let Some(dir) = working_dir {
        command.current_dir(dir);
    }

    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = command
        .status()
        .with_context(|| format!("Failed to execute: {} {}", cmd, args.join(" ")))?;

    if !status.success() {
        anyhow::bail!("Command failed: {} {}", cmd, args.join(" "));
    }

    Ok(())
}