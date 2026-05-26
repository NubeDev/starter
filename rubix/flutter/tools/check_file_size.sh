#!/usr/bin/env bash
# Fails if any lib/**/*.dart or test/**/*.dart file exceeds 400 lines,
# excluding generated suffixes (.g.dart, .freezed.dart).
set -euo pipefail

MAX=400
EXIT=0

while IFS= read -r f; do
  lines=$(wc -l < "$f")
  if (( lines > MAX )); then
    echo "FAIL: $f has $lines lines (max $MAX)"
    EXIT=1
  fi
done < <(find lib test -name '*.dart' \
  ! -name '*.g.dart' \
  ! -name '*.freezed.dart' \
  2>/dev/null)

if (( EXIT == 0 )); then
  echo "OK: all source files ≤ $MAX lines"
fi
exit $EXIT
