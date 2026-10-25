# Optional cargo-make shortcuts

Plain Cargo commands in [README](README.md) are the primary interface. If desired,
install `cargo-make` with `cargo install cargo-make --locked`, then run tasks from
the repository root. No task overrides `DATABASE_URL`; the server and CLI read `.env`.

| Task | Purpose |
| --- | --- |
| `cargo make run` / `dev` | Start `web-template` with the checked-in Cargo lockfile |
| `cargo make check` | Workspace compiler check |
| `cargo make build` / `build-release` | Debug/release workspace build |
| `cargo make fmt` / `fmt-check` | Format/check Rust |
| `cargo make clippy` | All targets/features, warnings denied |
| `cargo make test` | Workspace tests (SQLite, no service required) |
| `cargo make test-postgres` | Explicit ignored PostgreSQL contract; requires disposable `TEST_DATABASE_URL` |
| `cargo make assets` | Rebuild local CSS and HTMX after `npm ci` |
| `cargo make test-browser` | Playwright Chromium tests after `npx playwright install chromium` |
| `cargo make db-setup` / `db-stop` / `db-logs` | Optional local PostgreSQL Compose service |
| `cargo make docker-build` / `docker-run` | Container build / local HTTP demo with persistent SQLite volume |
| `cargo make ci` / `ci-full` | Local Rust checks / release plus browser checks |

Create users without putting passwords in arguments:

```bash
cargo make create-admin admin admin@example.com
cargo make create-user editor editor@example.com
```

Both tasks ask for a hidden password. The direct, equivalent commands are:

```bash
cargo run --locked --bin create_admin -- admin admin@example.com
cargo run --locked --bin create_admin -- editor editor@example.com user
```

Startup applies embedded migrations. No SQLx offline preparation is needed because
queries use SQLx's runtime query API. Destructive database-reset/fresh-start shortcuts
were removed; deleting a database/volume should be an explicit, backed-up decision.
