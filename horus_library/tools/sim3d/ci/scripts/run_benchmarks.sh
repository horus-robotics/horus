#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="${BENCHMARK_OUTPUT_DIR:-benchmark-results}"
BASELINE_FILE="$PROJECT_DIR/ci/baseline.json"
RESULTS_FILE="$OUTPUT_DIR/results.json"
SUMMARY_FILE="$OUTPUT_DIR/summary.json"
COMPARISON_FILE="$OUTPUT_DIR/comparison.json"
REGRESSION_THRESHOLD="${REGRESSION_THRESHOLD:-10}"

cd "$PROJECT_DIR"

mkdir -p "$OUTPUT_DIR"

log() {
    echo "[$(date -u +"%Y-%m-%d %H:%M:%S UTC")] $*"
}

error() {
    echo "[ERROR] $*" >&2
}

run_criterion_benchmarks() {
    log "Running Criterion benchmarks..."

    cargo bench --no-default-features --features headless -- \
        --noplot \
        --save-baseline current \
        2>&1 | tee "$OUTPUT_DIR/criterion_output.txt"

    if [[ -d "target/criterion" ]]; then
        log "Criterion benchmarks completed successfully"
        return 0
    else
        error "Criterion output directory not found"
        return 1
    fi
}

parse_criterion_results() {
    log "Parsing Criterion results..."

    local results="{\"benchmarks\": {}, \"timestamp\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\", \"commit\": \"${GITHUB_SHA:-$(git rev-parse HEAD)}\"}"

    if [[ -d "target/criterion" ]]; then
        for bench_dir in target/criterion/*/new; do
            if [[ -d "$bench_dir" ]]; then
                local bench_name=$(basename "$(dirname "$bench_dir")")
                local estimates_file="$bench_dir/estimates.json"

                if [[ -f "$estimates_file" ]]; then
                    local mean_ns=$(jq -r '.mean.point_estimate // 0' "$estimates_file")
                    local std_ns=$(jq -r '.std_dev.point_estimate // 0' "$estimates_file")

                    local sample_file="$bench_dir/sample.json"
                    local min_ns=0
                    local max_ns=0

                    if [[ -f "$sample_file" ]]; then
                        min_ns=$(jq -r '[.times[]] | min // 0' "$sample_file")
                        max_ns=$(jq -r '[.times[]] | max // 0' "$sample_file")
                    fi

                    local throughput_per_sec=0
                    if (( $(echo "$mean_ns > 0" | bc -l) )); then
                        throughput_per_sec=$(echo "scale=2; 1000000000 / $mean_ns" | bc -l)
                    fi

                    results=$(echo "$results" | jq \
                        --arg name "$bench_name" \
                        --argjson mean "$mean_ns" \
                        --argjson std "$std_ns" \
                        --argjson min "$min_ns" \
                        --argjson max "$max_ns" \
                        --argjson throughput "$throughput_per_sec" \
                        '.benchmarks[$name] = {
                            "mean_ns": $mean,
                            "std_ns": $std,
                            "min_ns": $min,
                            "max_ns": $max,
                            "throughput_per_sec": $throughput
                        }')
                fi
            fi
        done
    fi

    echo "$results" > "$RESULTS_FILE"
    log "Results saved to $RESULTS_FILE"
}

generate_summary() {
    log "Generating summary..."

    if [[ ! -f "$RESULTS_FILE" ]]; then
        error "Results file not found"
        return 1
    fi

    local summary=$(jq '{
        "total_benchmarks": (.benchmarks | length),
        "timestamp": .timestamp,
        "commit": .commit,
        "benchmarks": [
            .benchmarks | to_entries[] | {
                "name": .key,
                "mean_ns": .value.mean_ns,
                "std_ns": .value.std_ns,
                "throughput_per_sec": .value.throughput_per_sec
            }
        ] | sort_by(.name)
    }' "$RESULTS_FILE")

    echo "$summary" > "$SUMMARY_FILE"
    log "Summary saved to $SUMMARY_FILE"

    echo ""
    echo "=========================================="
    echo "           BENCHMARK SUMMARY"
    echo "=========================================="
    echo ""

    jq -r '.benchmarks[] | "\(.name): \(.mean_ns / 1000 | floor)us (throughput: \(.throughput_per_sec | floor) ops/sec)"' "$SUMMARY_FILE"

    echo ""
    echo "=========================================="
}

compare_with_baseline() {
    log "Comparing with baseline..."

    if [[ ! -f "$BASELINE_FILE" ]]; then
        log "No baseline file found at $BASELINE_FILE, skipping comparison"
        echo '{"has_regression": false, "message": "No baseline available", "comparisons": {}}' > "$COMPARISON_FILE"
        return 0
    fi

    if [[ ! -f "$RESULTS_FILE" ]]; then
        error "Results file not found for comparison"
        return 1
    fi

    local has_regression=false
    local comparisons="{}"

    while IFS= read -r bench_name; do
        local baseline_mean=$(jq -r ".benchmarks[\"$bench_name\"].mean_ns // 0" "$BASELINE_FILE")
        local current_mean=$(jq -r ".benchmarks[\"$bench_name\"].mean_ns // 0" "$RESULTS_FILE")

        if (( $(echo "$baseline_mean > 0" | bc -l) )) && (( $(echo "$current_mean > 0" | bc -l) )); then
            local change_percent=$(echo "scale=4; (($current_mean - $baseline_mean) / $baseline_mean) * 100" | bc -l)
            local change_abs=$(echo "scale=2; $current_mean - $baseline_mean" | bc -l)

            local status="ok"
            if (( $(echo "$change_percent > $REGRESSION_THRESHOLD" | bc -l) )); then
                status="regression"
                has_regression=true
            elif (( $(echo "$change_percent < -$REGRESSION_THRESHOLD" | bc -l) )); then
                status="improvement"
            fi

            comparisons=$(echo "$comparisons" | jq \
                --arg name "$bench_name" \
                --argjson baseline "$baseline_mean" \
                --argjson current "$current_mean" \
                --argjson change_percent "$change_percent" \
                --argjson change_abs "$change_abs" \
                --arg status "$status" \
                '.[$name] = {
                    "baseline_ns": $baseline,
                    "current_ns": $current,
                    "change_percent": $change_percent,
                    "change_abs_ns": $change_abs,
                    "status": $status
                }')
        fi
    done < <(jq -r '.benchmarks | keys[]' "$RESULTS_FILE")

    local comparison_result=$(jq -n \
        --argjson has_regression "$has_regression" \
        --argjson comparisons "$comparisons" \
        --arg threshold "$REGRESSION_THRESHOLD" \
        '{
            "has_regression": $has_regression,
            "threshold_percent": ($threshold | tonumber),
            "comparisons": $comparisons
        }')

    echo "$comparison_result" > "$COMPARISON_FILE"
    log "Comparison saved to $COMPARISON_FILE"

    echo ""
    echo "=========================================="
    echo "        BASELINE COMPARISON"
    echo "=========================================="
    echo ""

    jq -r '.comparisons | to_entries[] |
        "\(.key): \(.value.change_percent | . * 100 | round / 100)% [\(.value.status)]"' "$COMPARISON_FILE"

    echo ""

    if [[ "$has_regression" == "true" ]]; then
        echo "WARNING: Performance regression detected!"
        echo "Benchmarks exceeding ${REGRESSION_THRESHOLD}% threshold:"
        jq -r '.comparisons | to_entries[] | select(.value.status == "regression") |
            "  - \(.key): +\(.value.change_percent | . * 100 | round / 100)%"' "$COMPARISON_FILE"
    else
        echo "All benchmarks within acceptable range."
    fi

    echo "=========================================="

    return 0
}

check_regression() {
    if [[ -f "$COMPARISON_FILE" ]]; then
        local has_regression=$(jq -r '.has_regression' "$COMPARISON_FILE")
        if [[ "$has_regression" == "true" ]]; then
            error "Performance regression detected! See comparison results above."
            return 1
        fi
    fi
    return 0
}

main() {
    log "Starting benchmark suite for sim3d"
    log "Project directory: $PROJECT_DIR"
    log "Output directory: $OUTPUT_DIR"
    log "Regression threshold: ${REGRESSION_THRESHOLD}%"

    if ! command -v jq &> /dev/null; then
        error "jq is required but not installed"
        exit 1
    fi

    if ! command -v bc &> /dev/null; then
        error "bc is required but not installed"
        exit 1
    fi

    run_criterion_benchmarks

    parse_criterion_results

    generate_summary

    compare_with_baseline

    log "Benchmark suite completed"

    if [[ "${CI:-false}" == "true" ]]; then
        if ! check_regression; then
            exit 1
        fi
    fi

    exit 0
}

main "$@"
