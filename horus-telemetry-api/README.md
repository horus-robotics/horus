# HORUS Installation Counter API (v3.0)

Cloudflare Workers backend for counting HORUS installations.

**Privacy-first design**: No UUIDs, no tracking, pure counting.

## What It Does

Receives anonymous installation counts from `install.sh`:
```json
{
  "event": "install",
  "os": "Linux",
  "timestamp": 1699123456
}
```

That's it. No personal data, no tracking IDs, just counting.

## Endpoints

### `POST /count`
Submit an installation count.

**Request:**
```bash
curl -X POST https://telemetry.horus-registry.dev/count \
  -H "Content-Type: application/json" \
  -d '{"event":"install","os":"Linux","timestamp":1699123456}'
```

**Response:**
```json
{
  "success": true
}
```

### `GET /count/badge`
Get Shields.io badge JSON.

**Response:**
```json
{
  "schemaVersion": 1,
  "label": "installations",
  "message": "1,234",
  "color": "brightgreen"
}
```

**Usage in README:**
```markdown
![Installations](https://img.shields.io/endpoint?url=https://telemetry.horus-registry.dev/count/badge)
```

### `GET /count/stats`
Get public statistics.

**Response:**
```json
{
  "total_installations": 1234,
  "platforms": [
    {"os": "Linux", "count": 987},
    {"os": "Darwin", "count": 247}
  ],
  "installs_last_7_days": 89,
  "daily_installs": [
    {"date": "2025-01-06", "count": 15},
    {"date": "2025-01-05", "count": 12}
  ]
}
```

## Database Schema

```sql
CREATE TABLE install_counts (
    id INTEGER PRIMARY KEY,
    event TEXT NOT NULL,
    os TEXT NOT NULL,
    timestamp INTEGER NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

Simple, minimal, privacy-preserving.

## Development

### Local Testing
```bash
wrangler dev
```

### Deploy
```bash
# Apply schema to remote database
wrangler d1 execute horus-telemetry-db --file=./schema.sql --remote

# Deploy worker
wrangler deploy
```

### Test Endpoints
```bash
# Test installation count
curl -X POST http://localhost:8787/count \
  -H "Content-Type: application/json" \
  -d '{"event":"install","os":"Linux","timestamp":1699123456}'

# Get badge data
curl http://localhost:8787/count/badge

# Get stats
curl http://localhost:8787/count/stats
```

## Privacy

This API collects **minimal anonymous data**:
-  Event type (just "install")
-  OS (Linux/Darwin)
-  Timestamp
-  No UUIDs or tracking IDs
-  No personal information
-  No IP addresses stored
-  No user code or data

**Pure counting - can't track individuals.**

See [PRIVACY.md](../PRIVACY.md) for full privacy policy.

## Migration from v1.0/v2.0

**Old system collected:**
- `install_id` (UUID) 
- `version` 
- `arch` 
- `status` 

**New system collects:**
- `event` 
- `os` 
- `timestamp` 

Much simpler, more private, pure counting.

## Architecture

- **Runtime:** Cloudflare Workers (serverless)
- **Database:** D1 (SQLite)
- **Free Tier:** 100k requests/day
- **Latency:** < 50ms globally
- **Code:** 200 lines (down from 400)

## License

Apache 2.0 - same as HORUS framework
