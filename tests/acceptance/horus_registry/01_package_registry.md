# User Acceptance Test: Package Registry Backend

## Feature
Full-featured package registry backend with upload, download, search, and version management.

## User Story
As a package author, I want a reliable registry to host my packages so that other developers can easily discover and install them.

## Registry Setup Tests

### Scenario 1: Start Registry Server
**Given:** Registry backend is installed
**When:** Administrator runs `horus-registry start`
**Then:**
- [ ] Server starts on configured port
- [ ] Database connection established (SQLite)
- [ ] API endpoints are available
- [ ] Health check endpoint returns 200 OK

**Acceptance Criteria:**
```bash
$ horus-registry start
Starting HORUS registry server...
✓ Database initialized: registry.db
✓ Server listening on http://localhost:3000
✓ Health check: http://localhost:3000/health
Press Ctrl+C to stop
```

### Scenario 2: Health Check
**Given:** Registry is running
**When:** Client requests `/health`
**Then:**
- [ ] Returns 200 OK
- [ ] Response includes: status, version, uptime
- [ ] Response time < 100ms

**Acceptance Criteria:**
```bash
$ curl http://localhost:3000/health
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_seconds": 3600
}
```

## Package Upload Tests

### Scenario 3: Upload New Package
**Given:** Authenticated user with valid package
**When:** POST to `/api/packages/upload`
**Then:**
- [ ] Package is validated
- [ ] Package tarball is stored
- [ ] Metadata is saved to database
- [ ] Returns 201 Created with package URL

**Acceptance Criteria:**
```bash
$ curl -X POST http://localhost:3000/api/packages/upload \
  -H "Authorization: Bearer <token>" \
  -F "package=@lidar-driver-1.0.0.tar.gz" \
  -F "metadata=@package.json"

{
  "status": "success",
  "package": {
    "name": "lidar-driver",
    "version": "1.0.0",
    "url": "https://marketplace.horus-registry.dev/packages/lidar-driver/1.0.0"
  }
}
```

### Scenario 4: Upload Duplicate Version
**Given:** Package version already exists
**When:** Uploading same version again
**Then:**
- [ ] Returns 409 Conflict
- [ ] Error message explains duplicate version
- [ ] Original package is not modified

**Acceptance Criteria:**
```bash
$ curl -X POST .../upload ...
{
  "error": "Version 1.0.0 of lidar-driver already exists"
}
```

### Scenario 5: Upload Without Authentication
**Given:** No auth token provided
**When:** Attempting upload
**Then:**
- [ ] Returns 401 Unauthorized
- [ ] Error message requests authentication
- [ ] No package data is stored

### Scenario 6: Upload Invalid Package
**Given:** Package fails validation
**When:** Upload is attempted
**Then:**
- [ ] Returns 400 Bad Request
- [ ] Error lists all validation failures
- [ ] No partial data stored

**Acceptance Criteria:**
```bash
{
  "error": "Package validation failed",
  "details": [
    "Missing required field: description",
    "Invalid version format: 1.0",
    "Package name contains invalid characters"
  ]
}
```

## Package Download Tests

### Scenario 7: Download Latest Version
**Given:** Package exists with multiple versions
**When:** GET `/api/packages/{name}/latest`
**Then:**
- [ ] Latest version returned
- [ ] Tarball download link provided
- [ ] Metadata included

**Acceptance Criteria:**
```bash
$ curl http://localhost:3000/api/packages/lidar-driver/latest
{
  "name": "lidar-driver",
  "version": "1.2.0",
  "description": "USB Lidar sensor driver",
  "author": "robotics-team",
  "download_url": ".../lidar-driver-1.2.0.tar.gz",
  "published_at": "2024-10-18T12:00:00Z"
}
```

### Scenario 8: Download Specific Version
**Given:** Multiple versions exist
**When:** GET `/api/packages/{name}/{version}`
**Then:**
- [ ] Specified version returned
- [ ] Download link is correct
- [ ] 404 if version doesn't exist

**Acceptance Criteria:**
```bash
$ curl .../lidar-driver/1.0.0
{
  "name": "lidar-driver",
  "version": "1.0.0",
  "download_url": ".../lidar-driver-1.0.0.tar.gz"
}
```

