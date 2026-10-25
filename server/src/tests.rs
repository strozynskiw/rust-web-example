//! Router-level regression tests use the real migrations, cookies and templates.
use super::*;
use auth::{AuthService, UserRole};
use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
    response::Response,
};
use tower::ServiceExt;

async fn database() -> SharedClient {
    let db = db::create_pool("sqlite::memory:").await.unwrap();
    migrate(&db).await;
    db
}

async fn migrate(db: &SharedClient) {
    match db {
        db::DatabasePool::Sqlite(pool) => sqlx::migrate!("./migrations").run(pool).await.unwrap(),
        db::DatabasePool::Postgres(pool) => sqlx::migrate!("./migrations").run(pool).await.unwrap(),
    }
}

fn app(db: SharedClient) -> Router {
    let mut tera = Tera::default();
    tera.load_from_glob(&format!("{}/../templates/**/*", env!("CARGO_MANIFEST_DIR")))
        .unwrap();
    build_router(
        AppState {
            db,
            tera: Arc::new(RwLock::new(tera)),
        },
        security::SecurityConfig::new("https://example.test").unwrap(),
    )
}

async fn request(
    app: &Router,
    method: &str,
    path: &str,
    body: &str,
    cookie: Option<&str>,
    origin: Option<&str>,
) -> Response {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/x-www-form-urlencoded");
    if let Some(cookie) = cookie {
        builder = builder.header("cookie", cookie);
    }
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    app.clone()
        .oneshot(builder.body(Body::from(body.to_owned())).unwrap())
        .await
        .unwrap()
}

