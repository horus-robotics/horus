# User Acceptance Test: Web Marketplace Frontend

## Feature
Next.js-based web marketplace for browsing, searching, and discovering HORUS packages.

## User Story
As a developer, I want an attractive web interface to browse packages so that I can discover useful libraries for my robot projects.

## Homepage Tests

### Scenario 1: Load Homepage
**Given:** User navigates to https://horus-registry.dev
**When:** Homepage loads
**Then:**
- [ ] Page loads in < 2 seconds
- [ ] Hero section with HORUS branding
- [ ] Search bar is prominent
- [ ] Featured packages shown
- [ ] Clean, modern design
- [ ] No console errors

**Visual Acceptance:**
```
┌────────────────────────────────────────────┐
│   HORUS                           Login    │
├────────────────────────────────────────────┤
│                                            │
│         HORUS Package Registry             │
│    Discover robotics packages for HORUS   │
│                                            │
│   ┌──────────────────────────────┐        │
│   │ Search packages...       🔍  │        │
│   └──────────────────────────────┘        │
│                                            │
│   Featured Packages                        │
│   [Package Card] [Package Card] ...        │
│                                            │
└────────────────────────────────────────────┘
```

### Scenario 2: Navigation Bar
**Given:** User is on any page
**When:** Viewing navigation
**Then:**
- [ ] Logo links to homepage
- [ ] "Packages" link
- [ ] "Docs" link (to documentation)
- [ ] "Publish" link (if authenticated)
- [ ] "Login" button (if not authenticated)
- [ ] User avatar/menu (if authenticated)

### Scenario 3: Search Bar
**Given:** User types in search
**When:** Entering text
**Then:**
- [ ] Live search suggestions appear
- [ ] Results update as user types
- [ ] Keyboard navigation works (arrow keys)
- [ ] Enter submits search

**Acceptance Criteria:**
```
User types "lid"
Suggestions appear:
  - lidar-driver
  - lidar-slam
  - sliding-window-filter
```

## Package Listing Tests

### Scenario 4: Browse All Packages
**Given:** User clicks "Packages"
**When:** Package list page loads
**Then:**
- [ ] All packages displayed in grid/list
- [ ] Each package shows: name, description, version, downloads
- [ ] Pagination or infinite scroll
- [ ] Sort options (popular, recent, name)
- [ ] Filter by category (if available)

**Visual Acceptance:**
```
┌────────────────────────────────────────────┐
│ Packages (42 total)         Sort: Popular │
├────────────────────────────────────────────┤
│ ┌────────────────────────────────────────┐ │
│ │ 📦 lidar-driver v1.2.0         ⬇ 1.2K │ │
│ │ USB Lidar sensor driver with ROS...    │ │
│ │ @robotics-team · Updated 2 days ago    │ │
│ └────────────────────────────────────────┘ │
│                                            │
│ ┌────────────────────────────────────────┐ │
│ │ 📦 slam-toolkit v0.8.1         ⬇ 890  │ │
│ │ SLAM algorithms for autonomous nav...  │ │
│ │ @slam-lab · Updated 1 week ago         │ │
│ └────────────────────────────────────────┘ │
│                                            │
│ [Load More]                                │
└────────────────────────────────────────────┘
```

### Scenario 5: Filter Packages
**Given:** User wants specific category
**When:** Selecting "Sensors" filter
**Then:**
- [ ] Only sensor packages shown
- [ ] Filter is visually active
- [ ] Count updates (e.g., "42 packages → 8 packages")
- [ ] Can clear filter easily

### Scenario 6: Sort Packages
**Given:** User changes sort order
**When:** Selecting "Most Recent"
**Then:**
- [ ] Packages reorder by publish date
- [ ] Most recent first
- [ ] Visual indicator of current sort

## Package Details Tests

### Scenario 7: View Package Details
**Given:** User clicks on package
**When:** Details page loads
**Then:**
- [ ] Full package information displayed
- [ ] Name, version, author, description
- [ ] README rendered with markdown
- [ ] Installation instructions
- [ ] Version history
- [ ] Download statistics

**Visual Acceptance:**
```
┌────────────────────────────────────────────┐
│ lidar-driver                      v1.2.0   │
├────────────────────────────────────────────┤
│ USB Lidar sensor driver with ROS compat   │
│ by @robotics-team · MIT License            │
│                                            │
│ [Install] [GitHub] [Report Issue]          │
│                                            │
│ $ horus pkg install lidar-driver           │
│                                            │
│ ┌────────────────────────────────────────┐ │
│ │ README                                 │ │
│ │ =====================================  │ │
│ │ # Lidar Driver                         │ │
│ │ This package provides...               │ │
│ │                                        │ │
│ └────────────────────────────────────────┘ │
│                                            │
│ Versions:                                  │
│ - v1.2.0 (latest) - Oct 18, 2024           │
│ - v1.1.0 - Sep 15, 2024                    │
│ - v1.0.0 - Aug 1, 2024                     │
│                                            │
│ Stats:                                     │
│ - Downloads: 1,234                         │
│ - Dependents: 5 packages                   │
└────────────────────────────────────────────┘
```

