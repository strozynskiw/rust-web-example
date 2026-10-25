# web-template

A small, server-rendered Rust website starter: **Axum 0.8, Tera 2, SQLx 0.9, SQLite/PostgreSQL, HTMX 2 and Tailwind CSS 4**. It includes public pages, registration/login, database-backed sessions, a profile, an administrator panel and a private JSON storage example.

This is a working example to adapt, not a complete identity platform or a promise of production security. Password reset, email verification, MFA, distributed abuse prevention and application-specific permissions are intentionally not implemented.

## Start locally

Install stable Rust (the Docker build uses Rust 1.99) and run **from the repository root**:

```sh
cp .env.example .env
cargo run --locked --bin web-template
```

Open **http://localhost:3000**. The server creates `data.db` and applies embedded migrations automatically. No database server, SQLx CLI, Node or frontend dev server is needed for this first run: built CSS and HTMX assets are committed.

Use that exact hostname, or change `APP_ORIGIN` to the origin you actually visit. Requests with a missing or different Origin are rejected for all state-changing routes, including login and registration.

### Create an administrator

```sh
cargo run --locked --bin create_admin -- admin admin@example.com
```

The CLI applies migrations and prompts for a password without echo. Usernames have 3–32 ASCII letters/digits/underscores; new passwords require 12–128 characters. To create a regular user, append `user`. For automation, append `--password-stdin` and pipe a secret from your secret manager; **never pass passwords as command arguments or put them in shell history**.

## Configuration

Environment variables override `.env`; `.env` is for local development and is not copied into the Docker image.

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | `sqlite:./data.db` | SQLite URL or `postgresql://USER:PASSWORD@HOST:5432/DATABASE` |
| `HOST` | `127.0.0.1` | Bind address; use `0.0.0.0` in a container |
| `PORT` | `3000` | TCP port; malformed values fail startup |
| `APP_ORIGIN` | `http://localhost:<PORT>` | Exact browser-facing origin, without trailing slash or path |
| `RUST_LOG` | `web_template=info,tower_http=info` | Tracing filter |
| `POSTGRES_PASSWORD` | none | Required by the optional PostgreSQL Compose service |
| `TEST_DATABASE_URL` | none | Disposable PostgreSQL DB for explicit integration tests only |

An HTTPS `APP_ORIGIN` enables `Secure` session cookies and HSTS. The server itself speaks HTTP: terminate TLS at a trusted reverse proxy. Do not use an HTTP origin for public deployment. Proxy headers are not trusted to choose the origin or cookie policy.

### PostgreSQL

The default `docker-compose.yml` starts **only a local development database**. Set a unique `POSTGRES_PASSWORD` in `.env`, then:

```sh
docker compose up -d
```

Set `DATABASE_URL` to `postgresql://web-template:YOUR_URL_ENCODED_PASSWORD@localhost:5432/web-template`, then start the app from the root. Percent-encode reserved characters in URL passwords. The port is bound to loopback only, data persists in `postgres_data`, and Compose fails if the password is absent. `docker compose down` preserves data; `down -v` permanently deletes it.

SQLite and PostgreSQL use the same portable migrations. JSON documents are stored as **TEXT**, not JSONB. Ordinary queries use runtime SQLx decoding; they are not compile-time-checked `query!` macros and do not need `cargo sqlx prepare`.

## Build and customize

```text
server/src/main.rs          startup, router, middleware composition
server/src/site.rs          page handlers and template context
server/src/api.rs           HTMX HTML fragments
server/src/auth.rs          accounts, password work, session lifecycle
server/src/auth_handlers.rs login/registration/logout HTTP handlers
server/src/middlewares.rs   session and administrator authorization
server/src/security.rs      same-origin policy, security headers, admission limit
server/src/storage.rs       private JSON example, optimistic concurrent updates
server/src/tests.rs         router and database regression tests
common/src/db.rs           URL parsing and connection pools
templates/base.html        shared layout and navigation
templates/pages/           public page and profile
templates/auth/            ordinary HTML authentication forms
templates/admin/           user management example
templates/partials/        escaped HTMX fragments
assets/app.css             Tailwind entrypoint and accessibility styles
static/                    committed browser assets
tests/e2e/                 real-browser smoke tests
```

1. Change the site name, metadata, navigation and footer in `templates/base.html`; edit `templates/pages/index.html` for your landing page. Keep a single page `h1`, real labels, keyboard focus and mobile layout.
2. Add a handler in `site.rs`, a Tera page and a route in `build_router`. Use `require_auth`/`require_admin` on the router containing protected routes, not just on navigation links.
3. Add domain data through **new** migrations. Never edit already-applied migrations: SQLx verifies their checksums. Back up data before deploying schema changes.
4. Rebuild assets after changing utility classes or CSS:

