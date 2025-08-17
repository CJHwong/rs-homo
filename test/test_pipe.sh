#!/bin/bash

# Test script for pipe functionality in homo app
# This script tests various pipe scenarios to ensure the app handles streaming input correctly

set -e

DELAY=0.05
QUICK_DELAY=0.001
LONG_DELAY=0.5
SCRIPT_DIR=$(pwd)/$(dirname "$0")
EXAMPLE_FILE="$SCRIPT_DIR/example.md"

export RUST_LOG=debug 
export RUST_BACKTRACE=1

# Track test results
FAILED_TESTS=()
PASSED_TESTS=()

echo "🔧 Starting pipe test for homo app..."
echo "=================================================="
echo "This script will test various pipe scenarios:"
echo "1. Example.md with normal delays"
echo "2. Example.md with fast delays"
echo "3. Example.md with slow delays"
echo "4. Example.md in batches (every 5 lines)"
echo "5. Example.md with no delays"
echo "6. Example.md with random delays"
echo ""
echo "💡 Instructions:"
echo "• Each test will pipe content to homo"
echo "• Watch the app window for real-time updates"
echo "• Press Ctrl+C to stop any test early"
echo "• Close the app window between tests"
echo ""

# Function to run a test with a description
run_test() {
    local test_name="$1"
    local test_description="$2"
    
    echo ""
    echo "🧪 Test: $test_name"
    echo "📝 Description: $test_description"
    echo "🚀 Starting in 2 seconds... (Ctrl+C to skip)"
    sleep 2
    echo "▶️  Running test..."
}

# Function to wait for user input between tests
wait_for_next() {
    local exit_code=$?
    echo ""
    if [ $exit_code -eq 0 ]; then
        echo "✅ Test completed successfully (exit code: $exit_code)"
    else
        echo "❌ Test failed with exit code: $exit_code"
    fi
    echo "📱 Close the app window and press Enter for next test..."
    read -r
}

# Function to check exit code and report results
check_exit_code() {
    local exit_code=$?
    local test_name="$1"
    
    if [ $exit_code -eq 0 ]; then
        echo "✅ $test_name: SUCCESS (exit code: $exit_code)"
        PASSED_TESTS+=("$test_name")
        return 0
    else
        echo "❌ $test_name: FAILED (exit code: $exit_code)"
        FAILED_TESTS+=("$test_name (exit code: $exit_code)")
        return $exit_code
    fi
}

# Test 1: Simple markdown streaming
run_test "Example.md Streaming" "Streaming example.md line by line with delays"

{
    while IFS= read -r line || [ -n "$line" ]; do
        echo "$line"
        sleep $DELAY
    done < $EXAMPLE_FILE
} | cargo run --;

check_exit_code "Example.md Streaming"
wait_for_next

# Test 2: Fast streaming
run_test "Example.md Fast" "Streaming example.md with quick delays"

{
    while IFS= read -r line || [ -n "$line" ]; do
        echo "$line"
        sleep $QUICK_DELAY
    done < $EXAMPLE_FILE
} | cargo run --;

check_exit_code "Example.md Fast"
wait_for_next

# Test 3: Slow streaming  
run_test "Example.md Slow" "Streaming example.md with longer delays"

{
    while IFS= read -r line || [ -n "$line" ]; do
        echo "$line"
        sleep $LONG_DELAY
    done < $EXAMPLE_FILE
} | cargo run --;

check_exit_code "Example.md Slow"
wait_for_next

# Test 4: Batch streaming
run_test "Example.md Batched" "Streaming example.md in small batches"

{
    line_count=0
    while IFS= read -r line || [ -n "$line" ]; do
        echo "$line"
        ((line_count++))
        # Send a batch every 5 lines
        if [ $((line_count % 5)) -eq 0 ]; then
            sleep $DELAY
        fi
    done < $EXAMPLE_FILE
} | cargo run --;

check_exit_code "Example.md Batched"
wait_for_next

# Test 5: No delay streaming
run_test "Example.md No Delay" "Streaming example.md as fast as possible"

{
    while IFS= read -r line || [ -n "$line" ]; do
        echo "$line"
    done < $EXAMPLE_FILE
} | cargo run --;

check_exit_code "Example.md No Delay"
wait_for_next

# Test 6: Random delay streaming
run_test "Example.md Random Delays" "Streaming example.md with random delays to simulate real-world usage"

{
    while IFS= read -r line || [ -n "$line" ]; do
        echo "$line"
        # Random delay between 0.1 and 1.0 seconds
        random_delay=$(echo "scale=1; $RANDOM/32767 * 0.9 + 0.1" | bc -l 2>/dev/null || echo "0.5")
        sleep "$random_delay"
    done < $EXAMPLE_FILE
} | cargo run --;

check_exit_code "Example.md Random Delays"

echo ""
echo "🎉 All pipe tests completed!"
echo "=================================================="

# Print detailed results
echo "📊 Test Results Summary:"
echo "✅ Passed: ${#PASSED_TESTS[@]}"
echo "❌ Failed: ${#FAILED_TESTS[@]}"

if [ ${#PASSED_TESTS[@]} -gt 0 ]; then
    echo ""
    echo "✅ Passed Tests:"
    for test in "${PASSED_TESTS[@]}"; do
        echo "  • $test"
    done
fi

if [ ${#FAILED_TESTS[@]} -gt 0 ]; then
    echo ""
    echo "❌ Failed Tests:"
    for test in "${FAILED_TESTS[@]}"; do
        echo "  • $test"
    done
fi

echo ""
echo "🔍 Did you observe:"
echo "• Real-time content updates in the app?"
echo "• Proper markdown rendering?"
echo "• Syntax highlighting in code blocks?"
echo "• Smooth streaming without blocking?"
echo "• Correct handling of content boundaries?"
echo ""

if [ ${#FAILED_TESTS[@]} -eq 0 ]; then
    echo "🎊 All tests passed! Pipe functionality is working correctly."
    echo "📊 Pipe testing complete!"
    exit 0
else
    echo "⚠️  Some tests failed. Please check the output above for details."
    echo "📊 Pipe testing complete with failures!"
    exit 1
fi
