#!/bin/bash
# Serve the two-device demo cockpit on http://localhost:5050 (emu-up first).
source "$(dirname "$0")/env.sh"
cd /mnt/c/Users/sylve/projects/presse
exec python3 relay/cockpit.py
