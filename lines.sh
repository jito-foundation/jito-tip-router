#!/bin/bash

# Array of directories to scan
directories=(
    "./program"
    "./core"
    "./integration_tests"
    "./cli"
)

total=0

# Function to count lines in a directory
count_lines() {
    local dir=$1
    if [ ! -d "$dir" ]; then
        echo "Warning: Directory $dir does not exist, skipping..."
        echo 0
        return
    fi
    
    local count=$(find "$dir" -name "*.rs" -type f -exec wc -l {} + | awk '{total += $1} END {print total}')
    
    # If no Rust files found, count will be empty
    if [ -z "$count" ]; then
        count=0
    fi
    
    echo "$dir: $count lines"
    echo $count
}

# Process each directory
for dir in "${directories[@]}"; do
    count=$(count_lines "$dir")
    # Get the last line of output (the count)
    dir_count=$(echo "$count" | tail -n 1)
    total=$((total + dir_count))
done

echo "----------------------------------------"
echo "Total lines of Rust code: $total"