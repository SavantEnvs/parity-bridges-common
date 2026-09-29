#!/usr/bin/env bash
#
# mayhem/test.sh — behavioral oracle for StorageProofChecker (upstream has no test suite).
set -uo pipefail
[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH
: "${SRC:=/mayhem}"
cd "$SRC"

emit_ctrf() {
  local tool="$1" passed="$2" failed="$3" skipped="${4:-0}" pending="${5:-0}" other="${6:-0}"
  local tests=$(( passed + failed + skipped + pending + other ))
  cat > "${CTRF_REPORT:-$SRC/ctrf-report.json}" <<JSON
{
  "results": {
    "tool": { "name": "$tool" },
    "summary": {
      "tests": $tests,
      "passed": $passed,
      "failed": $failed,
      "pending": $pending,
      "skipped": $skipped,
      "other": $other
    }
  }
}
JSON
  printf 'CTRF {"results":{"tool":{"name":"%s"},"summary":{"tests":%d,"passed":%d,"failed":%d,"pending":%d,"skipped":%d,"other":%d}}}\n' \
    "$tool" "$tests" "$passed" "$failed" "$pending" "$skipped" "$other"
  [ "$failed" -eq 0 ]
}

ORACLE="/mayhem/storage-proof-oracle"
[ -x "$ORACLE" ] || { echo "ERROR: missing oracle binary $ORACLE (build.sh must produce it)" >&2; emit_ctrf "storage-proof-oracle" 0 1; exit 1; }

LOG="$(mktemp)"
if "$ORACLE" 2>&1 | tee "$LOG" | grep -q '^storage proof oracle: OK$'; then
  passed=1
  failed=0
else
  passed=0
  failed=1
fi
rm -f "$LOG"

emit_ctrf "storage-proof-oracle" "$passed" "$failed"