async fn text(response: Response) -> String {
    String::from_utf8(
        to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap()
}

async fn user(db: &SharedClient, name: &str, role: UserRole) -> auth::User {
    AuthService::create_user(
        db,
        name,
        &format!("{name}@example.test"),
        "long-password-123",
        role,
    )
    .await
    .unwrap()
}

#[tokio::test]
async fn logout_failure_keeps_cookie_and_allows_retry() {
    let db = database().await;
    let person = user(&db, "logout_retry", UserRole::User).await;
    let session = AuthService::create_session(&db, &person.id).await.unwrap();
    let cookie = format!("session_id={}", session.id);
    let app = app(db.clone());
    let db::DatabasePool::Sqlite(pool) = &db else {
        unreachable!()
    };
    sqlx::query("CREATE TRIGGER fail_session_delete BEFORE DELETE ON sessions BEGIN SELECT RAISE(ABORT, 'injected delete failure'); END")
        .execute(pool).await.unwrap();
    let response = request(
        &app,
        "POST",
        "/logout",
        "",
        Some(&cookie),
        Some("https://example.test"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert!(!response.headers().contains_key("set-cookie"));
    assert!(
        AuthService::validate_session(&db, &session.id)
            .await
            .is_ok()
    );
    sqlx::query("DROP TRIGGER fail_session_delete")
        .execute(pool)
        .await
        .unwrap();
    let response = request(
        &app,
        "POST",
        "/logout",
        "",
        Some(&cookie),
        Some("https://example.test"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert!(
        response.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Max-Age=0")
    );
    assert!(
        AuthService::validate_session(&db, &session.id)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn security_forms_sessions_and_rendering() {
    security_contract(database().await).await;
}

async fn security_contract(db: SharedClient) {
    let app = app(db.clone());
    for origin in [None, Some("https://attacker.test"), Some("null")] {
        let response = request(&app, "POST", "/register", "", None, origin).await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert!(!response.headers().contains_key("set-cookie"));
    }
    let registration = "username=alice&email=alice%40example.test&password=long-password-123&password_confirm=long-password-123";
    let response = request(
        &app,
        "POST",
        "/register",
        registration,
        None,
        Some("https://example.test"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let header = response.headers()["set-cookie"].to_str().unwrap();
    for flag in ["Secure", "HttpOnly", "SameSite=Lax", "Path=/"] {
        assert!(header.contains(flag), "{header}");
    }
    let cookie = header.split(';').next().unwrap().to_owned();
    let session = cookie.split_once('=').unwrap().1;
    let alice = AuthService::validate_session(&db, session).await.unwrap();
    let duplicate = request(
        &app,
        "POST",
        "/register",
        registration,
        None,
        Some("https://example.test"),
    )
    .await;
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
    let attack = "key=%3Cimg%20src%3Dx%20onerror%3Dalert%281%29%3E&value=%3Cscript%3Ealert%281%29%3C%2Fscript%3E";
    let response = request(
        &app,
        "POST",
        "/api/user-data",
        attack,
        Some(&cookie),
        Some("https://example.test"),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let html = text(response).await;
    assert!(
        !html.contains("<img") && !html.contains("<script>"),
        "{html}"
    );
    assert!(html.contains("&lt;"), "{html}");
    let html = text(request(&app, "GET", "/api/user-data", "", Some(&cookie), None).await).await;
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;"));
    let home = request(&app, "GET", "/api/example/refresh", "", Some(&cookie), None).await;
    assert_eq!(home.status(), StatusCode::OK);
    assert!(text(home).await.contains("alice"));
    assert_eq!(
        request(&app, "GET", "/logout", "", Some(&cookie), None)
            .await
            .status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert!(AuthService::validate_session(&db, session).await.is_ok());
    assert_eq!(
        request(&app, "GET", "/admin", "", Some(&cookie), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    AuthService::toggle_user_status(&db, &alice.id, false)
        .await
        .unwrap();
    assert!(AuthService::validate_session(&db, session).await.is_err());
    AuthService::toggle_user_status(&db, &alice.id, true)
        .await
        .unwrap();
    assert!(
        AuthService::validate_session(&db, session).await.is_err(),
        "Reactivation must not resurrect sessions"
    );
    let login = request(
        &app,
        "POST",
        "/login",
        "username=alice&password=long-password-123",
        None,
        Some("https://example.test"),
    )
    .await;
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    let cookie = login.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let logout = request(
        &app,
        "POST",
        "/logout",
        "",
        Some(&cookie),
        Some("https://example.test"),
    )
    .await;
    assert_eq!(logout.status(), StatusCode::SEE_OTHER);
    assert!(
        logout.headers()["set-cookie"]
            .to_str()
            .unwrap()
            .contains("Path=/")
    );
    assert!(
        AuthService::validate_session(&db, cookie.split_once('=').unwrap().1)
            .await
            .is_err()
    );
}

async fn storage_contract(db: SharedClient) {
    let first = user(&db, "storage_first", UserRole::User).await;
    let second = user(&db, "storage_second", UserRole::User).await;
    let mut jobs = tokio::task::JoinSet::new();
    for n in 0..12 {
        let db = db.clone();
        let id = first.id.clone();
        jobs.spawn(async move {
            storage::set_user_data_key(&db, &id, &format!("key{n}"), &serde_json::json!(n)).await
        });
    }
    while let Some(result) = jobs.join_next().await {
        result.unwrap().unwrap();
    }
    let data = storage::load_user_data(&db, &first.id)
        .await
        .unwrap()
        .unwrap();
    for n in 0..12 {
        assert_eq!(data[format!("key{n}")], n);
    }
    assert!(
        storage::load_user_data(&db, &second.id)
            .await
            .unwrap()
            .is_none()
    );
    storage::store_user_data(&db, &first.id, &serde_json::json!([]))
        .await
        .unwrap();
    assert!(
        storage::set_user_data_key(&db, &first.id, "key", &serde_json::json!(1))
            .await
            .is_err()
    );
    let session = AuthService::create_session(&db, &first.id).await.unwrap();
    assert!(
        AuthService::validate_session(&db, &session.id)
            .await
            .is_ok()
    );
    AuthService::toggle_user_status(&db, &first.id, false)
        .await
        .unwrap();
    assert!(
        AuthService::validate_session(&db, &session.id)
            .await
            .is_err()
    );
    assert!(AuthService::create_session(&db, &first.id).await.is_err());
    AuthService::toggle_user_status(&db, &first.id, true)
        .await
        .unwrap();
    // Whichever operation wins the row lock, deactivation must terminate sessions.
    let (new_session, deactivated) = tokio::join!(
        AuthService::create_session(&db, &first.id),
        AuthService::toggle_user_status(&db, &first.id, false)
    );
    deactivated.unwrap();
    AuthService::toggle_user_status(&db, &first.id, true)
        .await
        .unwrap();
    if let Ok(session) = new_session {
        assert!(
            AuthService::validate_session(&db, &session.id)
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn sqlite_storage_contract() {
    storage_contract(database().await).await;
}

/// Run against an empty disposable database, never a developer/production DB.
#[tokio::test]
#[ignore = "requires TEST_DATABASE_URL pointing to a disposable PostgreSQL database"]
async fn postgres_storage_contract() {
    let url = std::env::var("TEST_DATABASE_URL").expect("TEST_DATABASE_URL required");
    assert!(url.starts_with("postgres"));
    let db = db::create_pool(&url).await.unwrap();
    migrate(&db).await;
    security_contract(db.clone()).await;
    admin_contract(db.clone()).await;
    storage_contract(db).await;
}

#[tokio::test]
async fn public_pages_and_unauthorized_contract() {
    let app = app(database().await);
    for path in [
        "/",
        "/login",
        "/register",
        "/healthz",
        "/api/example/partial?counter=1",
    ] {
        let response = request(&app, "GET", path, "", None, None).await;
        assert_eq!(response.status(), StatusCode::OK, "{path}");
    }
    assert_eq!(
        request(&app, "GET", "/unknown", "", None, None)
            .await
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, "GET", "/api/user-data", "", None, None)
            .await
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/user-data")
                .header("hx-request", "true")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["hx-redirect"], "/login");
}

#[tokio::test]
async fn admin_authorization_and_mutation_contract() {
    admin_contract(database().await).await;
}

async fn admin_contract(db: SharedClient) {
    let admin = user(&db, "admin_first", UserRole::Admin).await;
    let target = user(&db, "admin_target", UserRole::Admin).await;
    let session = AuthService::create_session(&db, &admin.id).await.unwrap();
    let target_session = AuthService::create_session(&db, &target.id).await.unwrap();
    let cookie = format!("session_id={}", session.id);
    let app = app(db.clone());
    assert_eq!(
        request(&app, "GET", "/admin", "", Some(&cookie), None)
            .await
            .status(),
        StatusCode::OK
    );
    for (path, body) in [
        (
            "/admin/users/role",
            format!("user_id={}&role=user", admin.id),
        ),
        (
            "/admin/users/status",
            format!("user_id={}&is_active=false", admin.id),
        ),
    ] {
        assert_eq!(
            request(
                &app,
                "POST",
                path,
                &body,
                Some(&cookie),
                Some("https://example.test")
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let role_body = format!("user_id={}&role=user", target.id);
    assert_eq!(
        request(
            &app,
            "POST",
            "/admin/users/role",
            &role_body,
            Some(&cookie),
            Some("https://attacker.test")
        )
        .await
        .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            "POST",
            "/admin/users/role",
            &role_body,
            Some(&cookie),
            Some("https://example.test")
        )
        .await
        .status(),
        StatusCode::SEE_OTHER
    );
    let target_cookie = format!("session_id={}", target_session.id);
    assert_eq!(
        request(&app, "GET", "/admin", "", Some(&target_cookie), None)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    let body = format!("user_id={}&is_active=false", target.id);
    assert_eq!(
        request(
            &app,
            "POST",
            "/admin/users/status",
            &body,
            Some(&cookie),
            Some("https://example.test")
        )
        .await
        .status(),
        StatusCode::SEE_OTHER
    );
    assert_eq!(
        request(&app, "GET", "/profile", "", Some(&target_cookie), None)
            .await
            .status(),
        StatusCode::SEE_OTHER
    );
    assert!(
        AuthService::validate_session(&db, &target_session.id)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn request_limits_and_security_headers() {
    let app = app(database().await);
    let home = request(&app, "GET", "/", "", None, None).await;
    assert_eq!(home.headers()["cache-control"], "no-store");
    assert!(
        home.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("script-src 'self'")
    );
    assert!(home.headers().contains_key("strict-transport-security"));
    let oversized = "x".repeat(33 * 1024);
    assert_eq!(
        request(
            &app,
            "POST",
            "/login",
            &oversized,
            None,
            Some("https://example.test")
        )
        .await
        .status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    // Malformed input consumes admission budget but cannot start expensive hashing.
    for _ in 0..29 {
        assert_eq!(
            request(
                &app,
                "POST",
                "/login",
                "",
                None,
                Some("https://example.test")
            )
            .await
            .status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let limited = request(
        &app,
        "POST",
        "/login",
        "",
        None,
        Some("https://example.test"),
    )
    .await;
    assert_eq!(limited.status(), StatusCode::TOO_MANY_REQUESTS);
    assert!(limited.headers().contains_key("retry-after"));
}

#[test]
fn validation_and_origin_config() {
    for origin in [
        "https://example.test/path",
        "https://example.test/",
        "null",
        "ftp://example.test",
        "https://user@example.test",
    ] {
        assert!(security::SecurityConfig::new(origin).is_err(), "{origin}");
    }
    assert!(
        !security::SecurityConfig::new("http://localhost:3000")
            .unwrap()
            .secure_cookies
    );
    for (name, email, password) in [
        ("", "a@b.test", "long-password-123"),
        ("ok_name", "invalid", "long-password-123"),
        ("ok_name", "a@b.test", "short"),
    ] {
        assert!(auth::validate_registration(name, email, password).is_err());
    }
}
