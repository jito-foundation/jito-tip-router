#!/bin/bash

# File paths
FULL_OUTPUT="tests.output"
PROGRAM_ERRORS="program_errors.json"
TEST_ERRORS="test_errors.output"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Help text in heredoc format for better readability
usage() {
    cat << EOF
To use this script, pipe the output of the test command to it.

Examples:
    cargo nextest run --all-features -E 'not test(bpf)' |& ./pipe_test_output.sh
    cargo build-sbf --sbf-out-dir integration_tests/tests/fixtures && SBF_OUT_DIR=integration_tests/tests/fixtures cargo nextest run jito-tip-router-integration-tests::tests tip_router::meta_tests::tests::test_all_test_ncn_functions |& ./pipe_test_output.sh
EOF
}

# Check if there's input on stdin
if [ -t 0 ]; then
    usage
    exit 1
fi

rm -f "$FULL_OUTPUT" "$PROGRAM_ERRORS" "$TEST_ERRORS"

# Process input from stdin and save it to temp file
cat > "$FULL_OUTPUT"

# Process the output file to extract error logs
process_program_logs() {
    # Get just the first occurrence of logs array
    sed -n '0,/logs: \[/p;/\], units_consumed/{p;q;}' "$FULL_OUTPUT" | \
        # Remove the "logs: [" prefix and trailing content
        sed '1s/^.*logs: \[/[/' | \
        sed '$s/\], units_consumed.*$/]/' > "$PROGRAM_ERRORS"
}

# Process the output file to extract test output
process_test_logs() {
    sed -n '/--- STDOUT/,/--- STDERR/p' "$FULL_OUTPUT" | \
        # Remove the STDERR line
        sed '$d' > "$TEST_ERRORS"
}

# Process the output
process_program_logs
process_test_logs

echo " -------- FORMATTING DONE --------"
echo "Test output saved to $FULL_OUTPUT"
echo "Program error logs extracted to $PROGRAM_ERRORS"
echo "Test error logs extracted to $TEST_ERRORS"
echo " "
echo " ---------- PROGRAM ERRORS ----------"
echo " "
cat "$PROGRAM_ERRORS"
echo " "
echo " ---------- TEST ERRORS ----------"
cat "$TEST_ERRORS"
echo " "
echo " ---------- ORIGINAL OUTPUT ----------"
echo " "
# Color the output
cat "$FULL_OUTPUT" | sed \
    -e "s/PASS/$(echo -e "${GREEN}PASS${NC}")/g" \
    -e "s/FAIL/$(echo -e "${RED}FAIL${NC}")/g" \
    -e "s/Running/$(echo -e "${BLUE}Running${NC}")/g" \
    -e "s/Starting/$(echo -e "${BLUE}Starting${NC}")/g" \
    -e "s/Summary/$(echo -e "${BLUE}Summary${NC}")/g"
echo " "