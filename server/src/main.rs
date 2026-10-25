//! web-template
//!
//! A web application built with:
//! - Axum for HTTP handling
//! - Tera for templating
//! - SQLite/PostgreSQL for storage (SQLite by default)
//! - HTMX for dynamic interactions
//! - Tailwind CSS for styling

use axum::http::HeaderValue;
use axum::{
    Router,
    extract::{Extension, Request},
    middleware::Next,
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

use web_template_common::db;

mod admin;
mod api;
mod auth;
mod auth_handlers;
mod error;
mod logging;
mod middlewares;
mod security;
mod site;
mod storage;
#[cfg(test)]
mod tests;

/// Default server port.
const DEFAULT_SERVER_PORT: u16 = 3000;

/// Shared database connection pool (supports both SQLite and PostgreSQL).
pub type SharedClient = web_template_common::db::DatabasePool;

/// Thread-safe shared Tera template engine with hot-reloading support.
pub type SharedTera = Arc<RwLock<Tera>>;

/// Application state shared across all handlers.
#[derive(Clone)]
pub struct AppState {
    pub tera: SharedTera,
    pub db: SharedClient,
}

/// Initialize the Tera template engine.
fn setup_tera() -> anyhow::Result<SharedTera> {
    let mut tera = Tera::default();
    tera.load_from_glob("templates/**/*")?;
    let template_count = tera.get_template_names().count();
    info!("   Found {} template(s)", template_count);
    Ok(Arc::new(RwLock::new(tera)))
}

/// Initialize the database connection pool and run migrations.
async fn setup_database() -> anyhow::Result<SharedClient> {
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        tracing::warn!("DATABASE_URL not set, using default: sqlite:./data.db");
        "sqlite:./data.db".to_string()
    });

    // Determine database type
    let db_type = if database_url.starts_with("sqlite") {
        "SQLite"
    } else if database_url.starts_with("postgres") {
        "PostgreSQL"
    } else {
        "Unknown"
    };

    info!("   Database type: {}", db_type);
    let pool = db::create_pool(&database_url).await?;
    info!("   Connection pool created");

    // Run migrations based on database type
    info!("   Running migrations...");
    match &pool {
        web_template_common::db::DatabasePool::Sqlite(sqlite_pool) => {
            sqlx::migrate!("./migrations").run(sqlite_pool).await?;
            info!("   SQLite migrations applied");
        }
        web_template_common::db::DatabasePool::Postgres(postgres_pool) => {
            sqlx::migrate!("./migrations").run(postgres_pool).await?;
            info!("   PostgreSQL migrations applied");
        }
    }

    Ok(pool)
}

/// Initialize the tracing subscriber for structured logging.
fn setup_tracing() {
    logging::setup();
}

