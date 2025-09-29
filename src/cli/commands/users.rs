// User management commands
// Per spec-kit/005-cli-spec.md

use crate::cli::args::{UserCommands, UserListArgs, UserCreateArgs, UserDeleteArgs};
use anyhow::Result;

pub async fn execute(cmd: UserCommands) -> Result<()> {
    match cmd {
        UserCommands::List(args) => list(args).await,
        UserCommands::Create(args) => create(args).await,
        UserCommands::Delete(args) => delete(args).await,
    }
}

async fn list(args: UserListArgs) -> Result<()> {
    println!("👥 Users");

    if args.active {
        println!("  Filter: active only");
    }

    println!("\n⚠️  User management not yet implemented");

    Ok(())
}

async fn create(args: UserCreateArgs) -> Result<()> {
    println!("➕ Creating user: {}", args.username);

    if let Some(email) = &args.email {
        println!("  Email: {}", email);
    }

    if args.admin {
        println!("  Role: admin");
    }

    println!("\n⚠️  User creation not yet implemented");

    Ok(())
}

async fn delete(args: UserDeleteArgs) -> Result<()> {
    println!("🗑️  Deleting user: {}", args.username);

    if !args.force {
        println!("\n⚠️  Are you sure? Use --force to skip confirmation");
        return Ok(());
    }

    println!("\n⚠️  User deletion not yet implemented");

    Ok(())
}