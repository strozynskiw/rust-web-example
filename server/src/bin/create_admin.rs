//! Create an administrator (default) or regular user without putting passwords in argv.
use std::io::{self, BufRead, IsTerminal};
use web_template_common::db;

#[path = "../auth.rs"]
#[allow(dead_code)]
mod auth;
use auth::{AuthService, UserRole};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        (2..=4).contains(&args.len()),
        "Usage: create_admin <username> <email> [admin|user] [--password-stdin]"
    );
    let stdin_password = args.last().is_some_and(|v| v == "--password-stdin");
    let positional = if stdin_password {
        &args[..args.len() - 1]
    } else {
        &args[..]
    };
    anyhow::ensure!((2..=3).contains(&positional.len()), "Invalid arguments");
    let role = UserRole::from_str(positional.get(2).map(String::as_str).unwrap_or("admin"))?;
    let password = if stdin_password {
        let mut line = String::new();
        io::stdin().lock().read_line(&mut line)?;
        line.trim_end_matches(['\r', '\n']).to_owned()
    } else {
        anyhow::ensure!(
            io::stdin().is_terminal(),
            "Use --password-stdin for non-interactive input"
        );
        let password = rpassword::prompt_password("Password (12–128 characters): ")?;
        let confirm = rpassword::prompt_password("Confirm password: ")?;
        anyhow::ensure!(password == confirm, "Passwords do not match");
        password
    };
    auth::validate_registration(&positional[0], &positional[1], &password)?;
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:./data.db".into());
    let pool = db::create_pool(&url).await?;
    match &pool {
        db::DatabasePool::Sqlite(pool) => sqlx::migrate!("./migrations").run(pool).await?,
        db::DatabasePool::Postgres(pool) => sqlx::migrate!("./migrations").run(pool).await?,
    }
    let user =
        AuthService::create_user(&pool, &positional[0], &positional[1], &password, role).await?;
    println!("Created {} '{}' ({})", user.role, user.username, user.id);
    Ok(())
}
