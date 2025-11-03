# HORUS Telemetry API

Anonymous telemetry backend for HORUS Framework, deployed on Cloudflare Workers.

## Endpoints

### POST /telemetry
Submit a telemetry event.

**Request:**
```json
{
  "event": "install",
  "status": "success",
  "version": "0.1.4",
  "install_id": "a7f3e9c2...",
  "os": "Linux",
  "arch": "x86_64",
  "timestamp": 1699123456
}
```

**Response:**
```json
{
  "success": true
}
```

### GET /telemetry/badge
Get installation count in Shields.io badge format.

**Response:**
```json
{
  "schemaVersion": 1,
  "label": "installations",
  "message": "1,234",
  "color": "blue"
}
```

### GET /telemetry/stats
Get public statistics.

**Response:**
```json
{
  "total_installations": 1234,
  "platforms": [
    { "os": "Linux", "count": 950 },
    { "os": "Darwin", "count": 284 }
  ],
  "architectures": [
    { "arch": "x86_64", "count": 1000 },
    { "arch": "arm64", "count": 234 }
  ],
  "versions": [
    { "version": "0.1.4", "count": 800 },
    { "version": "0.1.3", "count": 434 }
  ],
  "install_success_rate": "94.50%",
  "installs_last_7_days": 45
}
```

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
# Test telemetry submission
curl -X POST https://horus-telemetry.YOUR_SUBDOMAIN.workers.dev/telemetry \
  -H "Content-Type: application/json" \
  -d '{
    "event": "install",
    "status": "success",
    "version": "0.1.4",
    "install_id": "test123",
    "os": "Linux",
    "arch": "x86_64",
    "timestamp": 1699123456
  }'

# Get badge data
curl https://horus-telemetry.YOUR_SUBDOMAIN.workers.dev/telemetry/badge

# Get stats
curl https://horus-telemetry.YOUR_SUBDOMAIN.workers.dev/telemetry/stats
```

## Privacy

This API collects only anonymous, aggregate data:
- ✅ Event type, OS, architecture, version
- ✅ Anonymous random install ID (for deduplication)
- ❌ No personal information
- ❌ No IP addresses stored
- ❌ No user code or data

See [PRIVACY.md](../PRIVACY.md) for full privacy policy.

## Architecture

- **Runtime:** Cloudflare Workers (serverless)
- **Database:** D1 (SQLite)
- **Free Tier:** 100k requests/day
- **Latency:** < 50ms globally
