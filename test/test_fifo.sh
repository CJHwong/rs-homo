#!/bin/bash

# Test script for FIFO (named pipe) functionality in homo app
# This script uses named pipes to test various streaming scenarios with more control

set -e

# Ignore SIGPIPE to prevent script crash when window closes early
trap '' PIPE

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

echo "🔧 Starting FIFO test for homo app..."
echo "=================================================="
echo "This script will test various FIFO scenarios:"
echo "1. Example.md with normal delays via FIFO"
echo "2. Example.md with fast delays via FIFO"
echo "3. Example.md with controlled pauses via FIFO"
echo "4. Interactive FIFO streaming"
echo "5. Burst then pause pattern via FIFO"
echo "6. Line-by-line manual control via FIFO"
echo ""
echo "💡 Instructions:"
echo "• Each test creates a named pipe (FIFO)"
echo "• Watch the app window for real-time updates"
echo "• Press Ctrl+C to stop any test early"
echo "• Tests will automatically clean up FIFOs"
echo ""

# Cleanup function to remove FIFOs and kill background processes
cleanup() {
    echo ""
    echo "🧹 Cleaning up..."
    
    local homo_exit_code=0
    if [ -n "$OUTPUT_PROCESS_PID" ]; then
        echo "Checking homo process (PID: $OUTPUT_PROCESS_PID)"
        # Try to get exit code if process is still running
        if kill -0 "$OUTPUT_PROCESS_PID" 2>/dev/null; then
            kill "$OUTPUT_PROCESS_PID" 2>/dev/null || true
        fi
        wait "$OUTPUT_PROCESS_PID" 2>/dev/null || homo_exit_code=$?
        
        if [ $homo_exit_code -eq 0 ]; then
            echo "✅ Final homo process exit code: $homo_exit_code"
        else
            echo "❌ Final homo process exit code: $homo_exit_code"
        fi
    fi
    
    if [ -n "$OUTPUT_PIPE" ] && [ -p "$OUTPUT_PIPE" ]; then
        echo "Removing FIFO: $OUTPUT_PIPE"
        rm -f "$OUTPUT_PIPE"
    fi
    
    # Close file descriptor if open
    exec 3>&- 2>/dev/null || true
    
    echo "✅ Cleanup complete"
}

# Set trap to cleanup on exit or interrupt
trap cleanup EXIT INT TERM

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

# Function to setup FIFO and start homo
setup_fifo() {
    # Create a unique FIFO
    OUTPUT_PIPE="$(mktemp -u).fifo"
    echo "📡 Creating FIFO: $OUTPUT_PIPE"
    mkfifo "$OUTPUT_PIPE"
    
    # Start homo in background, reading from the pipe
    echo "🚀 Starting homo process..."
    cargo run < "$OUTPUT_PIPE" &
    OUTPUT_PROCESS_PID=$!
    
    echo "🔗 Homo PID: $OUTPUT_PROCESS_PID"
    
    # Give homo a moment to start
    sleep 1
    
    # Open the pipe for writing with file descriptor 3
    exec 3>"$OUTPUT_PIPE"
    echo "✅ FIFO setup complete"
}

# Function to cleanup between tests
cleanup_test() {
    local test_name="$1"
    echo ""
    echo "🛑 Stopping current test..."
    
    # Close the file descriptor
    exec 3>&- 2>/dev/null || true
    
    # Kill the homo process and capture exit code
    local homo_exit_code=0
    if [ -n "$OUTPUT_PROCESS_PID" ]; then
        # Wait for the process to finish and capture exit code
        wait "$OUTPUT_PROCESS_PID" 2>/dev/null || homo_exit_code=$?
        
        if [ $homo_exit_code -eq 0 ]; then
            echo "✅ $test_name: SUCCESS (exit code: $homo_exit_code)"
            PASSED_TESTS+=("$test_name")
        else
            echo "❌ $test_name: FAILED (exit code: $homo_exit_code)"
            FAILED_TESTS+=("$test_name (exit code: $homo_exit_code)")
        fi
    fi
    
    # Remove the FIFO
    if [ -n "$OUTPUT_PIPE" ] && [ -p "$OUTPUT_PIPE" ]; then
        rm -f "$OUTPUT_PIPE"
    fi
    
    echo "✅ Test cleanup complete"
    echo "📱 Please close the homo app window and press Enter to continue..."
    read -r
    
    return $homo_exit_code
}

# Test 1: Normal delay streaming via FIFO
run_test "FIFO Normal Streaming" "Streaming example.md via FIFO with normal delays"

setup_fifo

echo "📤 Writing to FIFO with $DELAY second delays..."
while IFS= read -r line || [ -n "$line" ]; do
    if ! echo "$line" >&3 2>/dev/null; then
        echo "🔌 FIFO closed (window terminated early), stopping..."
        break
    fi
    sleep $DELAY
done < $EXAMPLE_FILE

cleanup_test "FIFO Normal Streaming"

# Test 2: Fast streaming via FIFO
run_test "FIFO Fast Streaming" "Streaming example.md via FIFO with quick delays"

