# User Acceptance Test: Monitoring Dashboard

## Feature
Real-time web-based and TUI monitoring for running HORUS nodes and topics.

## User Story
As a robotics developer, I want to visualize what my nodes are doing in real-time so that I can debug issues and monitor system health.

## Web Dashboard Tests

### Scenario 1: Launch Web Dashboard
**Given:** HORUS application is running
**When:** User runs `horus dashboard`
**Then:**
- [ ] Web server starts on port 8080
- [ ] Browser opens automatically
- [ ] Dashboard loads without errors
- [ ] Connection to backend established

**Acceptance Criteria:**
```bash
$ horus dashboard
Starting HORUS dashboard...
 Server running at http://localhost:8080
Opening browser...
Press Ctrl+C to stop
```

### Scenario 2: Dashboard Homepage
**Given:** Dashboard is open in browser
**When:** User views main page
**Then:**
- [ ] Clean, professional UI
- [ ] Navigation is intuitive
- [ ] Responsive design (works on different screen sizes)
- [ ] No console errors in browser DevTools

### Scenario 3: Node List View
**Given:** Multiple nodes are registered
**When:** User views Nodes tab
**Then:**
- [ ] All registered nodes listed
- [ ] Shows: name, priority, status, tick count
- [ ] Updates in real-time (every 1-2 seconds)
- [ ] Color coding for status (green=active, red=error, grey=stopped)

**Visual Acceptance:**
```
─────────────────────────────────────────────────────────
 HORUS Dashboard - Nodes                                 
─────────────────────────────────────────────────────────
 Total Nodes: 3 | Active: 3 | Stopped: 0 | Errors: 0    
────────────────────────────────────────────────────
 Node Name        Priority  Status  Ticks   Avg ms  
────────────────────────────────────────────────────
 KeyboardInput    0         🟢 Active  1,234   0.05    
 SnakeControl     2         🟢 Active  1,234   0.12    
 GUINode          3         🟢 Active  1,233   2.50    
────────────────────────────────────────────────────
```

### Scenario 4: Topic List View
**Given:** Nodes are publishing/subscribing
**When:** User views Topics tab
**Then:**
- [ ] All active topics listed
- [ ] Shows: topic name, publishers count, subscribers count, message rate
- [ ] Updates in real-time
- [ ] Message rate in Hz or msg/sec

**Visual Acceptance:**
```
──────────────────────────────────────────────────────
 HORUS Dashboard - Topics                             
──────────────────────────────────────────────────────
 Topic Name        Publishers  Subscribers  Rate   
───────────────────────────────────────────────────
 keyboard/input    1           1            60 Hz  
 snake/state       1           1            60 Hz  
 snake/cmd         1           1            60 Hz  
 sensors/lidar     1           2            10 Hz  
───────────────────────────────────────────────────
```

### Scenario 5: Node Details Page
**Given:** User clicks on a node name
**When:** Details page loads
**Then:**
- [ ] Full node information displayed
- [ ] Metrics: tick count, avg/max duration, uptime
- [ ] Messages sent/received counters
- [ ] Publisher and subscriber topics listed
- [ ] Real-time chart of tick duration (if available)

**Visual Acceptance:**
```
──────────────────────────────────────────
 Node: SnakeControl                       
──────────────────────────────────────────
 Priority: 2                              
 Status: Active                           
 Uptime: 00:05:32                         
                                          
 Performance:                             
  - Total Ticks: 1,234                    
  - Avg Tick: 0.12ms                      
  - Max Tick: 0.85ms                      
                                          
 Communication:                           
  - Messages Sent: 1,234                  
  - Messages Received: 1,234              
                                          
 Publishers:                              
  - snake/cmd                             
                                          
 Subscribers:                             
  - keyboard/input                        
──────────────────────────────────────────
```

### Scenario 6: System Overview
**Given:** Dashboard main page
**When:** User views system section
**Then:**
- [ ] Total nodes count
- [ ] Total topics count
- [ ] System uptime
- [ ] Overall health status
- [ ] CPU/memory usage (if available)

### Scenario 7: Real-Time Updates
**Given:** Dashboard is open
**When:** Nodes continue executing
**Then:**
- [ ] Tick counts increment
- [ ] Message rates update
- [ ] No manual page refresh required
- [ ] Updates via WebSocket or polling
- [ ] Smooth updates without flicker

### Scenario 8: Error Visualization
**Given:** Node encounters error
**When:** Dashboard updates
**Then:**
- [ ] Node status changes to error state
- [ ] Error message displayed
- [ ] Red indicator or color coding
- [ ] Timestamp of error shown

### Scenario 9: No Nodes Running
**Given:** Dashboard starts but no HORUS app running
**When:** User views dashboard
**Then:**
- [ ] Message: "No nodes detected"
- [ ] Instructions to start a HORUS application
- [ ] Dashboard waits for nodes to appear
- [ ] Auto-updates when nodes start

### Scenario 10: Stop Dashboard
**Given:** Dashboard is running
**When:** User presses Ctrl+C in terminal
**Then:**
- [ ] Server stops gracefully
- [ ] Message: "Dashboard stopped"
- [ ] Browser shows connection closed
- [ ] Port 8080 is freed

## TUI (Terminal UI) Tests

### Scenario 11: Launch TUI Dashboard
**Given:** User prefers terminal interface
**When:** User runs `horus dashboard --tui`
**Then:**
- [ ] Terminal UI is displayed
- [ ] Node list shown
- [ ] Updates in real-time
- [ ] Keyboard navigation works

