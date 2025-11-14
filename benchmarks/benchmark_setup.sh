#!/bin/bash
# HORUS IPC Benchmark Setup Script
# Configures the system for optimal benchmark performance

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}   HORUS IPC Benchmark - System Setup${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════${NC}"
echo ""

# Save original settings for restoration
RESTORE_FILE="/tmp/horus_benchmark_restore.sh"
echo "#!/bin/bash" > "$RESTORE_FILE"
echo "# Auto-generated restore script" >> "$RESTORE_FILE"
chmod +x "$RESTORE_FILE"

# ============================================================================
# 1. Check CPU Features
# ============================================================================
echo -e "${YELLOW}[1/8] Checking CPU features...${NC}"

if grep -q "constant_tsc" /proc/cpuinfo; then
    echo -e "  ${GREEN}[OK]${NC} CPU has constant TSC"
else
    echo -e "  ${RED}[FAIL]${NC} WARNING: CPU does not have constant TSC!"
    echo -e "      Results may be inaccurate."
fi

if grep -q "nonstop_tsc" /proc/cpuinfo; then
    echo -e "  ${GREEN}[OK]${NC} CPU has nonstop TSC"
else
    echo -e "  ${YELLOW}[WARNING]${NC} WARNING: CPU may not have nonstop TSC"
fi

CPU_MODEL=$(grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)
echo -e "  ${CYAN}•${NC} CPU: ${CPU_MODEL}"

# ============================================================================
# 2. Disable CPU Frequency Scaling
# ============================================================================
echo ""
echo -e "${YELLOW}[2/8] Configuring CPU frequency governor...${NC}"

if command -v cpupower &> /dev/null; then
    # Save current governor
    CURRENT_GOV=$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "unknown")
    echo "echo '$CURRENT_GOV' | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor > /dev/null" >> "$RESTORE_FILE"

    # Set to performance
    sudo cpupower frequency-set --governor performance > /dev/null 2>&1
    echo -e "  ${GREEN}[OK]${NC} Set CPU governor to 'performance'"
    echo -e "      (was: ${CURRENT_GOV})"

    # Lock to base frequency (disable turbo scaling)
    if [ -f /sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq ]; then
        BASE_FREQ=$(cat /sys/devices/system/cpu/cpufreq/policy0/base_frequency 2>/dev/null || cat /sys/devices/system/cpu/cpufreq/policy0/cpuinfo_min_freq)
        if [ -n "$BASE_FREQ" ]; then
            CURRENT_MAX=$(cat /sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq)
            echo "echo '$CURRENT_MAX' | sudo tee /sys/devices/system/cpu/cpufreq/policy*/scaling_max_freq > /dev/null" >> "$RESTORE_FILE"

            echo "$BASE_FREQ" | sudo tee /sys/devices/system/cpu/cpufreq/policy*/scaling_max_freq > /dev/null 2>&1 || true
            echo -e "  ${GREEN}[OK]${NC} Locked CPU to base frequency"
        fi
    fi
else
    echo -e "  ${YELLOW}[WARNING]${NC} cpupower not found (install linux-tools-common)"
    echo -e "      Frequency scaling may affect results"
fi

# ============================================================================
# 3. Disable Turbo Boost
# ============================================================================
echo ""
echo -e "${YELLOW}[3/8] Disabling Turbo Boost...${NC}"

# Intel
if [ -f /sys/devices/system/cpu/intel_pstate/no_turbo ]; then
    CURRENT_TURBO=$(cat /sys/devices/system/cpu/intel_pstate/no_turbo)
    echo "echo '$CURRENT_TURBO' | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo > /dev/null" >> "$RESTORE_FILE"

    echo 1 | sudo tee /sys/devices/system/cpu/intel_pstate/no_turbo > /dev/null
    echo -e "  ${GREEN}[OK]${NC} Intel Turbo Boost disabled"
# AMD
elif [ -f /sys/devices/system/cpu/cpufreq/boost ]; then
    CURRENT_BOOST=$(cat /sys/devices/system/cpu/cpufreq/boost)
    echo "echo '$CURRENT_BOOST' | sudo tee /sys/devices/system/cpu/cpufreq/boost > /dev/null" >> "$RESTORE_FILE"

    echo 0 | sudo tee /sys/devices/system/cpu/cpufreq/boost > /dev/null
    echo -e "  ${GREEN}[OK]${NC} AMD Boost disabled"
else
    echo -e "  ${YELLOW}[WARNING]${NC} Turbo boost control not found"
fi

# ============================================================================
# 4. Check Core Isolation
# ============================================================================
echo ""
echo -e "${YELLOW}[4/8] Checking core isolation...${NC}"

