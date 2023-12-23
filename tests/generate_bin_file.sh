#!/bin/bash

data=(
    40 181 47 253 48 40 181 0 0 42 0 165 47 16 16 246 23 64 0 2 0 0 0 0
    90 28 0 255 247 255 255
)

# Specify the file name
file_name="binary_file.bin"

# Convert each number to binary and append to the file
for number in "${data[@]}"; do
    printf "\\x$(printf "%x" "$number")" >>"$file_name"
done

echo "Binary file '$file_name' created successfully."
