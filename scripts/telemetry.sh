#!/bin/bash
# HORUS Anonymous Telemetry Module
# Privacy-first usage analytics

# Telemetry endpoint (Cloudflare Workers - Custom Domain)
TELEMETRY_ENDPOINT="https://telemetry.horus-registry.dev/telemetry"

# Config file location
TELEMETRY_CONFIG="$HOME/.horus/telemetry.conf"

# Check if telemetry is enabled
is_telemetry_enabled() {
    if [ -f "$TELEMETRY_CONFIG" ]; then
        grep -q "enabled=true" "$TELEMETRY_CONFIG" 2>/dev/null
        return $?
    fi
    return 1
}

# Ask user for telemetry consent (only first time)
ask_telemetry_consent() {
    # Skip if already configured
    if [ -f "$TELEMETRY_CONFIG" ]; then
        return
    fi

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}   Anonymous Telemetry${NC}"
    echo -e "${CYAN}═══════════════════════════════════════════════════════════${NC}"
    echo ""
    echo "Help us improve HORUS by sharing anonymous usage statistics!"
    echo ""
    echo -e "${GREEN}What we collect:${NC}"
    echo "  • Event type (install/update/uninstall)"
    echo "  • OS/Platform (Linux/macOS)"
    echo "  • Architecture (x86_64/arm64)"
    echo "  • HORUS version"
    echo "  • Anonymous install ID (random hash)"
    echo "  • Timestamp"
    echo ""
    echo -e "${RED}What we DON'T collect:${NC}"
    echo "  • Personal information (name, email, IP address)"
    echo "  • Your code or project data"
    echo "  • File paths or directory names"
    echo "  • Any identifiable information"
    echo ""
    echo -e "${BLUE}Why this helps:${NC}"
    echo "  • Understand platform usage (prioritize Linux vs macOS support)"
    echo "  • Track adoption and growth"
    echo "  • Identify installation issues"
    echo ""
    echo "You can disable this anytime: rm $TELEMETRY_CONFIG"
    echo ""

    read -p "$(echo -e ${YELLOW}?${NC}) Enable anonymous telemetry? [Y/n]: " -n 1 -r
    echo
    echo ""

    mkdir -p "$(dirname "$TELEMETRY_CONFIG")"

    if [[ $REPLY =~ ^[Nn]$ ]]; then
        echo "enabled=false" > "$TELEMETRY_CONFIG"
        echo -e "${YELLOW}⚠${NC}  Telemetry disabled"
    else
        # Generate anonymous install ID
        INSTALL_ID=$(cat /dev/urandom | tr -dc 'a-f0-9' | fold -w 32 | head -n 1)
        echo "enabled=true" > "$TELEMETRY_CONFIG"
        echo "install_id=$INSTALL_ID" >> "$TELEMETRY_CONFIG"
        echo "created=$(date +%s)" >> "$TELEMETRY_CONFIG"
        echo -e "${GREEN}✓${NC} Telemetry enabled (thank you!)"
    fi
    echo ""
}

# Get anonymous install ID
get_install_id() {
    if [ -f "$TELEMETRY_CONFIG" ]; then
        grep "install_id=" "$TELEMETRY_CONFIG" | cut -d= -f2
    else
        echo "unknown"
    fi
}

# Send telemetry event
send_telemetry_event() {
    local event_type="$1"  # install, update, uninstall
    local event_status="$2"  # success, failure
    local version="$3"

    # Skip if telemetry disabled
    if ! is_telemetry_enabled; then
        return 0
    fi

    # Get system info
    local os_type=$(uname -s)
    local arch=$(uname -m)
    local install_id=$(get_install_id)

    # Prepare JSON payload
    local payload=$(cat <<EOF
{
  "event": "$event_type",
  "status": "$event_status",
  "version": "$version",
  "install_id": "$install_id",
  "os": "$os_type",
  "arch": "$arch",
  "timestamp": $(date +%s)
}
EOF
)

    # Send to endpoint (with timeout, fail silently)
    # Using curl with --max-time to prevent hanging
    if command -v curl &> /dev/null; then
        curl -X POST \
             -H "Content-Type: application/json" \
             -d "$payload" \
             --max-time 5 \
             --silent \
             --show-error \
             "$TELEMETRY_ENDPOINT" &>/dev/null || true
    elif command -v wget &> /dev/null; then
        wget --timeout=5 \
             --quiet \
             --post-data="$payload" \
             --header="Content-Type: application/json" \
             -O /dev/null \
             "$TELEMETRY_ENDPOINT" &>/dev/null || true
    fi

    # Never fail the install if telemetry fails
    return 0
}

# Export functions
export -f is_telemetry_enabled
export -f ask_telemetry_consent
export -f send_telemetry_event
export -f get_install_id
