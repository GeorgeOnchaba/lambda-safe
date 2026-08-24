#!/usr/bin/env sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
binary=${1:-"$repo_dir/target/release/lambda-safe"}

if [ ! -x "$binary" ]; then
  echo "error: executable not found: $binary" >&2
  exit 70
fi

output_dir=$(mktemp -d)
trap 'rm -rf "$output_dir"' EXIT HUP INT TERM

"$binary" --solver finite check --input "$repo_dir/examples/valid/capacity.lambda" >/dev/null
"$binary" --solver finite compile \
  --input "$repo_dir/examples/valid/capacity.lambda" \
  --output "$output_dir/artifacts" >/dev/null
"$binary" --solver finite verify \
  --manifest "$output_dir/artifacts/manifest.json" \
  --certificate "$output_dir/artifacts/lambda-certificate.json" >/dev/null

test -s "$output_dir/artifacts/manifest.json"
test -s "$output_dir/artifacts/lambda-certificate.json"
echo "smoke test completed"
