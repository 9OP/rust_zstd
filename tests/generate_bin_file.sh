#!/bin/bash

data=(
    40 181 47 253 32 59 253 4 173 74 36 0 75 40 241 255 231 235 20 20 20
    70 20 235 0 255 255 255 26 0 0 0 16 0 0 235 235 235 235 171 235 235
    235 235 235 235 235 235 235 235 235 235 235 71 0 255 255 1 4 255 255 8
    255 255 255 251 40 181 47 255
)

# Specify the file name
file_name="binary_file.bin"

# Convert each number to binary and append to the file
for number in "${data[@]}"; do
    printf "\\x$(printf "%x" "$number")" >>"$file_name"
done

echo "Binary file '$file_name' created successfully."
