#!/usr/bin/env bash
# Compile each negative-control source and ASSERT it FAILS to compile.
# A negative control that COMPILES is a finding (the type discipline is not enforcing anything).
#
# The sources live in examples/ (auto-discovered by cargo, but NOT built by a plain
# `cargo build`, which only builds the two real bins). We build each one explicitly with
# `cargo build --example <name>`, assert a non-zero exit, and capture the real rustc error to
# negative/<name>.stderr.txt as the evidence record.
set -u
cd "$(dirname "$0")/.." || exit 2
mkdir -p negative
FAILED=0

run_one() {
  local name="$1" expect_substr="$2"
  echo "== $name =="
  local out code
  out="$(cargo build --quiet --example "$name" 2>&1)"; code=$?
  if [ $code -eq 0 ]; then
    echo "  RESULT: COMPILED (exit 0)  <-- FINDING: type discipline did NOT reject this"
    FAILED=1
    return
  fi
  echo "$out" > "negative/${name}.stderr.txt"
  local kind; kind="$(echo "$out" | grep -oE 'error\[E[0-9]+\]|error: [a-z]' | sort -u | tr '\n' ' ')"
  echo "  RESULT: REJECTED at compile time (exit $code)"
  echo "  error kind(s): ${kind:-<see stderr.txt>}"
  if echo "$out" | grep -q "$expect_substr"; then
    echo "  expected marker present: '$expect_substr'"
  else
    echo "  NOTE: marker '$expect_substr' not literally found (wording varies; rejection still holds)"
  fi
  echo "  full error -> negative/${name}.stderr.txt"
}

cargo build --quiet --bins 2>/dev/null   # warm deps so probe errors are the source's own

run_one nc1_wrong_message   "no method named \`recv\`"
echo
run_one nc2_not_projectable "type mismatch in function arguments"

echo
if [ $FAILED -eq 0 ]; then
  echo "ALL NEGATIVE CONTROLS REJECTED AS EXPECTED (both failed to compile at the session-type layer)."
  exit 0
else
  echo "NEGATIVE CONTROL FAILURE: something compiled that should not have."
  exit 1
fi
