#!/bin/bash
# HORUS IPC Benchmark Restore Script
# Restores system to normal operation after benchmarking

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}   HORUS IPC Benchmark - System Restore${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo ""

RESTORE_FILE="/tmp/horus_benchmark_restore.sh"

# Check if auto-generated restore script exists
if [ -f "$RESTORE_FILE" ]; then
    echo -e "${YELLOW}Using auto-generated restore script...${NC}"
    echo ""

    # Execute the saved restore commands
    bash "$RESTORE_FILE"

    echo -e "  ${GREEN}✓${NC} Restored settings from benchmark_setup.sh"
    echo ""

    rm -f "$RESTORE_FILE"
    echo -e "${GREEN}Automatic restore complete!${NC}"
    echo ""
else
    echo -e "${YELLOW}No auto-restore script found. Using defaults...${NC}"
    echo ""

    # Manual restore with safe defaults

    # 1. CPU Frequency Governor
    echo -e "${YELLOW}[1/5] Restoring CPU frequency governor...${NC}"
    if command -v cpupower &> /dev/null; then
        sudo cpupower frequency-set --governor ondemand > /dev/null 2>&1 || \
        sudo cpupower frequency-set --governor powersave > /dev/null 2>&1 || true
        echo -e "  ${GREEN}✓${NC} Set governor to ondemand/powersave"

        # Restore max frequency
        if [ -f /sys/devices/system/cpu/cpufreq/policy0/cpuinfo_max_freq ]; then
            MAX_FREQ=$(cat /sys/devices/system/cpu/cpufreq/policy0/cpuinfo_max_freq)
            echo "$MAX_FREQ" | sudo tee /sys/devices/system/cpu/cpufreq/policy*/scaling_max_freq > /dev/null 2>&1 || true
            echo -e "  ${GREEN}✓${NC} Restored max frequency"
        fi
    fi

    # 2. Turbo Boost
    echo ""
    echo -e "${YELLOW}[2/5] Enabling Turbo Boost...${NC}"
    if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
        echo 0 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo > /dev/null
        echo -e "  ${GREEN}✓${NC} Intel Turbo Boost enabled"
    elif [ -f /sys/devices/system/cpu/cpufreq/boost ]; then
        echo 1 | sudo tee /sys/devices/system/cpu/cpufreq/boost > /dev/null
        echo -e "  ${GREEN}✓${NC} AMD Boost enabled"
    fi

    # 3. ASLR
    echo ""
    echo -e "${YELLOW}[3/5] Re-enabling ASLR...${NC}"
    echo 2 | sudo tee /proc/sys/kernel/randomize_va_space > /dev/null
    echo -e "  ${GREEN}✓${NC} ASLR restored to default (2)"

    # 4. IRQ Affinity
    echo ""
    echo -e "${YELLOW}[4/5] Restoring IRQ affinity...${NC}"
    # Reset to all cores (0xFF for 8 cores, 0xFFFF for 16 cores)
    IRQ_COUNT=0
    for irq in /proc/irq/*/smp_affinity; do
        if [ -w "$irq" ]; then
            echo "ffff" | sudo tee "$irq" > /dev/null 2>&1 && IRQ_COUNT=$((IRQ_COUNT + 1)) || true
        fi
    done
    echo -e "  ${GREEN}✓${NC} Restored $IRQ_COUNT IRQs to all cores"

    # 5. Services
    echo ""
    echo -e "${YELLOW}[5/5] Restarting services...${NC}"
    RESTARTED=()
    for service in bluetooth cups whoopsie snapd packagekit; do
        if systemctl is-enabled --quiet "$service.service" 2>/dev/null; then
            sudo systemctl start "$service.service" 2>/dev/null && RESTARTED+=("$service") || true
        fi
    done

    if [ ${#RESTARTED[@]} -gt 0 ]; then
        echo -e "  ${GREEN}✓${NC} Restarted services: ${RESTARTED[*]}"
    else
        echo -e "  ${CYAN}•${NC} No services to restart"
    fi
fi

echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   System Restored!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "System returned to normal operation:"
echo -e "  ${GREEN}✓${NC} CPU frequency governor restored"
echo -e "  ${GREEN}✓${NC} Turbo boost re-enabled"
echo -e "  ${GREEN}✓${NC} ASLR re-enabled"
echo -e "  ${GREEN}✓${NC} IRQ affinity restored"
echo -e "  ${GREEN}✓${NC} Services restarted"
echo ""
