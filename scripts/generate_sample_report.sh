#!/usr/bin/env bash
set -euo pipefail

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

base_spec="$root_dir/examples/openapi/base.yaml"
current_spec="$root_dir/examples/openapi/current.yaml"
out_dir="$root_dir/docs/reports"
out_file="$out_dir/sample_report.html"

mkdir -p "$out_dir"

cargo run --quiet -- "$base_spec" "$current_spec" -o "$out_file"
echo "Wrote: $out_file"
