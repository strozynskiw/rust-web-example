//! Rust Web
//!
//! A web application built with:
//! - Axum for HTTP handling
//! - Tera for templating
//! - PostgreSQL for storage
//! - HTMX for dynamic interactions
//! - Tailwind CSS for styling

use axum::http::HeaderValue;
use axum::{
    Router,
    extract::{Extension, Request},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use std::sync::Arc;
use tera::Tera;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_cookies::CookieManagerLayer;
use tower_http::services::ServeDir;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use rust_web_common::db;

mod api;
mod site;
mod storage;

/// Cookie name used to identify users across sessions.
const USER_ID_COOKIE: &str = "user_id";

/// Default server port.
const DEFAULT_SERVER_PORT: u16 = 3000;

/// Represents an authenticated user identified by a UUID.
#[derive(Clone, Debug)]
pub struct User {
    pub id: Uuid,
}

/// Shared database connection pool (supports both SQLite and PostgreSQL).
pub type SharedClient = rust_web_common::db::DatabasePool;

/// Thread-safe shared Tera template engine with hot-reloading support.
pub type SharedTera = Arc<RwLock<Tera>>;

/// Initialize the Tera template engine.
async fn setup_tera() -> anyhow::Result<SharedTera> {
    let tera = Tera::new("templates/**/*")?;
    info!(
        template_count = tera.get_template_names().count(),
        "Templates loaded"
    );
    Ok(Arc::new(RwLock::new(tera)))
}

/// Initialize the database connection pool and run migrations.
async fn setup_database() -> anyhow::Result<SharedClient> {
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = db::create_pool(&database_url).await?;

    // Run migrations based on database type
    match &pool {
        rust_web_common::db::DatabasePool::Sqlite(sqlite_pool) => {
            sqlx::migrate!("./migrations").run(sqlite_pool).await?;
        }
        rust_web_common::db::DatabasePool::Postgres(postgres_pool) => {
            sqlx::migrate!("./migrations").run(postgres_pool).await?;
        }
    }

    Ok(pool)
}

/// Initialize the tracing subscriber for structured logging.
fn setup_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_web=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

/// Build the application router with all routes and middleware.
fn build_router(tera: SharedTera, db: SharedClient) -> Router {
    Router::new()
        // Page routes
        .route("/", get(site::index))
        // API routes for HTMX
        .route("/api/example/refresh", get(api::example_refresh))
        .route("/api/example/partial", get(api::example_partial))
        .route("/api/user-data", get(api::get_user_data))
        .route("/api/user-data", post(api::save_user_data))
        // Static files with cache headers
        .nest_service(
            "/static",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=31536000, immutable"),
                ))
                .service(ServeDir::new("static")),
        )
        // Fallback for 404
        .fallback(site::not_found)
        // Middleware
        .layer(middleware::from_fn(tera_reload_middleware))
        .layer(Extension(tera))
        .layer(Extension(db))
        .layer(middleware::from_fn(user_identity_middleware))
        .layer(CookieManagerLayer::new())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize logging
    setup_tracing();

    info!("Starting server initialization");

    let tera = setup_tera().await?;
    let db = setup_database().await?;

    let app = build_router(tera, db);

    // Start the server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_SERVER_PORT);
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(address = %addr, "Server listening");

    axum::serve(listener, app).await?;

    Ok(())
}

/// Middleware that hot-reloads Tera templates in debug mode.
#[allow(unused_variables)]
async fn tera_reload_middleware(
    Extension(tera): Extension<SharedTera>,
    request: Request,
    next: Next,
) -> Response {
    #[cfg(debug_assertions)]
    {
        if let Err(e) = tera.write().await.full_reload() {
            tracing::warn!(error = %e, "Failed to reload templates");
        }
    }
    next.run(request).await
}

/// Middleware that ensures each user has a persistent UUID identifier.
async fn user_identity_middleware(mut request: Request, next: Next) -> Response {
    use tower_cookies::Cookies;

    let cookies = request.extensions().get::<Cookies>().cloned();
    if let Some(cookies) = cookies {
        let id = get_or_create_user_id(&cookies);
        request.extensions_mut().insert(User { id });
    }
    next.run(request).await
}

/// Retrieves an existing user ID from cookies or creates a new one.
fn get_or_create_user_id(cookies: &tower_cookies::Cookies) -> Uuid {
    use std::str::FromStr;
    use tower_cookies::Cookie;

    if let Some(cookie) = cookies.get(USER_ID_COOKIE)
        && let Ok(id) = Uuid::from_str(cookie.value()) {
            return id;
        }

    // Create new user ID and cookie
    let id = Uuid::new_v4();
    let cookie = Cookie::build((USER_ID_COOKIE.to_owned(), id.to_string()))
        .permanent()
        .path("/")
        .build();
    cookies.add(cookie);
    id
}
