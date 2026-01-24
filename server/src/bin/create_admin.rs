//! CLI tool to create users (admin or regular)
//!
//! Usage: cargo run --bin create_admin -- <username> <email> <password> [role]
//!
//! Role options: 'admin' or 'user' (default: 'admin')

use web_template_common::db;

#[path = "../auth.rs"]
#[allow(dead_code)] // This binary only uses a subset of auth functions
mod auth;

use auth::{AuthService, UserRole};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 4 || args.len() > 5 {
        eprintln!("Usage: {} <username> <email> <password> [role]", args[0]);
        eprintln!("\nArguments:");
        eprintln!("  username  - User's username (alphanumeric and underscore)");
        eprintln!("  email     - User's email address");
        eprintln!("  password  - User's password (minimum 8 characters)");
        eprintln!("  role      - Optional: 'admin' or 'user' (default: 'admin')");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  # Create an admin user:");
        eprintln!("  cargo run --bin create_admin -- admin admin@example.com SecurePass123");
        eprintln!();
        eprintln!("  # Create a regular user:");
        eprintln!("  cargo run --bin create_admin -- john john@example.com SecurePass123 user");
        eprintln!();
        eprintln!("  # Create an admin user (explicit):");
        eprintln!("  cargo run --bin create_admin -- admin admin@example.com SecurePass123 admin");
        std::process::exit(1);
    }

    let username = &args[1];
    let email = &args[2];
    let password = &args[3];
    let role_str = if args.len() == 5 {
        &args[4]
    } else {
        "admin" // Default to admin for backward compatibility
    };

    // Parse role
    let role = match role_str.to_lowercase().as_str() {
        "admin" => UserRole::Admin,
        "user" => UserRole::User,
        _ => {
            eprintln!(
                "Error: Invalid role '{}'. Must be 'admin' or 'user'",
                role_str
            );
            std::process::exit(1);
        }
    };

    // Validate password strength
    if password.len() < 8 {
        eprintln!("Error: Password must be at least 8 characters long");
        std::process::exit(1);
    }

    // Validate username format
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        eprintln!("Error: Username can only contain letters, numbers, and underscores");
        std::process::exit(1);
    }

    // Validate email format (basic)
    if !email.contains('@') || !email.contains('.') {
        eprintln!("Error: Invalid email format");
        std::process::exit(1);
    }

    println!("Creating {} user...", role_str);
    println!("  Username: {}", username);
    println!("  Email: {}", email);
    println!("  Role: {}", role_str);
    println!();

    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        println!("⚠️  DATABASE_URL not set, using default: sqlite:./data.db");
        "sqlite:./data.db".to_string()
    });

    let pool = db::create_pool(&database_url).await?;

    match AuthService::create_user(&pool, username, email, password, role).await {
        Ok(user) => {
            println!("✓ User created successfully!");
            println!("  User ID: {}", user.id);
            println!("  Username: {}", user.username);
            println!("  Email: {}", user.email);
            println!("  Role: {}", user.role);
            println!();
            println!("You can now login at: http://localhost:3000/login");
        }
        Err(e) => {
            eprintln!("✗ Failed to create user: {}", e);
            if let auth::AuthError::DatabaseError(sqlx::Error::Database(ref db_err)) = e
                && db_err.message().contains("UNIQUE")
            {
                eprintln!("   Hint: Username or email already exists in database");
            }
            std::process::exit(1);
        }
    }

    Ok(())
}
