#!/usr/bin/env bash
# =============================================================================
# Bittime CLI — End-to-End Test Suite
# =============================================================================
# Tests all CLI commands against the live Bittime API.
# Public endpoints require no credentials.
# Private endpoints require BITTIME_API_KEY / BITTIME_API_SECRET or config.
#
# Usage:
#   ./scripts/e2e_test.sh              # Run all tests
#   ./scripts/e2e_test.sh --public     # Run public tests only
#   ./scripts/e2e_test.sh --private    # Run private tests only
# =============================================================================

set -euo pipefail

BINARY="${BITTIME_BIN:-./target/debug/bittime}"
PAIR="${BITTIME_TEST_PAIR:-USDTIDR}"
PAIR_LOWER=$(echo "$PAIR" | tr '[:upper:]' '[:lower:]')
TEST_COIN="${BITTIME_TEST_COIN:-usdt}"

# Counters
PASS=0
FAIL=0
SKIP=0
TOTAL=0

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# =============================================================================
# Helpers
# =============================================================================

log_header() {
    echo ""
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════════${NC}"
    echo -e "${CYAN}${BOLD}  $1${NC}"
    echo -e "${CYAN}${BOLD}══════════════════════════════════════════════${NC}"
}

run_test() {
    local description="$1"
    shift
    TOTAL=$((TOTAL + 1))

    echo -n "  [$TOTAL] $description ... "

    local output
    local exit_code=0
    output=$("$@" 2>&1) || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        echo -e "${GREEN}PASS${NC}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}FAIL${NC} (exit=$exit_code)"
        echo "       CMD: $*"
        echo "       OUT: $(echo "$output" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

run_test_json() {
    local description="$1"
    shift
    TOTAL=$((TOTAL + 1))

    echo -n "  [$TOTAL] $description ... "

    local output
    local exit_code=0
    output=$("$@" 2>&1) || exit_code=$?

    if [ $exit_code -eq 0 ]; then
        # Verify it's valid JSON
        if echo "$output" | python3 -c "import sys, json; json.load(sys.stdin)" 2>/dev/null; then
            echo -e "${GREEN}PASS${NC} (valid JSON)"
            PASS=$((PASS + 1))
        else
            echo -e "${RED}FAIL${NC} (invalid JSON)"
            echo "       OUT: $(echo "$output" | head -3)"
            FAIL=$((FAIL + 1))
        fi
    else
        echo -e "${RED}FAIL${NC} (exit=$exit_code)"
        echo "       CMD: $*"
        echo "       OUT: $(echo "$output" | head -3)"
        FAIL=$((FAIL + 1))
    fi
}

skip_test() {
    local description="$1"
    TOTAL=$((TOTAL + 1))
    SKIP=$((SKIP + 1))
    echo -e "  [$TOTAL] $description ... ${YELLOW}SKIP${NC}"
}

check_credentials() {
    $BINARY auth test >/dev/null 2>&1
    return $?
}

# =============================================================================
# Parse args
# =============================================================================

RUN_PUBLIC=true
RUN_PRIVATE=true

if [[ "${1:-}" == "--public" ]]; then
    RUN_PRIVATE=false
elif [[ "${1:-}" == "--private" ]]; then
    RUN_PUBLIC=false
fi

# =============================================================================
# Build
# =============================================================================

echo -e "${BOLD}Building bittime-cli ...${NC}"
cargo build 2>&1 | tail -1

if [ ! -f "$BINARY" ]; then
    echo -e "${RED}Binary not found at $BINARY${NC}"
    exit 1
fi

echo -e "${GREEN}Binary: $BINARY${NC}"
echo -e "Test pair: ${CYAN}$PAIR${NC}"
echo ""

# =============================================================================
# PUBLIC MARKET TESTS
# =============================================================================

if $RUN_PUBLIC; then

log_header "PUBLIC — Market Data"

run_test "market ping (table)" \
    $BINARY market ping

run_test "market ping (json)" \
    $BINARY -o json market ping

run_test "market server-time (table)" \
    $BINARY market server-time

run_test_json "market server-time (json)" \
    $BINARY -o json market server-time

run_test "market exchange-info" \
    $BINARY -o json market exchange-info

run_test "market ticker $PAIR (table)" \
    $BINARY market ticker "$PAIR"

run_test_json "market ticker $PAIR (json)" \
    $BINARY -o json market ticker "$PAIR"

run_test "market ticker-all (json)" \
    $BINARY -o json market ticker-all

run_test_json "market price $PAIR" \
    $BINARY -o json market price "$PAIR"

run_test_json "market book-ticker $PAIR" \
    $BINARY -o json market book-ticker "$PAIR"

run_test "market orderbook $PAIR (table)" \
    $BINARY market orderbook "$PAIR" -l 5

run_test_json "market orderbook $PAIR (json)" \
    $BINARY -o json market orderbook "$PAIR" -l 5

run_test_json "market trades $PAIR (limit=5)" \
    $BINARY -o json market trades "$PAIR" -l 5

run_test_json "market agg-trades $PAIR (limit=5)" \
    $BINARY -o json market agg-trades "$PAIR" -l 5

log_header "PUBLIC — CLI Features"

run_test "--help" \
    $BINARY --help

run_test "--version" \
    $BINARY --version

run_test "market --help" \
    $BINARY market --help

run_test "account --help" \
    $BINARY account --help

run_test "trade --help" \
    $BINARY trade --help

run_test "funding --help" \
    $BINARY funding --help

run_test "ws --help" \
    $BINARY ws --help

run_test "auth --help" \
    $BINARY auth --help

fi  # RUN_PUBLIC

# =============================================================================
# PRIVATE ACCOUNT TESTS
# =============================================================================

if $RUN_PRIVATE; then

log_header "PRIVATE — Account & Trade (requires credentials)"

HAS_CREDS=false
if check_credentials; then
    HAS_CREDS=true
    echo -e "  ${GREEN}Credentials verified ✓${NC}"
else
    echo -e "  ${YELLOW}No valid credentials — skipping private tests${NC}"
    echo -e "  Configure with: ${CYAN}bittime auth set --api-key KEY --api-secret SECRET${NC}"
fi

if $HAS_CREDS; then
    run_test "auth test" \
        $BINARY auth test

    run_test "auth show" \
        $BINARY auth show

    run_test "account info (table)" \
        $BINARY account info

    run_test_json "account info (json)" \
        $BINARY -o json account info

    run_test "account balance (table)" \
        $BINARY account balance

    run_test_json "account balance (json)" \
        $BINARY -o json account balance

    run_test_json "account info-v2" \
        $BINARY -o json account info-v2

    run_test_json "account assets $TEST_COIN" \
        $BINARY -o json account assets "$TEST_COIN"

    run_test_json "account trades $PAIR" \
        $BINARY -o json account trades "$PAIR"

    run_test_json "account trades-v2 $PAIR" \
        $BINARY -o json account trades-v2 "$PAIR"

    run_test "trade open-orders $PAIR" \
        $BINARY -o json trade open-orders "$PAIR"

    run_test "trade all-orders $PAIR" \
        $BINARY -o json trade all-orders "$PAIR"

    run_test "trade pending-orders $PAIR" \
        $BINARY -o json trade pending-orders "$PAIR"

    run_test "trade book-orders $PAIR" \
        $BINARY -o json trade book-orders "$PAIR"

    run_test_json "market historical-trades $PAIR" \
        $BINARY -o json market historical-trades "$PAIR" -l 5

    # Funding read-only tests
    run_test "funding withdraw-history" \
        $BINARY -o json funding withdraw-history

    run_test "funding deposit-history" \
        $BINARY -o json funding deposit-history

    run_test "funding otc-deposit-history" \
        $BINARY -o json funding otc-deposit-history

    run_test "funding otc-withdraw-history" \
        $BINARY -o json funding otc-withdraw-history

else
    skip_test "auth test"
    skip_test "auth show"
    skip_test "account info"
    skip_test "account balance"
    skip_test "account info-v2"
    skip_test "account assets"
    skip_test "account trades"
    skip_test "account trades-v2"
    skip_test "trade open-orders"
    skip_test "trade all-orders"
    skip_test "trade pending-orders"
    skip_test "trade book-orders"
    skip_test "market historical-trades"
    skip_test "funding withdraw-history"
    skip_test "funding deposit-history"
    skip_test "funding otc-deposit-history"
    skip_test "funding otc-withdraw-history"
fi

fi  # RUN_PRIVATE

# =============================================================================
# Summary
# =============================================================================

echo ""
echo -e "${BOLD}══════════════════════════════════════════════${NC}"
echo -e "${BOLD}  E2E Test Results${NC}"
echo -e "${BOLD}══════════════════════════════════════════════${NC}"
echo -e "  Total:   ${BOLD}$TOTAL${NC}"
echo -e "  Passed:  ${GREEN}${BOLD}$PASS${NC}"
echo -e "  Failed:  ${RED}${BOLD}$FAIL${NC}"
echo -e "  Skipped: ${YELLOW}${BOLD}$SKIP${NC}"
echo -e "${BOLD}══════════════════════════════════════════════${NC}"

if [ $FAIL -gt 0 ]; then
    echo -e "${RED}${BOLD}SOME TESTS FAILED${NC}"
    exit 1
else
    echo -e "${GREEN}${BOLD}ALL TESTS PASSED ✓${NC}"
    exit 0
fi
