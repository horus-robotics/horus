# User Acceptance Test: `horus env` and `horus dashboard` Commands

## Feature
Environment management for reproducible builds and real-time monitoring dashboard.

## User Story
As a developer, I want to freeze my project environment for reproducibility and monitor my running nodes in real-time.

## Environment Management Tests

### Scenario 1: Freeze Environment
**Given:** User has a project with installed packages
**When:** User runs `horus env freeze`
**Then:**
- [ ] All installed packages are recorded
- [ ] Versions are locked
- [ ] File `horus-lock.toml` or similar is created
- [ ] Success message confirms packages frozen

**Acceptance Criteria:**
```bash
$ horus env freeze
Freezing environment...
Recorded 5 packages:
  - horus-core v0.1.0
  - lidar-driver v1.2.0
  - sensor-common v1.0.2
  - slam-toolkit v0.8.1
  - nav-stack v2.1.0
 Environment frozen to horus-lock.toml
```

### Scenario 2: Restore Environment
**Given:** `horus-lock.toml` exists
**When:** User runs `horus env restore`
**Then:**
- [ ] Locked packages are installed
- [ ] Exact versions match lock file
- [ ] Dependencies are resolved
- [ ] Success message confirms restoration

**Acceptance Criteria:**
```bash
$ horus env restore
Restoring environment from horus-lock.toml...
Installing packages:
   horus-core v0.1.0
   lidar-driver v1.2.0
   sensor-common v1.0.2
   slam-toolkit v0.8.1
   nav-stack v2.1.0
 Environment restored successfully
```

### Scenario 3: Restore Without Lock File
**Given:** No `horus-lock.toml` exists
**When:** User runs `horus env restore`
**Then:**
- [ ] Error: "No horus-lock.toml found"
- [ ] Suggestion to run `horus env freeze` first
- [ ] Exit code is non-zero

### Scenario 4: Freeze with No Packages
**Given:** No packages are installed
**When:** User runs `horus env freeze`
**Then:**
- [ ] Warning: "No packages to freeze"
- [ ] Empty lock file is created (optional)
- [ ] Or informative message about empty environment

### Scenario 5: Version Conflict on Restore
**Given:** Lock file has conflicting versions
**When:** User runs `horus env restore`
**Then:**
- [ ] Conflict is detected
- [ ] Error explains which packages conflict
- [ ] No packages are installed (atomic operation)
- [ ] Suggestion to resolve manually

### Scenario 6: Export Environment
**Given:** User wants to share environment
**When:** User runs `horus env freeze --output my-env.toml`
**Then:**
- [ ] Environment saved to specified file
- [ ] File can be shared with team
- [ ] Restore works with custom file: `horus env restore --file my-env.toml`

### Scenario 7: Compare Environments (FUTURE FEATURE)
>  **Note:** This feature is planned but not yet implemented. Marked for future release.

**Given:** User has local changes to environment
**When:** User runs `horus env diff`
**Then:**
- [ ] Shows differences between current and locked environment
- [ ] Additions shown in green
- [ ] Removals shown in red
- [ ] Version changes highlighted

**Acceptance Criteria:**
```bash
$ horus env diff
Environment differences:
  + new-package v1.0.0 (added)
  - old-package v0.5.0 (removed)
  ~ lidar-driver v1.2.0  v1.3.0 (upgraded)
```

**Implementation Status:** Planned for v0.2.0

## Dashboard Tests

### Scenario 8: Start Web Dashboard
**Given:** HORUS project is running
**When:** User runs `horus dashboard`
**Then:**
- [ ] Web server starts on localhost:8080
- [ ] Browser opens automatically
- [ ] Dashboard shows running nodes
- [ ] Real-time metrics are visible
- [ ] Scheduler status is shown

**Acceptance Criteria:**
```bash
$ horus dashboard
Starting HORUS dashboard...
 Server running at http://localhost:8080
Opening browser...
Press Ctrl+C to stop
```

### Scenario 9: Dashboard with Custom Port
**Given:** Port 8080 is already in use
**When:** User runs `horus dashboard --port 3000`
**Then:**
- [ ] Server starts on port 3000
- [ ] URL reflects custom port
- [ ] Dashboard functions normally

**Acceptance Criteria:**
```bash
$ horus dashboard --port 3000
 Server running at http://localhost:3000
```

### Scenario 10: Dashboard Shows Node List
**Given:** Dashboard is running
**And:** Multiple nodes are executing
**When:** User views dashboard in browser
**Then:**
- [ ] All registered nodes are listed
- [ ] Each node shows: name, priority, status
- [ ] Tick count is displayed
- [ ] Last tick duration shown

**Visual Acceptance:**
```
HORUS Dashboard

Nodes:
──────────────────────────────────────────────
 Node Name        Priority  Status  Ticks     
──────────────────────────────────────────────
 KeyboardInput    0         Active  1,234     
 SnakeControl     2         Active  1,234     
 GUINode          3         Active  1,233     
──────────────────────────────────────────────
```

