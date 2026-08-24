#!/usr/bin/env sh
set -eu

repo_dir=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
parent_dir=$(dirname -- "$repo_dir")
version=$(tr -d '\r\n' < "$repo_dir/VERSION")
case "$version" in
  ''|*[!0-9A-Za-z.-]*)
    echo "error: invalid VERSION value" >&2
    exit 70
    ;;
esac
package_name="nigenje-lambda-safe-compiler-open-v$version"
archive="$parent_dir/$package_name.zip"

command -v zip >/dev/null 2>&1 || {
  echo "error: zip is required" >&2
  exit 70
}

stage_dir=$(mktemp -d)
trap 'rm -rf "$stage_dir"' EXIT HUP INT TERM
stage_project="$stage_dir/$package_name"
mkdir -p "$stage_project"
cp -R "$repo_dir/." "$stage_project/"
rm -rf \
  "$stage_project/.git" \
  "$stage_project/artifacts" \
  "$stage_project/target" \
  "$stage_project/fuzz/target"
find "$stage_project" -type f -name '*.zip' -delete

(cd "$stage_dir" && zip -q -r "$archive.tmp" "$package_name")
mv "$archive.tmp" "$archive"

sha256sum "$archive"