setup_fifo

echo "📤 Writing to FIFO with $QUICK_DELAY second delays..."
while IFS= read -r line || [ -n "$line" ]; do
    if ! echo "$line" >&3 2>/dev/null; then
        echo "🔌 FIFO closed (window terminated early), stopping..."
        break
    fi
    sleep $QUICK_DELAY
done < $EXAMPLE_FILE

cleanup_test "FIFO Fast Streaming"

# Test 3: Controlled pauses
run_test "FIFO Controlled Pauses" "Streaming with strategic pauses at sections"

setup_fifo

echo "📤 Writing to FIFO with strategic pauses..."
line_count=0
while IFS= read -r line || [ -n "$line" ]; do
    if ! echo "$line" >&3 2>/dev/null; then
        echo "🔌 FIFO closed (window terminated early), stopping..."
        break
    fi
    ((line_count++))
    
    # Pause longer at headers
    if [[ "$line" =~ ^#+[[:space:]] ]]; then
        echo "  🔶 Header detected, pausing..."
        sleep $LONG_DELAY
    # Pause at code block boundaries
    elif [[ "$line" =~ ^\`\`\` ]]; then
        echo "  💻 Code block boundary, pausing..."
        sleep $DELAY
    # Regular delay for other lines
    else
        sleep $QUICK_DELAY
    fi
    
    # Every 50 lines, take a longer pause
    if [ $((line_count % 50)) -eq 0 ]; then
        echo "  ⏸️  Checkpoint at line $line_count, longer pause..."
        sleep $LONG_DELAY
    fi
done < $EXAMPLE_FILE

cleanup_test "FIFO Controlled Pauses"

# Test 4: Interactive streaming
run_test "FIFO Interactive" "Manual control over streaming"

setup_fifo

echo "📤 Interactive FIFO streaming mode"
echo "🎮 Press Enter to send each line, 'q' to quit early"

line_num=0
# Use file descriptor 4 to read from the file, keeping stdin free for user input
exec 4< "$EXAMPLE_FILE"
while IFS= read -r line <&4 || [ -n "$line" ]; do
    ((line_num++))
    echo ""
    echo "📝 Line $line_num: $line"
    echo "👆 Press Enter to send this line (or 'q' to quit):"
    
    read -r user_input
    if [ "$user_input" = "q" ]; then
        echo "🛑 User requested quit"
        break
    fi
    
    if ! echo "$line" >&3 2>/dev/null; then
        echo "🔌 FIFO closed (window terminated early), stopping..."
        break
    fi
    echo "✅ Line sent to homo"
done
# Close the file descriptor
exec 4<&-

cleanup_test "FIFO Interactive"

# Test 5: Burst then pause pattern
run_test "FIFO Burst Pattern" "Send lines in bursts with pauses between"

setup_fifo

echo "📤 Burst pattern: 10 lines quickly, then pause"
line_count=0
burst_size=10

while IFS= read -r line || [ -n "$line" ]; do
    if ! echo "$line" >&3 2>/dev/null; then
        echo "🔌 FIFO closed (window terminated early), stopping..."
        break
    fi
    ((line_count++))
    
    if [ $((line_count % burst_size)) -eq 0 ]; then
        echo "  💥 Burst of $burst_size lines sent, pausing..."
        sleep $LONG_DELAY
    else
        sleep 0.05  # Very quick within burst
    fi
done < $EXAMPLE_FILE

cleanup_test "FIFO Burst Pattern"

# Test 6: Line-by-line with progress
run_test "FIFO Progress Streaming" "Streaming with progress indicators"

setup_fifo

echo "📤 Streaming with progress indicators..."
total_lines=$(wc -l < $EXAMPLE_FILE)
line_count=0

while IFS= read -r line || [ -n "$line" ]; do
    ((line_count++))
    if ! echo "$line" >&3 2>/dev/null; then
        echo "🔌 FIFO closed (window terminated early), stopping..."
        break
    fi
    
    # Show progress every 25 lines
    if [ $((line_count % 25)) -eq 0 ]; then
        progress=$((line_count * 100 / total_lines))
        echo "  📊 Progress: $line_count/$total_lines ($progress%)"
    fi
    
    sleep $DELAY
done < $EXAMPLE_FILE

echo "✅ Streaming complete: $line_count lines sent"

cleanup_test "FIFO Progress Streaming"

echo ""
echo "🎉 All FIFO tests completed!"
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
echo "🔍 FIFO Testing Benefits:"
echo "• More precise control over timing"
echo "• Ability to pause and resume streaming"
echo "• Interactive testing capabilities"
echo "• Better simulation of real-world pipe scenarios"
echo "• Process isolation and cleanup"
echo ""

if [ ${#FAILED_TESTS[@]} -eq 0 ]; then
    echo "🎊 All tests passed! FIFO functionality is working correctly."
    echo "📊 FIFO testing complete!"
    exit 0
else
    echo "⚠️  Some tests failed. Please check the output above for details."
    echo "📊 FIFO testing complete with failures!"
    exit 1
fi