**Visual Acceptance:**
```

            HORUS Dashboard (TUI)                  

 Nodes: 3 active | Topics: 5 | Uptime: 00:05:23   

 [Tab] Switch View | [q] Quit | [r] Refresh       

                                                   
 Nodes:                                            
 > KeyboardInput  [Pri:0]   Active  1234 ticks   
   SnakeControl   [Pri:2]   Active  1234 ticks   
   GUINode        [Pri:3]   Active  1233 ticks   
                                                   

```

### Scenario 12: TUI Navigation
**Given:** TUI dashboard is running
**When:** User presses arrow keys or Tab
**Then:**
- [ ] Can navigate between nodes
- [ ] Can switch between tabs (Nodes, Topics, System)
- [ ] Selected item is highlighted
- [ ] Enter key shows details

### Scenario 13: TUI Auto-Refresh
**Given:** TUI dashboard is running
**When:** Data updates
**Then:**
- [ ] Screen refreshes automatically
- [ ] No flicker or artifacts
- [ ] Cursor position maintained
- [ ] Updates every 1-2 seconds

### Scenario 14: TUI Color Support
**Given:** Terminal supports colors
**When:** TUI renders
**Then:**
- [ ] Status indicators are colored
- [ ] Active nodes show in green
- [ ] Errors show in red
- [ ] Headers are highlighted

### Scenario 15: TUI Exit
**Given:** TUI is running
**When:** User presses 'q' or Ctrl+C
**Then:**
- [ ] TUI closes cleanly
- [ ] Terminal restored to normal
- [ ] No leftover artifacts
- [ ] Exit code 0

## API and Backend Tests

### Scenario 16: REST API Endpoints
**Given:** Dashboard backend is running
**When:** Client requests `/api/nodes`
**Then:**
- [ ] Returns JSON with all nodes
- [ ] Response is well-formed
- [ ] CORS headers present (if needed)
- [ ] Response time < 100ms

**Acceptance Criteria:**
```bash
$ curl http://localhost:8080/api/nodes
[
  {
    "name": "KeyboardInput",
    "priority": 0,
    "status": "active",
    "tick_count": 1234,
    "avg_tick_ms": 0.05
  },
  ...
]
```

### Scenario 17: WebSocket Connection
**Given:** Dashboard web page is open
**When:** WebSocket connects
**Then:**
- [ ] Connection establishes successfully
- [ ] Real-time data streams
- [ ] Reconnects on disconnect
- [ ] Error handling for connection loss

### Scenario 18: Metrics Endpoint
**Given:** Backend running
**When:** Requesting `/api/metrics`
**Then:**
- [ ] System-wide metrics returned
- [ ] Includes: total_nodes, total_topics, uptime
- [ ] Performance statistics
- [ ] JSON format

## Performance Tests

### Scenario 19: Dashboard with 100 Nodes
**Given:** 100 nodes registered
**When:** Dashboard renders
**Then:**
- [ ] All nodes displayed
- [ ] UI remains responsive
- [ ] Updates don't lag
- [ ] Pagination or virtualization used (if needed)

### Scenario 20: High Update Frequency
**Given:** Nodes publishing at 1000 Hz
**When:** Dashboard updates
**Then:**
- [ ] Message rates calculated correctly
- [ ] UI doesn't freeze
- [ ] Server CPU usage reasonable (<5%)
- [ ] Updates throttled to 1-2 Hz for UI

### Scenario 21: Long-Running Dashboard
**Given:** Dashboard runs for 24 hours
**When:** Monitoring memory usage
**Then:**
- [ ] No memory leaks
- [ ] Performance remains stable
- [ ] WebSocket connections stable
- [ ] No accumulation of resources

## Error Handling

### Scenario 22: Port Already in Use
**Given:** Port 8080 is occupied
**When:** User runs `horus dashboard`
**Then:**
- [ ] Error: "Port 8080 is already in use"
- [ ] Suggestion to use --port flag
- [ ] Exit code is non-zero

### Scenario 23: Backend Crash
**Given:** Dashboard is open in browser
**When:** Backend crashes
**Then:**
- [ ] Browser shows connection error
- [ ] User notified of connection loss
- [ ] Retry mechanism attempts reconnection

### Scenario 24: Invalid Data
**Given:** Backend receives corrupted data
**When:** Processing metrics
**Then:**
- [ ] Error is caught
- [ ] Dashboard shows "Data unavailable"
- [ ] Doesn't crash entire system

## User Experience

### Scenario 25: First-Time User
**Given:** User opens dashboard for first time
**When:** Landing on main page
**Then:**
- [ ] Purpose is immediately clear
- [ ] Navigation is intuitive
- [ ] Help or tooltips available
- [ ] No technical jargon unnecessarily

### Scenario 26: Mobile Viewing
**Given:** User opens dashboard on phone/tablet
**When:** Viewing on small screen
**Then:**
- [ ] Layout adapts to screen size
- [ ] All features accessible
- [ ] Text is readable
- [ ] Touch targets are adequate

### Scenario 27: Dark Mode (Optional)
**Given:** User prefers dark theme
**When:** Switching to dark mode
**Then:**
- [ ] UI switches to dark theme
- [ ] Colors remain distinguishable
- [ ] Preference is saved

## Non-Functional Requirements

- [ ] Dashboard starts in < 3 seconds
- [ ] Page load time < 1 second
- [ ] API response time < 100ms
- [ ] WebSocket latency < 50ms
- [ ] Supports 100+ concurrent nodes
- [ ] Works in Chrome, Firefox, Safari, Edge
- [ ] Responsive design (mobile/tablet/desktop)
- [ ] Accessible (keyboard navigation, screen readers)
- [ ] No JavaScript errors in console
- [ ] HTTPS support (if deployed remotely)
