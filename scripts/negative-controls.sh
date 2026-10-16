#!/usr/bin/env bash
# Compile each negative-control source and ASSERT it FAILS to compile.
# A negative control that COMPILES is a finding (the type discipline is not enforcing anything).
#
# The sources live in examples/ (auto-discovered by cargo, but NOT built by a plain
# `cargo build`, which only builds the two real bins). We build each one explicitly with
# `cargo build --example <name>`, assert a non-zero exit, and capture the real rustc error to
# negative/<name>.stderr.txt as the evidence record.
#
# A rejection counts only if rustc's output contains the expected marker. Any compile
# error is not enough: a typo or a missing import also exits non-zero. nc0_wrong_reason
# is a meta-control that fails for an unrelated reason and must be classed as such.
set -u
cd "$(dirname "$0")/.." || exit 2
mkdir -p negative
FAILED=0

# Returns 0 = rejected with the expected marker, 1 = compiled, 2 = rejected for another reason.
run_one() {
  local name="$1" expect_substr="$2"
  echo "== $name =="
  local out code
  out="$(cargo build --quiet --example "$name" 2>&1)"; code=$?
  if [ $code -eq 0 ]; then
    echo "  RESULT: COMPILED (exit 0)  <-- FINDING: type discipline did NOT reject this"
    return 1
  fi
  echo "$out" > "negative/${name}.stderr.txt"
  local kind; kind="$(echo "$out" | grep -oE 'error\[E[0-9]+\]|error: [a-z]' | sort -u | tr '\n' ' ')"
  echo "  RESULT: REJECTED at compile time (exit $code)"
  echo "  error kind(s): ${kind:-<see stderr.txt>}"
  echo "  full error -> negative/${name}.stderr.txt"
  if echo "$out" | grep -qF -- "$expect_substr"; then
    echo "  expected marker present: '$expect_substr'"
    return 0
  fi
  echo "  WRONG REASON: marker '$expect_substr' not found; this rejection does not count"
  return 2
}

cargo build --quiet --bins 2>/dev/null   # warm deps so probe errors are the source's own

run_one nc1_wrong_message   "no method named \`recv\`";            [ $? -eq 0 ] || FAILED=1
echo
run_one nc2_not_projectable "type mismatch in function arguments"; [ $? -eq 0 ] || FAILED=1
echo
# Meta-control: an unrelated compile error checked against nc1's marker must come back
# as WRONG REASON (2). Anything else means the marker check cannot tell reasons apart.
run_one nc0_wrong_reason    "no method named \`recv\`"; rc=$?
if [ $rc -eq 2 ]; then
  echo "  meta-control OK: a wrong-reason compile error is not counted as a rejection"
else
  echo "  META-CONTROL FAILURE: expected WRONG REASON (2), got $rc; the marker check is inert"
  FAILED=1
fi

echo
if [ $FAILED -eq 0 ]; then
  echo "ALL NEGATIVE CONTROLS REJECTED AS EXPECTED (both failed to compile at the session-type layer,"
  echo "with the expected marker; the wrong-reason meta-control was classed as wrong reason)."
  exit 0
else
  echo "NEGATIVE CONTROL FAILURE: a control compiled, was rejected for the wrong reason,"
  echo "or the wrong-reason meta-control was not detected."
  exit 1
fi
