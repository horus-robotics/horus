-- HORUS Telemetry Database Schema
-- Stores anonymous installation and update events

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event TEXT NOT NULL,
    status TEXT NOT NULL,
    version TEXT NOT NULL,
    install_id TEXT NOT NULL,
    os TEXT NOT NULL,
    arch TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for fast queries
CREATE INDEX IF NOT EXISTS idx_install_id ON events(install_id);
CREATE INDEX IF NOT EXISTS idx_event_status ON events(event, status);
CREATE INDEX IF NOT EXISTS idx_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_version ON events(version);