### Scenario 9: Download Tarball
**Given:** Package exists
**When:** GET download URL
**Then:**
- [ ] Tarball is streamed
- [ ] Content-Type: application/gzip
- [ ] Content-Disposition header set
- [ ] Checksum can be verified

**Acceptance Criteria:**
```bash
$ curl -O .../lidar-driver-1.2.0.tar.gz
# File downloads successfully
$ sha256sum lidar-driver-1.2.0.tar.gz
# Matches published checksum
```

### Scenario 10: Download Non-Existent Package
**Given:** Package doesn't exist
**When:** Requesting download
**Then:**
- [ ] Returns 404 Not Found
- [ ] Error message is helpful
- [ ] Suggests search functionality

## Search and Discovery Tests

### Scenario 11: Search by Name
**Given:** Multiple packages in registry
**When:** GET `/api/packages/search?q=lidar`
**Then:**
- [ ] All matching packages returned
- [ ] Results sorted by relevance
- [ ] Pagination supported
- [ ] Latest version shown for each

**Acceptance Criteria:**
```bash
$ curl '.../search?q=lidar'
{
  "results": [
    {
      "name": "lidar-driver",
      "latest_version": "1.2.0",
      "description": "USB Lidar sensor driver",
      "downloads": 1234
    },
    {
      "name": "lidar-slam",
      "latest_version": "0.5.1",
      "description": "Real-time SLAM using lidar",
      "downloads": 567
    }
  ],
  "total": 2,
  "page": 1,
  "per_page": 20
}
```

### Scenario 12: Search No Results
**Given:** No packages match query
**When:** Searching
**Then:**
- [ ] Returns empty results array
- [ ] total: 0
- [ ] No error

**Acceptance Criteria:**
```bash
$ curl '.../search?q=nonexistent'
{
  "results": [],
  "total": 0
}
```

### Scenario 13: List All Packages
**Given:** Registry has packages
**When:** GET `/api/packages` (no query)
**Then:**
- [ ] All packages listed
- [ ] Paginated (default 20 per page)
- [ ] Sorted by popularity or recent uploads

**Acceptance Criteria:**
```bash
$ curl '.../packages?page=1&per_page=10'
{
  "results": [ ... 10 packages ... ],
  "total": 45,
  "page": 1,
  "per_page": 10,
  "pages": 5
}
```

### Scenario 14: Filter by Category (If Implemented)
**Given:** Packages have categories
**When:** GET `/api/packages?category=sensors`
**Then:**
- [ ] Only sensor packages returned
- [ ] Filtering works correctly

## Version Management Tests

### Scenario 15: List Package Versions
**Given:** Package has multiple versions
**When:** GET `/api/packages/{name}/versions`
**Then:**
- [ ] All versions listed
- [ ] Sorted by semantic version (newest first)
- [ ] Includes publish dates

**Acceptance Criteria:**
```bash
$ curl '.../lidar-driver/versions'
{
  "name": "lidar-driver",
  "versions": [
    {
      "version": "1.2.0",
      "published_at": "2024-10-18T12:00:00Z",
      "downloads": 500
    },
    {
      "version": "1.1.0",
      "published_at": "2024-09-15T10:00:00Z",
      "downloads": 300
    },
    {
      "version": "1.0.0",
      "published_at": "2024-08-01T08:00:00Z",
      "downloads": 434
    }
  ]
}
```

### Scenario 16: Semantic Version Comparison
**Given:** Versions: 1.0.0, 1.1.0, 2.0.0-beta, 2.0.0
**When:** Requesting latest
**Then:**
- [ ] 2.0.0 is returned (not beta)
- [ ] Or beta is included if explicitly requested
- [ ] Semantic versioning rules followed

## Statistics and Analytics

### Scenario 17: Package Download Count
**Given:** Package has been downloaded
**When:** Viewing package metadata
**Then:**
- [ ] Download count is accurate
- [ ] Increments on each download
- [ ] Per-version counts available

### Scenario 18: Popular Packages
**Given:** Registry has download statistics
**When:** GET `/api/packages/popular`
**Then:**
- [ ] Packages sorted by downloads
- [ ] Top 20 returned
- [ ] Download counts shown

