#!/usr/bin/env sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary=${1:-"$repo_dir/target/release/lambda-safe"}
solver=${2:-finite}

if [ ! -x "$binary" ]; then
  echo "error: executable not found: $binary" >&2
  exit 70
fi

output_dir=$(mktemp -d)
trap 'rm -rf "$output_dir"' EXIT HUP INT TERM

"$binary" check --input "$repo_dir/examples/valid/capacity.lambda" >"$output_dir/check.json"
grep -q '"valid": true' "$output_dir/check.json"
"$binary" --solver "$solver" compile \
  --input "$repo_dir/examples/valid/capacity.lambda" \
  --output "$output_dir/artifacts" >"$output_dir/compile.json"
grep -q '"status": "STATICALLY_CERTIFIED"' "$output_dir/compile.json"
"$binary" verify \
  --manifest "$output_dir/artifacts/manifest.json" \
  --certificate "$output_dir/artifacts/lambda-certificate.json" >"$output_dir/verify.json"
grep -q '"valid": true' "$output_dir/verify.json"

if "$binary" --solver "$solver" compile \
  --input "$repo_dir/examples/invalid/counterexample.lambda" \
  --output "$output_dir/invalid-artifacts" >"$output_dir/invalid.json"; then
  echo "error: invalid contract was unexpectedly certified with $solver" >&2
  exit 1
else
  status=$?
fi
if [ "$status" -ne 3 ]; then
  cat "$output_dir/invalid.json" >&2
  echo "error: expected invariant counterexample exit code 3, received $status" >&2
  exit "$status"
fi
grep -q '"status": "INVARIANT_VIOLATED"' "$output_dir/invalid.json"
test ! -e "$output_dir/invalid-artifacts"

test -s "$output_dir/artifacts/manifest.json"
test -s "$output_dir/artifacts/lambda-certificate.json"
echo "smoke test completed with $solver"
