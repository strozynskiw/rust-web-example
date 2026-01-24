# Authentication & User Management

This application includes a comprehensive authentication system with role-based access control.

## Features

### Security Best Practices
- **Argon2id Password Hashing**: Industry-standard password hashing algorithm resistant to GPU and side-channel attacks
- **Constant-Time Comparison**: Protection against timing attacks during password verification
- **Secure Session Management**: Time-limited sessions with automatic cleanup
- **HTTP-Only Cookies**: Session cookies are HTTP-only to prevent XSS attacks
- **Role-Based Access Control**: Separate user and admin roles with middleware protection

### User Roles
- **User**: Standard user with access to user-specific features
- **Admin**: Full access including admin panel and user management

## Getting Started

### 1. Run Migrations

Migrations run automatically on server startup. The user management tables will be created:
- `users`: Stores user accounts with hashed passwords
- `sessions`: Manages active user sessions

### 2. Create an Admin User

Use the CLI tool to create your first admin user:

```bash
cd server
cargo run --bin create_admin -- admin admin@example.com YourSecurePassword123
```

Example output:
```
Creating admin user...
  Username: admin
  Email: admin@example.com

✓ Admin user created successfully!
  User ID: 550e8400-e29b-41d4-a716-446655440000
  Username: admin
  Email: admin@example.com
  Role: admin

You can now login at: http://localhost:3000/login
```

### 3. Start the Server

```bash
cd server
cargo run
```

### 4. Access the Application

- **Login Page**: http://localhost:3000/login
- **Register Page**: http://localhost:3000/register
- **Admin Panel**: http://localhost:3000/admin (requires admin role)

## Routes

### Public Routes (No Authentication)
- `GET /` - Home page
- `GET /login` - Login page
- `POST /login` - Login handler
- `GET /register` - Registration page
- `POST /register` - Registration handler
- `GET /logout` - Logout handler

### Protected Routes (Authentication Required)
- `GET /api/user-data` - Get user data
- `POST /api/user-data` - Save user data

### Admin Routes (Admin Role Required)
- `GET /admin` - Admin dashboard
- `POST /admin/users/role` - Update user role
- `POST /admin/users/status` - Toggle user active status

## Admin Panel

The admin panel provides:

- **User Statistics**: Total users, active users, admin count
- **User Management Table**: View all users with their details
- **Role Management**: Promote users to admin or demote to user
- **Status Management**: Activate or deactivate user accounts
- **User Information**: View username, email, role, status, and last login

### Admin Actions

**Promote/Demote Users**:
- Click "Promote" to upgrade a user to admin
- Click "Demote" to downgrade an admin to user

**Activate/Deactivate Users**:
- Click "Deactivate" to disable a user account
- Click "Activate" to re-enable a user account

Note: You cannot modify your own account from the admin panel.

## API Usage

### Create a User (Programmatically)

```rust
use web_template::auth::{AuthService, UserRole};

let user = AuthService::create_user(
    &db,
    "username",
    "email@example.com",
    "password",
    UserRole::User
).await?;
```

### Authenticate a User

```rust
let user = AuthService::authenticate(&db, "username", "password").await?;
```

### Create a Session

```rust
let session = AuthService::create_session(&db, &user.id).await?;
```

### Validate a Session

```rust
let user = AuthService::validate_session(&db, &session_id).await?;
```

## Middleware

### `require_auth`
Requires authentication. Redirects to `/login` if not authenticated.

```rust
Router::new()
    .route("/protected", get(handler))
    .layer(axum::middleware::from_fn(middlewares::require_auth))
```

### `require_admin`
Requires admin role. Returns 403 if user is not an admin. Must be used after `require_auth`.

```rust
Router::new()
    .route("/admin", get(admin_handler))
    .layer(axum::middleware::from_fn(middlewares::require_admin))
    .layer(axum::middleware::from_fn(middlewares::require_auth))
```

### `optional_auth`
Optionally authenticates if session exists, but doesn't redirect if not authenticated.

```rust
Router::new()
    .route("/", get(handler))
    .layer(axum::middleware::from_fn(middlewares::optional_auth))
```

## Password Requirements

- Minimum 8 characters
- No maximum length
- All characters allowed

For production, consider adding:
- Uppercase/lowercase requirements
- Number requirements
- Special character requirements
- Maximum length limits
- Password strength meter

## Session Management

- **Session Duration**: 7 days by default
- **Session Cleanup**: Expired sessions should be cleaned up periodically
- **Logout**: Deletes the session from database and removes cookie

### Clean Up Expired Sessions

```rust
let deleted_count = AuthService::cleanup_expired_sessions(&db).await?;
```

Consider running this periodically (e.g., daily cron job).

## Security Considerations

### Production Checklist

1. **HTTPS**: Enable HTTPS and set `secure` flag on cookies
2. **CSRF Protection**: Add CSRF tokens for state-changing operations
3. **Rate Limiting**: Add rate limiting on login/register endpoints
4. **Email Verification**: Add email verification for new accounts
5. **Password Reset**: Implement secure password reset flow
6. **2FA**: Consider adding two-factor authentication
7. **Audit Logging**: Log security-relevant events
8. **Session Rotation**: Rotate session IDs after privilege changes

### Current Security Features

✅ Argon2id password hashing  
✅ Constant-time password comparison  
✅ HTTP-only session cookies  
✅ SameSite cookie protection  
✅ Session expiration  
✅ Password strength validation  
✅ Input validation  
✅ Role-based access control  

### Recommended Improvements for Production

⚠️ Enable HTTPS and secure cookies  
⚠️ Add CSRF protection  
⚠️ Implement rate limiting  
⚠️ Add email verification  
⚠️ Add password reset functionality  
⚠️ Consider 2FA  
⚠️ Add audit logging  

## Database Schema

### Users Table

```sql
CREATE TABLE users (
    id TEXT PRIMARY KEY NOT NULL,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL DEFAULT 'user' CHECK (role IN ('user', 'admin')),
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_login_at TIMESTAMP
);
```

### Sessions Table

```sql
CREATE TABLE sessions (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    expires_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);
```

## Troubleshooting

### "Invalid credentials" error
- Check username and password are correct
- Ensure user account is active

### "Session not found or expired" error
- Session may have expired (7 days)
- Session may have been deleted (logout)
- Clear cookies and login again

### Cannot access admin panel
- Ensure user has admin role
- Check that user is logged in
- Verify session is valid

### Database errors
- Ensure migrations have run
- Check DATABASE_URL is correct
- Verify database is accessible