/// Build the application router with all routes and middleware.
fn build_router(state: AppState, config: security::SecurityConfig) -> Router {
    // Public routes (no authentication required, but check if user is logged in)
    let public_routes = Router::new()
        .route("/", get(site::index_public))
        .route("/healthz", get(|| async { "ok" }))
        .route("/login", get(auth_handlers::login_page))
        .route("/register", get(auth_handlers::register_page))
        // Add optional auth middleware to inject user if logged in
        .layer(axum::middleware::from_fn(middlewares::optional_auth));

    // Auth action routes (login/logout/register actions)
    let auth_action_routes = Router::new()
        .route("/login", post(auth_handlers::login))
        .route("/register", post(auth_handlers::register))
        .layer(axum::middleware::from_fn(security::auth_capacity))
        .route("/logout", post(auth_handlers::logout));

    // HTMX example routes
    let example_routes = Router::new()
        .route("/api/example/refresh", get(site::index_public))
        .route("/api/example/partial", get(api::example_partial))
        .layer(axum::middleware::from_fn(middlewares::optional_auth));

    // Protected user routes (authentication required)
    let user_routes = Router::new()
        .route("/profile", get(site::user_profile))
        .route("/dashboard", get(site::user_profile))
        .route("/api/user-data", get(api::get_user_data))
        .route("/api/user-data", post(api::save_user_data))
        .layer(axum::middleware::from_fn(middlewares::require_auth));

    // Admin routes (authentication + admin role required)
    let admin_routes = Router::new()
        .route("/admin", get(admin::admin_dashboard))
        .route("/admin/users/role", post(admin::update_user_role))
        .route("/admin/users/status", post(admin::toggle_user_status))
        .layer(axum::middleware::from_fn(middlewares::require_admin))
        .layer(axum::middleware::from_fn(middlewares::require_auth));

    Router::new()
        .merge(public_routes)
        .merge(auth_action_routes)
        .merge(example_routes)
        .merge(user_routes)
        .merge(admin_routes)
        // Static files with cache headers
        .nest_service(
            "/static",
            ServiceBuilder::new()
                .layer(SetResponseHeaderLayer::overriding(
                    axum::http::header::CACHE_CONTROL,
                    HeaderValue::from_static("public, max-age=3600"),
                ))
                .service(ServeDir::new("static")),
        )
        // Fallback for 404
        .fallback(site::not_found)
        // Middleware
        .layer(axum::middleware::from_fn(tera_reload_middleware))
        .layer(Extension(state.db.clone()))
        .layer(Extension(state.tera.clone()))
        .with_state(state)
        .layer(CookieManagerLayer::new())
        .layer(axum::extract::DefaultBodyLimit::max(32 * 1024))
        .layer(axum::middleware::from_fn(security::same_origin))
        .layer(Extension(config))
        .layer(axum::middleware::from_fn(logging::access))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables first
    dotenvy::dotenv().ok();

    // Initialize logging early
    setup_tracing();

    let started = std::time::Instant::now();
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    info!(version = env!("CARGO_PKG_VERSION"), %profile, "Starting web-template");
    info!("Loading templates from templates/ and serving assets from static/");

    let tera = setup_tera()?;
    let db = setup_database().await?;
    start_session_maintenance(db.clone()).await?;
    let state = AppState { tera, db };

    // Start the server
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| DEFAULT_SERVER_PORT.to_string())
        .parse()?;
    let origin = std::env::var("APP_ORIGIN").unwrap_or_else(|_| format!("http://localhost:{port}"));
    let config = security::SecurityConfig::new(&origin)?;
    let app = build_router(state, config);
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let addr = format!("{host}:{port}");

    info!(bind = %addr, "Binding HTTP listener");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let bound = listener.local_addr()?;
    let local = std::net::SocketAddr::new(
        if bound.ip().is_unspecified() {
            if bound.is_ipv4() {
                std::net::Ipv4Addr::LOCALHOST.into()
            } else {
                std::net::Ipv6Addr::LOCALHOST.into()
            }
        } else {
            bound.ip()
        },
        bound.port(),
    );
    info!(bind = %bound, elapsed_ms = started.elapsed().as_millis(), "Server ready");
    info!("Local listener: http://{local}");
    info!("Application URL (APP_ORIGIN): {origin}");
    info!("Routes: {origin}/login | {origin}/register | {origin}/profile | {origin}/admin");
    info!("Health: http://{local}/healthz | Press Ctrl+C to stop");
    if bound.ip().is_unspecified() {
        info!(
            "Listening on all interfaces; use APP_ORIGIN for browser requests and form submissions"
        );
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
            info!("Shutdown requested; draining active connections");
        })
        .await?;

    info!("Server stopped");
    Ok(())
}

async fn start_session_maintenance(db: SharedClient) -> anyhow::Result<()> {
    auth::AuthService::cleanup_expired_sessions(&db).await?;
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(error) = auth::AuthService::cleanup_expired_sessions(&db).await {
                tracing::error!(%error, "Session cleanup failed");
            }
        }
    });
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
    if !request.uri().path().starts_with("/static/") && request.uri().path() != "/healthz" {
        let tera = tera.clone();
        match tokio::task::spawn_blocking(move || {
            let mut templates = tera.blocking_write();
            templates.full_reload()
        })
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(%error, "Failed to reload templates"),
            Err(error) => tracing::warn!(%error, "Template reload task failed"),
        }
    }
    next.run(request).await
}
