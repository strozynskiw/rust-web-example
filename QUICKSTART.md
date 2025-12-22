# Quick Start Guide

Get up and running in 2 steps (SQLite - no setup required!):

## 1. Configure Environment

```bash
cp .env.example .env
```

The default uses SQLite - no database setup needed!

## 2. Start the Server

```bash
cd server
cargo run
```

## 3. Open Your Browser

Visit: http://localhost:3000

---

## Using PostgreSQL (Optional)

If you prefer PostgreSQL:

### Start Database

```bash
docker-compose up -d
```

### Update Environment

Edit `.env` and change:
```
DATABASE_URL=postgresql://rustweb:rustweb@localhost:5432/rustweb
```

### Run Server

```bash
cd server
cargo run
```

---

## Troubleshooting

### Database Connection Issues

Check if PostgreSQL is running:
```bash
docker-compose ps
```

View database logs:
```bash
docker-compose logs -f postgres
```

### Port Already in Use

If port 5432 is already in use, update `docker-compose.yml`:
```yaml
ports:
  - "5433:5432"  # Use different port
```

And update `.env`:
```
DATABASE_URL=postgresql://rustweb:rustweb@localhost:5433/rustweb
```

### Migration Errors

If migrations fail, you can reset the database:
```bash
docker-compose down -v
docker-compose up -d
```

This will remove all data and recreate the database.

