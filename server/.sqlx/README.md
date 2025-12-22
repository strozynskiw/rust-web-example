# SQLx Offline Mode

This directory contains SQLx offline metadata for faster builds without a database connection.

To regenerate this metadata:

```bash
cd server
DATABASE_URL=postgresql://rustweb:rustweb@localhost:5432/rustweb cargo sqlx prepare
```

This is useful for CI/CD pipelines where you don't have a database available during build.