### Scenario 19: Recent Packages
**Given:** New packages uploaded
**When:** GET `/api/packages/recent`
**Then:**
- [ ] Latest uploads shown first
- [ ] Publish dates included
- [ ] Limit to recent N packages

## Authentication and Authorization

### Scenario 20: GitHub OAuth Flow
**Given:** User wants to publish
**When:** Initiating OAuth
**Then:**
- [ ] Redirects to GitHub
- [ ] User authorizes app
- [ ] Token is issued
- [ ] Token stored securely

### Scenario 21: Token Validation
**Given:** Request includes auth token
**When:** API validates token
**Then:**
- [ ] Valid tokens are accepted
- [ ] Expired tokens rejected (401)
- [ ] Invalid tokens rejected (401)
- [ ] Error messages are clear

### Scenario 22: Permission Check
**Given:** User tries to upload package
**When:** Checking permissions
**Then:**
- [ ] User must own package or be authorized
- [ ] New packages can be uploaded by any authenticated user
- [ ] Updates require ownership

## Database and Storage

### Scenario 23: SQLite Database
**Given:** Registry uses SQLite
**When:** Storing package metadata
**Then:**
- [ ] ACID transactions
- [ ] Foreign key constraints enforced
- [ ] Indexes for performance
- [ ] Database file is portable

### Scenario 24: Package Storage
**Given:** Packages are uploaded
**When:** Storing tarballs
**Then:**
- [ ] Files stored in configured directory
- [ ] Organized by package/version
- [ ] Checksums stored and verified
- [ ] No duplicate storage

**Acceptance Criteria:**
```bash
$ ls -la packages/
lidar-driver/
├── 1.0.0/
│   └── lidar-driver-1.0.0.tar.gz
├── 1.1.0/
│   └── lidar-driver-1.1.0.tar.gz
└── 1.2.0/
    └── lidar-driver-1.2.0.tar.gz
```

### Scenario 25: Database Backup
**Given:** Registry has been running
**When:** Administrator backs up database
**Then:**
- [ ] SQLite file can be copied
- [ ] Backup is consistent
- [ ] Can restore from backup

## Error Handling and Resilience

### Scenario 26: Database Connection Failure
**Given:** Database is unavailable
**When:** API request occurs
**Then:**
- [ ] Returns 503 Service Unavailable
- [ ] Error message explains issue
- [ ] Retries connection

### Scenario 27: Disk Full
**Given:** Storage is full
**When:** Uploading package
**Then:**
- [ ] Upload fails gracefully
- [ ] Error: "Insufficient storage"
- [ ] No partial uploads
- [ ] Database remains consistent

### Scenario 28: Corrupted Package
**Given:** Uploaded tarball is corrupted
**When:** Download is requested
**Then:**
- [ ] Checksum mismatch detected
- [ ] Error returned
- [ ] Package marked as corrupted
- [ ] Admin is notified

## Performance Tests

### Scenario 29: Large Package Upload
**Given:** Package is 100MB
**When:** Uploading
**Then:**
- [ ] Upload succeeds
- [ ] Progress can be tracked
- [ ] Timeout is sufficient
- [ ] Storage is managed

### Scenario 30: Concurrent Uploads
**Given:** 10 users upload simultaneously
**When:** All uploads proceed
**Then:**
- [ ] All succeed or fail independently
- [ ] No database locks or conflicts
- [ ] Performance remains acceptable

### Scenario 31: High Search Load
**Given:** 1000 concurrent search requests
**When:** Server processes requests
**Then:**
- [ ] All requests complete
- [ ] Response time < 500ms
- [ ] No crashes or errors
- [ ] Database connections managed

## Non-Functional Requirements

- [ ] API response time < 200ms (p95)
- [ ] Supports 10,000 packages
- [ ] 99.9% uptime target
- [ ] Database size scalable to 100GB+
- [ ] Handles 1000 concurrent connections
- [ ] HTTPS only in production
- [ ] Rate limiting for API endpoints
- [ ] Logging for all operations
- [ ] Metrics for monitoring
- [ ] Automated backups
