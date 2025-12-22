-- Simple user_data table for storing user-specific data
-- Works with both SQLite and PostgreSQL

-- Create table with SQLite-compatible types (TEXT works for both)
CREATE TABLE IF NOT EXISTS user_data (
    user_id TEXT PRIMARY KEY,
    data TEXT NOT NULL DEFAULT '{}',
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create index (works for both)
CREATE INDEX IF NOT EXISTS idx_user_data_updated_at ON user_data(updated_at);
