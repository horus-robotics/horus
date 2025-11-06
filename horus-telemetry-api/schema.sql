-- HORUS Installation Counter Database Schema (v3.0)
-- Simple counting - no tracking, no UUIDs

CREATE TABLE IF NOT EXISTS install_counts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event TEXT NOT NULL,
    os TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_event ON install_counts(event);
CREATE INDEX IF NOT EXISTS idx_timestamp ON install_counts(timestamp);
CREATE INDEX IF NOT EXISTS idx_os ON install_counts(os);
