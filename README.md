# web-template

A clean, production-ready template for building web applications with Rust. This template demonstrates a modern server-side rendered (SSR) architecture using Rust's async ecosystem, combining powerful backend frameworks with lightweight frontend technologies for a fast, maintainable web application.

## Architecture Overview

This template is built using a modular workspace structure with clear separation of concerns:

- **Server Application** (`server/`) - The main Axum-based HTTP server handling routing, middleware, and request processing
- **Common Library** (`common/`) - Shared database utilities supporting both SQLite and PostgreSQL
- **Templates** (`templates/`) - Tera template files organized into base layouts, pages, and HTMX partials

## Technology Stack

### Backend Framework
- **Axum 0.8** - A modern, ergonomic HTTP web framework built on Tokio, providing type-safe routing, middleware, and request/response handling
- **Tokio** - The async runtime powering the entire application with full feature set for I/O operations

### Database & ORM
- **SQLx 0.8** - Type-safe SQL query builder with compile-time verification, supporting both SQLite and PostgreSQL
- **SQLite** - Default embedded database for zero-configuration development
- **PostgreSQL** - Optional production-grade database with full SQLx support
- **Automatic Migrations** - SQLx migrations run automatically on server startup

### Templating
- **Tera 1.20** - Jinja2-inspired template engine with template inheritance, filters, and hot-reloading in debug mode
- **Template Organization** - Base templates with block inheritance, page templates, and reusable partials for HTMX responses

### Frontend Technologies
- **HTMX 1.9** - Enables dynamic interactions without writing JavaScript, using HTML attributes for AJAX, WebSockets, and server-sent events
- **Tailwind CSS** - Utility-first CSS framework loaded via CDN for rapid UI development
- **Progressive Enhancement** - Works without JavaScript, enhanced with HTMX for dynamic behavior

### Middleware & Utilities
- **Tower** - Modular middleware stack for HTTP services
- **Tower-HTTP** - HTTP-specific middleware including static file serving with cache headers
- **Tower-Cookies** - Cookie management middleware for session handling
- **Tracing** - Structured logging with `tracing` and `tracing-subscriber` for observability

### Development Tools
- **Dotenv** - Environment variable management for configuration
- **UUID** - User identification via persistent cookie-based UUIDs
- **Serde** - Serialization framework for JSON handling and form data
- **Chrono** - Date and time handling with timezone support

## Features

- **Axum** - Modern, fast HTTP framework with type-safe routing
- **Tera** - Template engine with hot-reloading in debug mode
- **Dual Database Support** - SQLite for development, PostgreSQL for production with automatic detection
- **SQLx** - Type-safe database queries with compile-time verification
- **HTMX** - Dynamic interactions without writing JavaScript
- **Tailwind CSS** - Utility-first CSS framework
- **User Identity** - Automatic user ID management via cookies
- **Simple Storage** - Store/load user data as JSON/JSONB
- **Structured Logging** - Tracing-based logging with configurable levels
- **Static File Serving** - Optimized static asset delivery with cache headers
- **Workspace Structure** - Modular Cargo workspace for code organization

## Structure

```
web-template/
├── server/          # Main application
│   ├── src/
│   │   ├── main.rs  # Server setup and routing
│   │   ├── site.rs  # Page handlers
│   │   ├── api.rs   # HTMX API endpoints
│   │   └── storage.rs # Database operations
│   └── migrations/  # SQL migrations
├── common/          # Shared utilities
│   └── src/
│       └── db.rs    # Database connection pool
└── templates/       # Tera templates
    ├── base.html    # Base template with header/footer
    ├── pages/       # Page templates
    └── partials/    # HTMX partial templates
```

## Getting Started

### Quick Start (SQLite - Default)

1. **Copy environment file:**
   ```bash
   cp .env.example .env
   ```

2. **Start the server:**
   ```bash
   cd server
   cargo run
   ```

3. **Visit:** http://localhost:3000

That's it! SQLite requires no setup - the database file will be created automatically.

### Using PostgreSQL (Optional)

If you prefer PostgreSQL:

1. **Start PostgreSQL:**
   ```bash
   docker-compose up -d
   ```

2. **Update `.env`:**
   ```bash
   DATABASE_URL=postgresql://web-template:web-template@localhost:5432/web-template
   ```

3. **Start the server:**
   ```bash
   cd server
   cargo run
   ```

The application automatically detects the database type from the connection string and runs appropriate migrations.

## Database Setup

### SQLite (Default)

SQLite is the default database and requires no setup. The database file (`data.db`) will be created automatically in the project root.

**Advantages:**
- Zero configuration
- No external dependencies
- Perfect for development and small projects
- Fast and reliable

**Connection string:**
```
DATABASE_URL=sqlite:./data.db
```

### PostgreSQL (Optional)

For production or when you need PostgreSQL features:

#### Using Docker Compose

The included `docker-compose.yml` provides a ready-to-use PostgreSQL instance:

```bash
# Start database
docker-compose up -d

# Stop database
docker-compose down

# View logs
docker-compose logs -f postgres

# Remove database and data
docker-compose down -v
```

#### Using Existing PostgreSQL

1. Create a database:
   ```sql
   CREATE DATABASE "web-template";
   ```

2. Update `.env` with your connection string:
   ```
   DATABASE_URL=postgresql://user:password@localhost:5432/web-template
   ```

**Connection string format:**
```
postgresql://username:password@host:port/database
```

### SQLx Offline Mode

For faster builds, you can use SQLx's offline mode:

```bash
# Generate offline metadata (requires database connection)
cd server

# For SQLite:
DATABASE_URL=sqlite:./data.db cargo sqlx prepare

# For PostgreSQL:
DATABASE_URL=postgresql://web-template:web-template@localhost:5432/web-template cargo sqlx prepare

# Build without database connection
cargo build
```

## HTMX Examples

The template includes two HTMX examples:

1. **Full Page Refresh** - Demonstrates replacing the entire page
2. **Partial Rendering** - Shows how to update specific content areas

## User Data Storage

Simple key-value storage for user-specific data:

- `POST /api/user-data` - Save data (form: `key`, `value`)
- `GET /api/user-data` - Load all user data

Data is stored as JSONB in PostgreSQL/SQlite, allowing flexible schema.

## Development

### Hot Reloading

Templates automatically reload in debug mode. No server restart needed for template changes.

### Database Migrations

Migrations are located in `server/migrations/` and run automatically on startup.

To create a new migration:

```bash
# Install sqlx-cli if needed
cargo install sqlx-cli --no-default-features --features postgres

# Create migration
cd server
sqlx migrate add migration_name
```

### Testing

```bash
cd server
cargo test
```

## Production

For production deployment:

1. Set `DATABASE_URL` environment variable
2. Set `RUST_LOG` for appropriate logging level
3. Build with release mode:
   ```bash
   cargo build --release
   ```

## License

MIT
