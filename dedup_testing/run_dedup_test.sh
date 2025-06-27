#!/bin/bash

ROOT="test_profiles"
PROFILES=("low" "medium" "high")
OUTFILE="dedup_results.txt"

# Start a fresh output file
echo "=== Deduplication Test Results ===" > "$OUTFILE"
echo "Timestamp: $(date)" >> "$OUTFILE"
echo "-----------------------------------" >> "$OUTFILE"

for profile in "${PROFILES[@]}"; do
  echo "==== Testing deduplication for profile: $profile ===="
  DIR="$ROOT/$profile"
  FILES=($DIR/*.*)

  # Clean existing store
  rm -rf ~/.nebula/node*/content || true

  for file in "${FILES[@]}"; do
    [[ $file == *.json ]] && continue
    echo "Putting $file..."
    cargo run put "$file" > /dev/null
  done

  echo "Fetching final stats..."
  STATS=$(cargo run stats)

  # Extract relevant stats
  CHUNKS=$(echo "$STATS" | grep "Total chunks" | awk '{print $3}')
  CHUNK_SIZE=$(echo "$STATS" | grep "Total chunk size" | awk '{print $4}')
  FILE_SIZE=$(echo "$STATS" | grep "Total file size" | awk '{print $4}')
  SAVED=$(echo "scale=2; (1 - $CHUNK_SIZE / $FILE_SIZE) * 100" | bc)

  # Write to output file
  {
    echo "Profile: $profile"
    echo "Files stored: ${#FILES[@]}"
    echo "Total file size: $FILE_SIZE bytes"
    echo "Unique chunk storage: $CHUNK_SIZE bytes"
    echo "Total chunks: $CHUNKS"
    echo "Deduplication savings: $SAVED %"
    echo "-----------------------------------"
  } >> "$OUTFILE"

  echo "✅ Profile '$profile' results saved."
done

echo -e "\nAll results saved to: $OUTFILE"
