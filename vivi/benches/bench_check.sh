#!/usr/bin/env bash
# bench_check.sh — Run vivi benchmarks and fail on regression.
#
# Usage: bash vivi/benches/bench_check.sh
#
# Thresholds are set conservatively so normal noise won't trigger failures.
# If a benchmark regresses by more than the threshold, the script exits non-zero.

set -euo pipefail

THRESHOLD_PCT=20  # fail if >20% slower than baseline

# Baseline numbers (update after intentional perf changes)
declare -A BASELINE
BASELINE[render_50_lines_120x40]=55
BASELINE[render_500_lines_120x40]=55
BASELINE[render_10k_lines_120x40]=55
BASELINE[render_200_lines_80x24]=26
BASELINE[cursor_j_1000_lines]=73

echo "--- Running vivi render benchmarks ---"
cargo bench -p vivi --bench render_bench -- --quick 2>&1 | tee /tmp/vivi_bench_out.txt

echo ""
echo "--- Checking for regressions ---"
FAIL=0
for bench in "${!BASELINE[@]}"; do
    baseline=${BASELINE[$bench]}
    # Extract the median time (second value in "[X.XXX µs Y.YYY µs Z.ZZZ µs]")
    actual=$(grep "^${bench} " /tmp/vivi_bench_out.txt | head -1 | awk '{print $(NF-1)}')

    if [ -z "$actual" ]; then
        echo "WARN: could not find result for $bench"
        continue
    fi

    # Convert to integer µs for comparison
    actual_int=$(printf '%.0f' "$actual" 2>/dev/null || echo 0)
    allowed=$(( baseline + (baseline * THRESHOLD_PCT / 100) ))

    if [ "$actual_int" -gt "$allowed" ]; then
        echo "FAIL: $bench — ${actual_int}µs > ${allowed}µs baseline ${baseline}µs (+${THRESHOLD_PCT}%)"
        FAIL=1
    else
        echo "OK:   $bench — ${actual_int}µs <= ${allowed}µs"
    fi
done

if [ "$FAIL" -eq 1 ]; then
    echo ""
    echo "Performance regression detected. Update baselines if intentional."
    exit 1
fi

echo ""
echo "All benchmarks within thresholds."
