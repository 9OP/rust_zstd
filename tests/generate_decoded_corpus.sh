#!/bin/bash

# Set the source and destination directories
source_dir="./tests/corpus"
destination_dir="./tests/decoded_corpus"

# Create the destination directory if it doesn't exist
mkdir -p "$destination_dir"

# Loop through all zstd compressed files in the source directory
for compressed_file in "$source_dir"/*.zst; do
    # Get the base name of the file without the extension
    base_name=$(basename "$compressed_file" .zst)

    # Decompress the file to the destination directory
    yes | zstd -d -o "$destination_dir/$base_name.bin" "$compressed_file"

    # Optional: Remove the original compressed file
    # Uncomment the line below if you want to delete the compressed file after decompression
    # rm "$compressed_file"
done

# echo "completed"