### Scenario 11: Dashboard Shows Hub Activity
**Given:** Nodes are publishing/subscribing
**When:** User views hub metrics in dashboard
**Then:**
- [ ] All topics are listed
- [ ] Publishers count shown per topic
- [ ] Subscribers count shown per topic
- [ ] Message rate (msgs/sec) displayed
- [ ] Latest message preview (if small)

**Visual Acceptance:**
```
Topics:
─────────────────────────────────────────────
 Topic             Pub     Sub     Rate      
─────────────────────────────────────────────
 keyboard/input    1       1       60 msg/s  
 snake/state       1       1       60 msg/s  
 snake/cmd         1       1       60 msg/s  
─────────────────────────────────────────────
```

### Scenario 12: Dashboard Real-Time Updates
**Given:** Dashboard is open in browser
**When:** Nodes continue executing
**Then:**
- [ ] Metrics update every 1-2 seconds
- [ ] No page refresh required
- [ ] WebSocket or polling keeps data fresh
- [ ] CPU/memory usage minimal

### Scenario 13: Dashboard with No Running Nodes
**Given:** No HORUS application is running
**When:** User runs `horus dashboard`
**Then:**
- [ ] Dashboard starts anyway
- [ ] Message: "No nodes detected"
- [ ] Instruction to run a HORUS project
- [ ] Dashboard waits for nodes to appear

### Scenario 14: Dashboard TUI Mode (If Implemented)
**Given:** User prefers terminal interface
**When:** User runs `horus dashboard --tui`
**Then:**
- [ ] Terminal UI is displayed
- [ ] Node list shown in terminal
- [ ] Updates in real-time
- [ ] Ctrl+C exits cleanly

**Acceptance Criteria:**
```bash
$ horus dashboard --tui

       HORUS Dashboard (TUI)           

 Nodes: 3 active                       
 Topics: 5 active                      
 Uptime: 00:05:23                      

 KeyboardInput  [Pri:0]   1234 ticks 
 SnakeControl   [Pri:2]   1234 ticks 
 GUINode        [Pri:3]   1233 ticks 

Press 'q' to quit
```

### Scenario 15: Stop Dashboard
**Given:** Dashboard is running
**When:** User presses Ctrl+C
**Then:**
- [ ] Server stops gracefully
- [ ] Message: "Dashboard stopped"
- [ ] Port is freed
- [ ] Exit code 0

## Edge Cases

### Edge Case 1: Port Already in Use
**Given:** Port 8080 is occupied
**When:** User runs `horus dashboard` (default port)
**Then:**
- [ ] Error: "Port 8080 is already in use"
- [ ] Suggestion to use --port flag
- [ ] Alternative ports suggested (8081, 8082, etc.)

### Edge Case 2: No Browser Available
**Given:** Running in headless environment
**When:** User runs `horus dashboard`
**Then:**
- [ ] Server starts successfully
- [ ] Message shows URL to access manually
- [ ] No error about browser launch failure

### Edge Case 3: Lock File Corruption
**Given:** `horus-lock.toml` is corrupted
**When:** User runs `horus env restore`
**Then:**
- [ ] Parsing error is detected
- [ ] Error: "Lock file is corrupted"
- [ ] Suggestion to regenerate with `freeze`

### Edge Case 4: Dashboard During Node Crash
**Given:** Dashboard is showing nodes
**When:** A node crashes
**Then:**
- [ ] Node status updates to "Crashed" or "Error"
- [ ] Error message displayed (if available)
- [ ] Other nodes continue showing correctly
- [ ] Dashboard remains stable

## Help Documentation

**When:** User runs `horus env --help`
**Then:**
```bash
$ horus env --help
Manage project environment

Usage: horus env <COMMAND>

Commands:
  freeze   Freeze current environment to lock file
  restore  Restore environment from lock file
  diff     Show differences between current and locked
  help     Print this message

Options:
  -h, --help  Print help
```

**When:** User runs `horus dashboard --help`
**Then:**
```bash
$ horus dashboard --help
Launch monitoring dashboard

Usage: horus dashboard [OPTIONS]

Options:
      --port <PORT>  Port for web server [default: 8080]
      --tui          Use terminal UI instead of web
      --no-browser   Don't open browser automatically
  -h, --help         Print help
```

## Non-Functional Requirements

### Environment Management
- [ ] Freeze completes in < 1 second
- [ ] Restore shows progress for many packages
- [ ] Lock file is human-readable (TOML)
- [ ] Atomic restore (all or nothing)

### Dashboard
- [ ] Dashboard starts in < 3 seconds
- [ ] Updates refresh every 1-2 seconds
- [ ] Supports 50+ concurrent nodes without lag
- [ ] Minimal CPU usage (< 5%)
- [ ] Works in all modern browsers
- [ ] Mobile-responsive UI
- [ ] Accessible keyboard navigation
