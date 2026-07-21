#!/bin/bash
# Build the presse device app for Flex (inside WSL).
set -e
source "$(dirname "$0")/env.sh"
cd "$APP_DIR"
cargo ledger build flex "$@"