```sh
npm ci
npm run build
```

Node 22+ is only needed for asset development and browser tests. Tailwind scans templates and Rust fragments; keep complete utility class names in source. Commit `static/app.css`, `static/htmx.min.js`, `package-lock.json` and `Cargo.lock`. Assets are served locally, so the site does not depend on a public CDN. Third-party HTMX license notices ship alongside it.

Templates reload in debug builds; release builds load them at startup. Restart a release process after changing templates. Run from a directory containing `templates/` and `static/`, or deploy the image which sets the working directory.

## HTTP examples and security contract

- `GET /`, `/login`, `/register`: public pages; authenticated visitors to login/register are redirected.
- `GET /profile` (`/dashboard` alias): authenticated profile.
- `GET /admin`, `POST /admin/users/role`, `POST /admin/users/status`: administrator-only example. The server rejects self-demotion/deactivation. Use the CLI for account recovery.
- `POST /logout`: deletes the database session and clears the root-path cookie. GET is deliberately not a logout action.
- `GET /api/example/refresh`: full page rendering retaining the signed-in navigation.
- `GET /api/example/partial?counter=1`: server-rendered fragment with the next counter; no inline event handlers.
- `GET /api/user-data`, `POST /api/user-data` (`key`, `value`): authenticated, user-scoped JSON example. A JSON value is parsed if valid; otherwise it is stored as a string. Keys are 1–128 bytes, values at most 16 KiB. Output is HTML-escaped by Tera. Concurrent successful key updates do not overwrite unrelated keys; contention exhaustion returns an error rather than false success.
- `GET /healthz`: process liveness, **not** a database-readiness guarantee.

Normal forms/navigation work without JavaScript. With HTMX, examples replace fragments in place; without it, fragment endpoints open as standalone responses. HTMX authentication expiry sends `HX-Redirect`; direct unauthenticated API requests receive 401, and normal protected page visits redirect to login.

All unsafe requests must include `Origin: <APP_ORIGIN>`; this also applies to curl/tests and reverse proxies must preserve it. No CORS policy is enabled. Session cookies are HttpOnly, SameSite=Lax, scoped to `/` and expire after seven days. Deactivation atomically removes sessions; session creation and deactivation serialize on the user row. Expired or inactive sessions cannot authorize requests. Authorization does not cancel a request already authorized before a concurrent role/status change.

Argon2id runs on blocking workers, with four concurrent password jobs maximum. Login/registration share a small **single-process global budget of 30 attempts/minute**, not a distributed or per-IP rate limiter. Put per-client throttling at the trusted edge before public deployment. Logout is not rate-limited by this budget. Request bodies are limited to 32 KiB. CSP disallows inline scripts/styles and eval; add external files under `static/` instead. Personalized responses use `Cache-Control: no-store`.

## Verification

```sh
cargo check --workspace --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --locked
cargo fmt --all --check
cargo audit
npm ci
npm run build
npm audit
npx playwright install chromium
npm run test:e2e
```

SQLite tests use isolated in-memory databases. The PostgreSQL contract is ignored by the default suite on purpose; run it against an **empty disposable** database:

```sh
TEST_DATABASE_URL=postgresql://USER:PASSWORD@localhost:5432/DISPOSABLE_DB \
  cargo test --locked -p web-template --bin web-template postgres_storage_contract -- --ignored
```

That test inserts accounts and runs migrations; never point it at your development or production database. CI provisions its own PostgreSQL service and also builds the Docker image. Browser tests launch a separate server on port 3107 using SQLite in memory and stop it afterwards.

Optional `cargo make` shortcuts are documented in [README_MAKEFILE.md](README_MAKEFILE.md); Cargo commands above are the canonical path.

## Deploy with Docker

```sh
docker build -t web-template .
docker run --rm --name web-template -p 127.0.0.1:3000:3000 \
  -e APP_ORIGIN=https://your-domain.example \
  -v web-template-data:/data web-template
```

The image runs as a non-root user and defaults to SQLite at `/data/data.db`. The named volume is essential for persistence. A bind-mounted directory must be writable by UID 10001. To use PostgreSQL, inject `DATABASE_URL` securely instead; never bake credentials into the image. To create the first admin in a running container:

```sh
docker exec -it web-template ./create_admin admin admin@example.com
```

Before going public: configure HTTPS and the exact origin, edge rate limits/timeouts, regular backups and restore tests, a database least-privilege user, logging/monitoring and schema rollout strategy. Do not expose the database port publicly. Automatic startup migrations are convenient for this starter; coordinate them before running multiple app replicas. There is no built-in password-reset/email-delivery flow. Session cleanup occurs periodically while the server runs. Handle graceful stop via SIGTERM/SIGINT and allow in-flight requests to finish.
