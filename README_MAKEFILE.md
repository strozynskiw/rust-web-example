# Makefile Tasks Guide

This project uses `cargo-make` for task automation. It provides convenient shortcuts for common development tasks.

## Installation

First, install cargo-make:

```bash
cargo install cargo-make
```

## Quick Reference

### See All Available Tasks
```bash
cargo make
# or
cargo make help
```

## Common Tasks

### 🚀 Development

```bash
# Run the server (with type checking first)
cargo make run

# Run with debug logging
cargo make dev

# Auto-reload on file changes (requires cargo-watch)
cargo install cargo-watch
cargo make watch
```

### 🔨 Building

```bash
# Build in debug mode
cargo make build

# Build optimized release binary
cargo make build-release

# Fast type checking (faster than full build)
cargo make check

# Clean build artifacts
cargo make clean
```

### ✅ Code Quality

```bash
# Run clippy linter
cargo make clippy

# Format code
cargo make fmt

# Check formatting (don't modify)
cargo make fmt-check

# Run all linters (clippy + format check)
cargo make lint

# Run tests
cargo make test

# Run tests with output
cargo make test-verbose
```

### 🗄️ Database Management

```bash
# Start PostgreSQL via docker-compose
cargo make db-setup

# Stop PostgreSQL
cargo make db-stop

# Reset database (remove all data)
cargo make db-reset

# View database logs
cargo make db-logs

# Run migrations manually
cargo make migrate

# Revert last migration
cargo make migrate-revert

# Prepare SQLx offline metadata
cargo make sqlx-prepare
```

### 👤 User Management

```bash
# Create an admin user
cargo make create-admin -- admin admin@example.com SecurePass123

# Create a regular user
cargo make create-user -- john john@example.com SecurePass123 user

# Using the script directly
./scripts/create_user.sh admin admin@example.com SecurePass123
./scripts/create_user.sh john john@example.com SecurePass123 user
```

### 🐳 Docker

```bash
# Build Docker image (includes release build)
cargo make docker-build

# Run Docker container
cargo make docker-run

# Stop Docker container
cargo make docker-stop
```

### 🔄 CI/CD

```bash
# Run CI checks locally (fast)
cargo make ci

# Run full CI suite with release build
cargo make ci-full
```

### 🛠️ Utilities

```bash
# Initial project setup
cargo make setup

# Clean everything and start fresh
cargo make fresh-start

# Update dependencies
cargo make update

# Check for outdated dependencies
cargo make outdated

# Show dependency tree
cargo make tree

# Analyze binary size
cargo make bloat
```

## Workflow Examples

### Starting Development

```bash
# First time setup
cargo make setup
cargo make create-admin -- admin admin@example.com SecurePass123

# Start development
cargo make run
```

### Before Committing

```bash
# Run all checks
cargo make ci

# Or individually
cargo make fmt
cargo make clippy
cargo make test
```

### Production Build

```bash
# Build optimized binary
cargo make build-release

# Or use Docker
cargo make docker-build
cargo make docker-run
```

### Database Workflow

```bash
# Start with PostgreSQL
cargo make db-setup
DATABASE_URL=postgresql://web-template:web-template@localhost:5432/web-template cargo make run

# Or use default SQLite
cargo make run
```

## Environment Variables

You can override environment variables in your `.env` file or inline:

```bash
# Use PostgreSQL
DATABASE_URL=postgresql://user:pass@localhost:5432/db cargo make run

# Debug logging
RUST_LOG=debug cargo make run

# Custom port
PORT=8080 cargo make run
```

## Task Dependencies

Some tasks automatically run dependencies:

- `run` → runs `check` first
- `docker-build` → runs `build-release` first
- `ci` → runs `fmt-check`, `clippy`, `test`, `build`
- `dev-server` → runs `db-setup` first

## Tips

1. **Fast Iteration**: Use `cargo make check` during development (faster than build)
2. **Auto-reload**: Use `cargo make watch` for instant feedback
3. **CI Locally**: Run `cargo make ci` before pushing
4. **Debug Issues**: Use `cargo make dev` for detailed logs
5. **Clean Slate**: Use `cargo make fresh-start` if things get weird

## Customization

Edit `Makefile.toml` to add your own tasks or modify existing ones. See [cargo-make documentation](https://github.com/sagiegurari/cargo-make) for more details.

## Common Issues

### `cargo-make` not found
```bash
cargo install cargo-make
```

### `cargo-watch` not found (for watch task)
```bash
cargo install cargo-watch
```

### `sqlx` not found (for migration tasks)
```bash
cargo install sqlx-cli --no-default-features --features sqlite,postgres
```

### Database connection errors
Make sure DATABASE_URL is set in your `.env` file or environment.

### Port already in use
Change the port: `PORT=8080 cargo make run`
