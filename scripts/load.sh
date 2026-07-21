#!/bin/bash
# Build and sideload the presse app onto a connected Flex (inside WSL).
# Prereqs: the Flex is attached to WSL via usbipd (see scripts/attach-usb.ps1),
# unlocked, on the dashboard, and the custom CA is installed (scripts/install-ca.sh).
set -e
source "$(dirname "$0")/env.sh"
cd "$APP_DIR"
cargo ledger build flex -l
