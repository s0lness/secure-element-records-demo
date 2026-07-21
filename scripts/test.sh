#!/bin/bash
# Run the pytest suite (inside WSL).
source "$(dirname "$0")/env.sh"
cd /mnt/c/Users/sylve/projects/presse/tests
exec pytest -x -q "$@"
