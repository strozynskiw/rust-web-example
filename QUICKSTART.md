# Quick start

Run all commands from the repository root. Install a current stable Rust toolchain.

```bash
cp .env.example .env
cargo run --locked --bin web-template
```

Open **http://localhost:3000**. SQLite is created and migrations run automatically.
Use **Register** to create a regular account. To create an administrator, open a
second terminal in the root and run:

```bash
cargo run --locked --bin create_admin -- admin admin@example.com
```

The CLI asks for a password without echoing it. Passwords require 12–128 characters.
Do not put passwords in command arguments.

CSS and HTMX are checked in under `static/`, so Node and a CDN are not required to
run the example. After editing template classes or `assets/app.css`, regenerate:

```bash
npm ci
npm run build
```

## PostgreSQL (optional)

1. Set a unique `POSTGRES_PASSWORD` in `.env`.
2. Run `docker compose up -d postgres`.
3. Set `DATABASE_URL` to `postgresql://web-template:YOUR_URL_ENCODED_PASSWORD@localhost:5432/web-template`.
4. Restart `cargo run --locked --bin web-template` from the root.

The Compose database port is accessible only on loopback. Do not expose a database
with development credentials publicly. `docker compose down` preserves its volume;
`docker compose down -v` **deletes all database data**.

## Troubleshooting

- **Templates not found:** run from the repository root, not `server/`.
- **403 on POST:** `APP_ORIGIN` must exactly match the browser origin (scheme, hostname,
  port, no trailing slash). `localhost` and `127.0.0.1` are different origins.
- **Port busy:** change both `PORT` and `APP_ORIGIN` in `.env`.
- **429:** the example limits authentication attempts and concurrent password work;
  wait for `Retry-After` before retrying.
- **Database error:** check the URL, permissions and logs; do not delete an existing
  database or edit already-applied migrations to work around an error.

See [README](README.md) for tests, architecture, customization and HTTPS deployment.
