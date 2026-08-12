#!/usr/bin/env bash
#
# mayhem/build.sh — build the storage-proof cargo-fuzz target as a sanitized libFuzzer
# binary, plus a deterministic oracle binary for mayhem/test.sh.
set -euo pipefail

[ -n "${SOURCE_DATE_EPOCH:-}" ] || unset SOURCE_DATE_EPOCH

: "${SRC:=/mayhem}"
# polkadot-sdk compiles are memory-heavy; default to single-job unless overridden.
: "${MAYHEM_JOBS:=1}"
export CARGO_BUILD_JOBS="$MAYHEM_JOBS"
export CARGO_INCREMENTAL=0

cd "$SRC"

: "${RUST_DEBUG_FLAGS:=-C debuginfo=2 -C force-frame-pointers=yes}"
DWARF_FLAGS="-Zdwarf-version=3"

FUZZ_RUSTFLAGS="${RUSTFLAGS:-} --cfg fuzzing -Zsanitizer=address ${RUST_DEBUG_FLAGS} ${DWARF_FLAGS}"
echo "SANITIZER_FLAGS (base, informational) = ${SANITIZER_FLAGS:-<unset>}"

FUZZ_DIR="mayhem/fuzz"
TRIPLE="x86_64-unknown-linux-gnu"

ASAN_A="$(rustc --print sysroot)/lib/rustlib/${TRIPLE}/lib/librustc-nightly_rt.asan.a"
if [ -f "$ASAN_A" ]; then
  echo "stripping debug info from prebuilt ASan runtime: $ASAN_A"
  objcopy --strip-debug "$ASAN_A" 2>/dev/null || objcopy --remove-section '.debug_*' "$ASAN_A" 2>/dev/null || true
fi

FUZZ_TARGETS=()
for f in "$FUZZ_DIR"/fuzz_targets/*.rs; do
  FUZZ_TARGETS+=("$(basename "${f%.*}")")
done
[ "${#FUZZ_TARGETS[@]}" -gt 0 ] || { echo "ERROR: no fuzz targets under $FUZZ_DIR/fuzz_targets/" >&2; exit 1; }

export CFLAGS="${CFLAGS:-} -gdwarf-3"
export CXXFLAGS="${CXXFLAGS:-} -gdwarf-3"

echo "=== cargo fetch fuzz crate (warm $CARGO_HOME git/registry cache) ==="
cargo fetch --manifest-path "$FUZZ_DIR/Cargo.toml"

echo "=== cargo fuzz build (ASan, DWARF 3) ==="
echo "RUSTFLAGS=$FUZZ_RUSTFLAGS"
echo "targets: ${FUZZ_TARGETS[*]}"

for t in "${FUZZ_TARGETS[@]}"; do
  echo "--- building fuzz target: $t ---"
  RUSTFLAGS="$FUZZ_RUSTFLAGS" cargo fuzz build --fuzz-dir "$FUZZ_DIR" -O --debug-assertions "$t"
  bin="$SRC/$FUZZ_DIR/target/$TRIPLE/release/$t"
  [ -x "$bin" ] || { echo "ERROR: expected fuzz binary not found at $bin" >&2; exit 1; }
done

# Mayhemfile target name is storage-proof-fuzzer (preserved from the old integration).
cp "$SRC/$FUZZ_DIR/target/$TRIPLE/release/main" /mayhem/storage-proof-fuzzer
echo "built /mayhem/storage-proof-fuzzer"

echo "=== cargo build storage-proof oracle (clean flags, for test.sh) ==="
ORACLE_RUSTFLAGS="--cap-lints=warn"
RUSTFLAGS="$ORACLE_RUSTFLAGS" cargo build --manifest-path "$FUZZ_DIR/Cargo.toml" --release --target "$TRIPLE" --bin storage-proof-oracle
oracle_bin="$SRC/$FUZZ_DIR/target/$TRIPLE/release/storage-proof-oracle"
[ -x "$oracle_bin" ] || { echo "ERROR: oracle binary not found at $oracle_bin" >&2; exit 1; }
cp "$oracle_bin" /mayhem/storage-proof-oracle
echo "built /mayhem/storage-proof-oracle"

echo "build.sh complete"