### Scenario 8: README Rendering
**Given:** Package has README.md
**When:** Viewing package details
**Then:**
- [ ] Markdown is rendered correctly
- [ ] Code blocks are syntax-highlighted
- [ ] Images display (if any)
- [ ] Links work correctly
- [ ] Tables formatted properly

### Scenario 9: Version Selection
**Given:** Package has multiple versions
**When:** User selects different version
**Then:**
- [ ] Page updates to show selected version
- [ ] README for that version shown
- [ ] Install command updates
- [ ] URL includes version

**Acceptance Criteria:**
```
URL: /packages/lidar-driver/1.1.0
Install command: horus pkg install lidar-driver@1.1.0
```

### Scenario 10: Copy Install Command
**Given:** User wants to install package
**When:** Clicking copy button on install command
**Then:**
- [ ] Command copied to clipboard
- [ ] Visual feedback (checkmark or toast)
- [ ] User can paste directly in terminal

## Search Tests

### Scenario 11: Full-Text Search
**Given:** User submits search query
**When:** Results page loads
**Then:**
- [ ] All matching packages shown
- [ ] Search terms highlighted in results
- [ ] Sorted by relevance
- [ ] Shows match count

**Acceptance Criteria:**
```
Search: "slam algorithm"
Found 3 packages:
- slam-toolkit (matches: description, tags)
- lidar-slam (matches: description)
- visual-slam (matches: name, description)
```

### Scenario 12: No Results
**Given:** Search has no matches
**When:** Viewing results
**Then:**
- [ ] Message: "No packages found"
- [ ] Suggestions to refine search
- [ ] Link to browse all packages
- [ ] No error state

### Scenario 13: Search Autocomplete
**Given:** User types in search bar
**When:** Typing "lid"
**Then:**
- [ ] Dropdown shows suggestions
- [ ] Keyboard navigation (up/down arrows)
- [ ] Click or Enter selects
- [ ] ESC closes dropdown

## User Account Tests

### Scenario 14: Login Flow
**Given:** User clicks "Login"
**When:** OAuth flow starts
**Then:**
- [ ] Redirects to GitHub
- [ ] User authorizes
- [ ] Redirects back to marketplace
- [ ] User is logged in
- [ ] Avatar shown in nav

**Acceptance Criteria:**
```
Before: [Login] button
After: [@username] avatar with dropdown
```

### Scenario 15: User Profile
**Given:** User is logged in
**When:** Clicking on avatar
**Then:**
- [ ] Dropdown menu appears
- [ ] "My Packages" link
- [ ] "Settings" link
- [ ] "Logout" link

### Scenario 16: My Packages Page
**Given:** User has published packages
**When:** Viewing "My Packages"
**Then:**
- [ ] All user's packages listed
- [ ] Shows: name, version, downloads, last updated
- [ ] Edit/delete options (if implemented)
- [ ] "Publish New Package" button

**Visual Acceptance:**
```
┌────────────────────────────────────────────┐
│ My Packages                [Publish New]   │
├────────────────────────────────────────────┤
│ lidar-driver v1.2.0            ⬇ 1,234    │
│ Last updated: 2 days ago                   │
│ [View] [Edit]                              │
│                                            │
│ sensor-fusion v2.0.0           ⬇ 567      │
│ Last updated: 1 week ago                   │
│ [View] [Edit]                              │
└────────────────────────────────────────────┘
```

### Scenario 17: Logout
**Given:** User is logged in
**When:** Clicking "Logout"
**Then:**
- [ ] User is logged out
- [ ] Redirected to homepage
- [ ] Login button reappears
- [ ] Protected routes inaccessible

## Publish Workflow (Web Interface)

### Scenario 18: Publish Page
**Given:** Authenticated user
**When:** Navigating to /publish
**Then:**
- [ ] Upload form displayed
- [ ] Instructions shown
- [ ] File upload dropzone
- [ ] Validation feedback

