# HORUS Privacy Policy - Silent Installation Counter

**Last Updated:** 2025-01-06
**Version:** 3.0

## Overview

HORUS uses **silent installation counting** to understand how many times the framework is installed. This is privacy-first by design: no prompts, minimal data, pure counting.

## What We Collect

When you run `./install.sh`, we send a single anonymous ping with:

| Data Point | Example | Purpose |
|------------|---------|---------|
| Event type | `install` | Know it's an installation |
| Operating system | `Linux`, `Darwin` | Prioritize platform support |
| Timestamp | `1699123456` (Unix time) | Understand adoption trends |

**That's it.** No UUID, no version, no personal info, nothing else. Just a pure counter.

## What We DON'T Collect

We **never** collect:

- Personal information (name, email, address)
- IP addresses or location data
- Your code or project data
- File paths or directory names
- Environment variables
- Machine identifiers
- UUIDs or tracking IDs
- Anything identifiable

## How It Works

### 1. No Installation Prompt

There's **no prompt**. The counter runs silently in the background when you install.

### 2. On Installation

When you run `./install.sh`:
1. A background curl request sends: `{event: "install", os: "Linux", timestamp: 123}`
2. The request runs in the background (3s timeout)
3. Installation continues immediately (never blocked)
4. Fails silently if network is down

### 3. No Tracking

Unlike the old system, there's **no UUID at all**. We can't distinguish between:
- First install vs reinstall
- Same machine vs different machine
- Individual users

We just count: "500 install events happened"

### 4. The Code

```bash
# From install.sh
if [ -z "$HORUS_NO_TELEMETRY" ]; then
    (curl -X POST https://telemetry.horus-registry.dev/count \
         -H "Content-Type: application/json" \
         -d "{\"event\":\"install\",\"os\":\"$(uname -s)\",\"timestamp\":$(date +%s)}" \
         --max-time 3 --silent 2>/dev/null || true) &
fi
```

That's the entire implementation - visible in `install.sh`.

## Why We Do This

Silent counting helps us:

1. **Count Total Installations**
   Example: "500 install attempts this month"

2. **Prioritize Platform Support**
   Example: If 80% use Linux, focus Linux testing

3. **Understand Adoption Trends**
   Example: See growth over time

We **don't** track:
- Individual users
- Usage patterns
- What you build
- How you use HORUS

## Your Control

### Opt Out

Set an environment variable before running `install.sh`:

```bash
export HORUS_NO_TELEMETRY=1
./install.sh
```

Or add to your shell profile to disable permanently:

```bash
# Add to ~/.bashrc or ~/.zshrc
export HORUS_NO_TELEMETRY=1
```

### Verify It's Disabled

```bash
echo $HORUS_NO_TELEMETRY
# Should print: 1
```

Then run `./install.sh` - no ping will be sent.

## Data Storage & Retention

- **Storage:** Simple analytics endpoint (`https://telemetry.horus-registry.dev/count`)
- **Retention:** Aggregated counts stored indefinitely (e.g., "500 Linux installs")
- **No personal data:** Since we collect no personal info, there's nothing to delete
- **Aggregation:** Pure counting - we can't distinguish individual machines

## Third Parties

- Data is **not shared** with third parties
- Data is **not sold** to anyone
- We use it **only** for HORUS development

## Compliance

### GDPR (Europe)

Since we collect **no personal data**, GDPR doesn't apply. We don't collect any identifiers - just increment a counter.

### CCPA (California)

Since we collect **no personal information**, CCPA doesn't apply. We have no way to identify individuals.

## Open Source

The telemetry code is **fully open source**:

- `install.sh` lines 682-690 - Complete implementation (8 lines)
- View exactly what we collect and when

You can audit the code yourself: https://github.com/softmata/horus

## Comparison to Previous Versions

| Feature | v1.0 (Prompt) | v2.0 (First-run) | v3.0 (Install) |
|---------|---------------|------------------|----------------|
| Prompt |  Yes |  No |  No |
| When | Install | First horus run | Install |
| UUID |  Yes |  Yes |  No |
| Tracking | Individual | Individual | None |
| Data fields | 7 | 4 | 3 |
| Count metric | Unique users | Active users | Total installs |

## Why Install Count is Better

**Install count** (v3.0) is better than "unique users" (v2.0) because:

1. **More accurate metric** - Counts actual installation attempts
2. **Simpler** - No UUID, no flag files, just counting
3. **More private** - Can't track individuals at all
4. **Better for analytics** - Know reinstall frequency, installation success rate
5. **Honest** - We want to know "how many installs", not "how many users"

## Changes to This Policy

If we change what we collect, we will:
1. Update this document with a new version number
2. Update the "Last Updated" date
3. Announce changes in release notes

## Questions or Concerns

If you have questions about telemetry or privacy:

- **GitHub Issues:** https://github.com/softmata/horus/issues
- **View the code:** `install.sh` (lines 682-690)

## Summary

**In plain English:**

- One anonymous ping per install
- No UUID, no tracking, pure counting
- 3 fields: event, OS, timestamp
- Easy opt-out: `export HORUS_NO_TELEMETRY=1`
- 8 lines of code - verify it yourself

Thank you for helping us understand HORUS adoption!

---

**HORUS Team**
Open Source Robotics Framework
https://horus.rs
