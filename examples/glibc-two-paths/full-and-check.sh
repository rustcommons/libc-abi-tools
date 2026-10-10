#!/bin/bash
# Convenience entry point; CI runs the two stages in separate jobs.
set -euo pipefail
example_dir=$(cd "$(dirname "$0")" && pwd)
work=$(realpath "${1:?Usage: full-and-check.sh EXISTING_EXPERIMENT_DIRECTORY}")
bash "$example_dir/prepare-and-check.sh" "$work"
bash "$example_dir/application-and-check.sh" "$work"