**Visual Acceptance:**
```
┌────────────────────────────────────────────┐
│ Publish Package                            │
├────────────────────────────────────────────┤
│ Upload your package tarball:               │
│                                            │
│ ┌────────────────────────────────────────┐ │
│ │  Drag & drop file here                 │ │
│ │  or click to browse                    │ │
│ │                                        │ │
│ │  Accepted: .tar.gz, .tgz               │ │
│ └────────────────────────────────────────┘ │
│                                            │
│ Package metadata will be extracted from    │
│ horus.yaml. Ensure it includes:            │
│ - name, version, description, license      │
│                                            │
│ [Upload and Publish]                       │
└────────────────────────────────────────────┘
```

### Scenario 19: Upload Progress
**Given:** User uploads large package
**When:** Upload is in progress
**Then:**
- [ ] Progress bar shown
- [ ] Percentage or size uploaded
- [ ] Can cancel upload
- [ ] Success message on completion

### Scenario 20: Publish Success
**Given:** Package uploaded successfully
**When:** Processing completes
**Then:**
- [ ] Success message with package URL
- [ ] Link to view package page
- [ ] Option to publish another

## Responsive Design Tests

### Scenario 21: Mobile View
**Given:** User on phone (320-480px width)
**When:** Viewing any page
**Then:**
- [ ] Layout adapts to screen
- [ ] Navigation collapses to hamburger menu
- [ ] Text is readable
- [ ] Buttons are tappable
- [ ] No horizontal scroll

### Scenario 22: Tablet View
**Given:** User on tablet (768-1024px width)
**When:** Viewing package grid
**Then:**
- [ ] 2-column layout
- [ ] All features accessible
- [ ] Touch-friendly interface

### Scenario 23: Desktop View
**Given:** User on desktop (1280px+ width)
**When:** Browsing packages
**Then:**
- [ ] 3-4 column grid
- [ ] Sidebar filters (if applicable)
- [ ] Optimal reading width for content

## Performance Tests

### Scenario 24: Page Load Speed
**Given:** User navigates to any page
**When:** Measuring load time
**Then:**
- [ ] First Contentful Paint < 1.5s
- [ ] Time to Interactive < 3s
- [ ] Lighthouse score > 90

### Scenario 25: Large Package List
**Given:** 1000+ packages in registry
**When:** Browsing all packages
**Then:**
- [ ] Pagination or virtual scrolling
- [ ] Smooth scrolling
- [ ] No lag when filtering/sorting

### Scenario 26: Image Optimization
**Given:** Package READMEs have images
**When:** Rendering images
**Then:**
- [ ] Images are lazy-loaded
- [ ] Optimized formats (WebP if supported)
- [ ] Responsive image sizes

## Accessibility Tests

### Scenario 27: Keyboard Navigation
**Given:** User navigates without mouse
**When:** Using Tab/Shift+Tab
**Then:**
- [ ] All interactive elements reachable
- [ ] Focus indicators visible
- [ ] Skip links available
- [ ] Enter/Space activates buttons

### Scenario 28: Screen Reader Support
**Given:** User with screen reader
**When:** Navigating site
**Then:**
- [ ] Semantic HTML used
- [ ] ARIA labels present
- [ ] Alt text on images
- [ ] Headings structured logically

### Scenario 29: Color Contrast
**Given:** WCAG 2.1 standards
**When:** Checking all text
**Then:**
- [ ] Contrast ratio ≥ 4.5:1 for normal text
- [ ] Contrast ratio ≥ 3:1 for large text
- [ ] Links distinguishable from text

## SEO and Metadata

### Scenario 30: Meta Tags
**Given:** Package detail page
**When:** Viewing page source
**Then:**
- [ ] Title tag includes package name
- [ ] Meta description present
- [ ] Open Graph tags for social sharing
- [ ] Canonical URLs set

**Acceptance Criteria:**
```html
<title>lidar-driver v1.2.0 | HORUS Registry</title>
<meta name="description" content="USB Lidar sensor driver...">
<meta property="og:title" content="lidar-driver">
<meta property="og:description" content="...">
```

### Scenario 31: Sitemap
**Given:** Marketplace is deployed
**When:** Accessing /sitemap.xml
**Then:**
- [ ] All package pages listed
- [ ] Dynamic sitemap generation
- [ ] Submitted to search engines

## Non-Functional Requirements

- [ ] Next.js SSR for SEO
- [ ] CDN for static assets
- [ ] 99.5% uptime
- [ ] Mobile-first responsive design
- [ ] WCAG 2.1 AA compliance
- [ ] Cross-browser support (Chrome, Firefox, Safari, Edge)
- [ ] Page load < 3s on 3G
- [ ] Works with JavaScript disabled (core content)
- [ ] HTTPS only
- [ ] CSP headers configured
