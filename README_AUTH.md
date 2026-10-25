# Authentication and authorization

See [README](README.md) for setup and deployment. Commands run from the repository root.

## Create an administrator

```bash
cargo run --locked --bin create_admin -- admin admin@example.com
# Regular user:
cargo run --locked --bin create_admin -- editor editor@example.com user
```

Both server and CLI apply migrations and use the same validation service.
Passwords are entered via a hidden prompt; automation can explicitly use
`--password-stdin` with input from a secret manager. Passwords in argv are no longer
accepted. Usernames require 3–32 ASCII letters/digits/underscores; passwords require
12–128 characters (at most 1024 UTF-8 bytes); emails have basic syntax and length checks.
Email ownership is **not verified**.

## Routes and response contracts

| Route | Access / behavior |
| --- | --- |
| `GET /login`, `GET /register` | Public forms; already authenticated users receive an HTTP redirect |
| `POST /login`, `POST /register` | Exact `Origin` must equal configured `APP_ORIGIN` |
| `POST /logout` | Same-origin only; deletes the current database session and cookie |
| `GET /logout` | Method not allowed; never mutates a session |
| `GET /profile`, `GET /dashboard` | Active account and unexpired session required |
| `GET/POST /api/user-data` | Same session requirement; data is always scoped to authenticated user ID |
| `GET /admin`, `POST /admin/users/role`, `POST /admin/users/status` | Current database role must be admin |

Unauthenticated page navigation redirects to `/login`. Plain API requests receive
401; HTMX gets `HX-Redirect: /login`. Authorization failures receive 403. Database
outages receive 503 instead of silently deleting a valid login cookie.

Admin actions redirect back to the table after success. The server, not just the
UI, rejects changing your own role/status. Role changes take effect on the next
request because roles are not trusted from a cookie. The example has no account
deletion or password-reset flow.

## Security invariants

- Argon2id hashes use random salts. Hash/verify run off Tokio executor threads.
  A process-wide semaphore permits four password jobs, retained through job
  completion even when the initiating request is cancelled.
- Login and registration share a single-process 30-attempt/minute budget. Exhaustion
  returns 429 with `Retry-After`. This is a small-site baseline, not distributed or
  per-user brute-force protection; deploy per-client rate limiting at your proxy.
- Session IDs are random UUIDv4 values, stored in the database; cookies contain
  no trusted role or user-supplied identity. The lifetime is seven days.
- Cookies have `HttpOnly`, `SameSite=Lax`, `Path=/`, and `Secure` when `APP_ORIGIN`
  uses HTTPS. HTTP is intended only for explicit local development.
- Deactivation and deletion of all existing sessions are transactional. Session
  creation locks/checks the active account in the same transaction, preventing a
  racing login from surviving deactivation and later reactivation. A request
  already authorized before deactivation may finish.
- Expiration/activity are checked on every authenticated request. Startup and a
  periodic maintenance task remove expired sessions from the database.
- All unsafe HTTP methods require an exact configured `Origin`, including login
  and registration. Missing, `null`, and foreign origins fail closed. There is no
  permissive CORS policy and neither Host nor proxy headers determine trusted origin.
- Templates escape user content. CSP blocks inline script/eval and third-party
  scripts; HTMX is local with script evaluation disabled. `HttpOnly` alone would
  not make an XSS harmless.

## Testing

`server/src/tests.rs` exercises real routers, migrations, cookies and templates:
registration, duplicate conflicts, login/logout, CSRF rejection, XSS escaping,
role/status changes, revocation, concurrent writes, limits and security headers.
The PostgreSQL contract is explicit/ignored locally and runs against a disposable
database in CI. Browser tests cover normal and no-JavaScript forms.

Before exposing real accounts: use HTTPS, persistent storage and tested backups,
limit requests at the trusted proxy, decide whether to allow public registration,
and implement email verification/recovery if your use case requires them.
