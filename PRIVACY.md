# HORUS Privacy Policy - Anonymous Telemetry

**Last Updated:** 2025-11-02
**Version:** 1.0

## Overview

HORUS uses **opt-in anonymous telemetry** to help us improve the framework. This document explains exactly what we collect, why, and how you can control it.

## What We Collect

When you **opt in** to telemetry during installation, we collect:

| Data Point | Example | Purpose |
|------------|---------|---------|
| Event type | `install`, `update`, `uninstall` | Understand usage patterns |
| Event status | `success`, `failure` | Identify installation issues |
| HORUS version | `0.1.4` | Track version adoption |
| Anonymous install ID | `a7f3e9c2...` (random hash) | Count unique installs (no tracking) |
| Operating system | `Linux`, `Darwin` (macOS) | Prioritize platform support |
| Architecture | `x86_64`, `arm64` | Ensure compatibility |
| Timestamp | `1699123456` (Unix time) | Understand usage trends |

## What We DON'T Collect

We **never** collect:

- Personal information (name, email, address)
- IP addresses or location data
- Your code or project data
- File paths or directory names
- Environment variables
- Command history
- Any identifiable information

## How It Works

### 1. First-Time Prompt

When you run `./install.sh` for the first time, you'll see:

```
═══════════════════════════════════════════════════════════
   Anonymous Telemetry
═══════════════════════════════════════════════════════════

Help us improve HORUS by sharing anonymous usage statistics!

What we collect:
  • Event type (install/update/uninstall)
  • OS/Platform (Linux/macOS)
  • Architecture (x86_64/arm64)
  • HORUS version
  • Anonymous install ID (random hash)
  • Timestamp

What we DON'T collect:
  • Personal information (name, email, IP address)
  • Your code or project data
  • File paths or directory names
  • Any identifiable information

? Enable anonymous telemetry? [Y/n]:
```

### 2. Anonymous Install ID

If you opt in, we generate a random 32-character hash:

```bash
# Generated locally, never sent to us before opt-in
INSTALL_ID=$(cat /dev/urandom | tr -dc 'a-f0-9' | fold -w 32 | head -n 1)
# Example: a7f3e9c2d1b4f5e8c9a6d2e1f4b7c5a8
```

This ID is:
- **Completely random** (not based on your machine or personal info)
- **Locally generated** (we don't assign it)
- **Anonymous** (can't be traced back to you)
- **Used only for counting** unique installs

### 3. Data Transmission

When an event occurs (install/update), we send a JSON payload:

```json
{
  "event": "install",
  "status": "success",
  "version": "0.1.4",
  "install_id": "a7f3e9c2d1b4f5e8c9a6d2e1f4b7c5a8",
  "os": "Linux",
  "arch": "x86_64",
  "timestamp": 1699123456
}
```

The request:
- **Times out after 5 seconds** (doesn't delay your install)
- **Fails silently** (never breaks your installation)
- **HTTPS only** (encrypted in transit)
- **No cookies or tracking**

## Why We Do This

Anonymous telemetry helps us:

1. **Prioritize Platform Support**
   Example: If 80% of users are on Linux, we'll focus Linux testing.

2. **Track Adoption**
   Example: See if people are upgrading to new versions.

3. **Identify Installation Issues**
   Example: If 50% of macOS installs fail, we know there's a problem.

4. **Understand Usage Trends**
   Example: See when users typically install HORUS.

## Your Control

### Check Status

```bash
cat ~/.horus/telemetry.conf
```

Output:
```
enabled=true
install_id=a7f3e9c2d1b4f5e8c9a6d2e1f4b7c5a8
created=1699123456
```

### Disable Telemetry

**Option 1: Delete the config file**
```bash
rm ~/.horus/telemetry.conf
```

**Option 2: Edit the config**
```bash
echo "enabled=false" > ~/.horus/telemetry.conf
```

After disabling, **no data will be sent** on future installs/updates.

### Re-enable Telemetry

```bash
rm ~/.horus/telemetry.conf
./install.sh  # Will ask again
```

## Data Storage & Retention

- **Storage:** We use a simple analytics endpoint (`https://api.horus.rs/telemetry`)
- **Retention:** Event data is aggregated and stored for 1 year
- **No personal data:** Since we collect no personal info, there's nothing to delete
- **Aggregation:** Data is used only in aggregate (e.g., "500 Linux installs this month")

## Third Parties

- Data is **not shared** with third parties
- Data is **not sold** to anyone
- We use it **only** for HORUS development

## Compliance

### GDPR (Europe)

Since we collect **no personal data**, GDPR doesn't apply. However, we still:
- Provide opt-in consent
- Explain what we collect
- Allow easy opt-out
- Don't track individuals

### CCPA (California)

Since we collect **no personal information**, CCPA doesn't apply. The random install ID cannot be used to identify you.

## Open Source

The telemetry code is **fully open source**:

- `scripts/telemetry.sh` - View exactly what we collect
- `install.sh` - See how it's integrated
- `update.sh` - See update tracking
- `recovery_install.sh` - See recovery tracking

You can audit the code yourself: https://github.com/horus-robotics/horus

## Changes to This Policy

If we change what we collect, we will:
1. Update this document with a new version number
2. Update the "Last Updated" date
3. Require re-consent if changes are material

## Questions or Concerns

If you have questions about telemetry or privacy:

- **GitHub Issues:** https://github.com/horus-robotics/horus/issues
- **View the code:** `scripts/telemetry.sh`

## Summary

**In plain English:**

- We count installs to understand adoption
- We don't collect personal info
- You opt-in during first install
- Easy to disable anytime
- Code is open source - verify it yourself

Thank you for helping us improve HORUS!

---

**HORUS Team**
Open Source Robotics Framework
https://horus.rs