if grep -q "isolcpus" /proc/cmdline; then
    ISOLATED=$(grep -o "isolcpus=[^ ]*" /proc/cmdline | cut -d= -f2)
    echo -e "  ${GREEN}[OK]${NC} Cores isolated: ${ISOLATED}"
else
    echo -e "  ${YELLOW}[WARNING]${NC} No cores isolated"
    echo -e "      For best results, add 'isolcpus=0,1' to kernel command line"
    echo -e "      Edit /etc/default/grub and add to GRUB_CMDLINE_LINUX:"
    echo -e "      ${CYAN}isolcpus=0,1${NC}"
    echo -e "      Then run: sudo update-grub && reboot"
fi

# ============================================================================
# 5. Move IRQs Away from Benchmark Cores
# ============================================================================
echo ""
echo -e "${YELLOW}[5/8] Configuring IRQ affinity...${NC}"

# Move IRQs to cores 2+ (away from 0,1)
# Affinity mask: 0x03 = cores 0-1, 0xFC = cores 2-7, 0xFFFC = cores 2-15
IRQ_COUNT=0
for irq in /proc/irq/*/smp_affinity; do
    if [ -w "$irq" ]; then
        # Try to set affinity to cores 2+ (mask 0xfc for 8 cores)
        echo "fc" | sudo tee "$irq" > /dev/null 2>&1 && IRQ_COUNT=$((IRQ_COUNT + 1)) || true
    fi
done

echo -e "  ${GREEN}[OK]${NC} Moved $IRQ_COUNT IRQs away from cores 0-1"
echo -e "      (IRQs now on cores 2+)"

# ============================================================================
# 6. Disable ASLR (Address Space Layout Randomization)
# ============================================================================
echo ""
echo -e "${YELLOW}[6/8] Disabling ASLR...${NC}"

CURRENT_ASLR=$(cat /proc/sys/kernel/randomize_va_space)
echo "echo '$CURRENT_ASLR' | sudo tee /proc/sys/kernel/randomize_va_space > /dev/null" >> "$RESTORE_FILE"

echo 0 | sudo tee /proc/sys/kernel/randomize_va_space > /dev/null
echo -e "  ${GREEN}[OK]${NC} ASLR disabled (was: $CURRENT_ASLR)"
echo -e "      Reduces memory layout variance"

# ============================================================================
# 7. Drop Caches and Sync
# ============================================================================
echo ""
echo -e "${YELLOW}[7/8] Dropping caches...${NC}"

sync
echo 3 | sudo tee /proc/sys/vm/drop_caches > /dev/null
echo -e "  ${GREEN}[OK]${NC} Page cache, dentries, and inodes dropped"

# ============================================================================
# 8. Stop Background Services
# ============================================================================
echo ""
echo -e "${YELLOW}[8/8] Stopping background services...${NC}"

STOPPED_SERVICES=()

# Try to stop common services that may interfere
for service in bluetooth cups whoopsie snapd packagekit; do
    if systemctl is-active --quiet "$service.service" 2>/dev/null; then
        sudo systemctl stop "$service.service" 2>/dev/null && STOPPED_SERVICES+=("$service") || true
        echo "sudo systemctl start $service.service" >> "$RESTORE_FILE"
    fi
done

if [ ${#STOPPED_SERVICES[@]} -gt 0 ]; then
    echo -e "  ${GREEN}[OK]${NC} Stopped services: ${STOPPED_SERVICES[*]}"
else
    echo -e "  ${CYAN}•${NC} No interfering services found"
fi

# ============================================================================
# Summary
# ============================================================================
echo ""
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo -e "${GREEN}   Setup Complete!${NC}"
echo -e "${GREEN}═══════════════════════════════════════════════════════${NC}"
echo ""
echo -e "System configured for benchmarking:"
echo -e "  ${GREEN}[OK]${NC} CPU frequency locked to base"
echo -e "  ${GREEN}[OK]${NC} Turbo boost disabled"
echo -e "  ${GREEN}[OK]${NC} IRQs moved away from cores 0-1"
echo -e "  ${GREEN}[OK]${NC} ASLR disabled"
echo -e "  ${GREEN}[OK]${NC} Caches dropped"
echo ""
echo -e "${CYAN}Next steps:${NC}"
echo -e "  1. Build benchmark:  ${CYAN}cargo build --release --bin ipc_benchmark${NC}"
echo -e "  2. Run benchmark:    ${CYAN}./target/release/ipc_benchmark${NC}"
echo -e "  3. Restore system:   ${CYAN}./benchmarks/benchmark_restore.sh${NC}"
echo ""
echo -e "Restore script saved to: ${CYAN}$RESTORE_FILE${NC}"
echo ""
