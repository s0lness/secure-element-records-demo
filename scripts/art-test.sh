#!/bin/bash
# Runtime-bitmap go/no-go: fire ART_TEST on the running emulator A, snapshot.
source "$(dirname "$0")/env.sh"
curl -s --max-time 10 http://127.0.0.1:5001/apdu -d '{"data":"b561000000"}' -o /dev/null &
sleep 2.5
curl -s http://127.0.0.1:5001/screenshot -o /mnt/c/Users/sylve/projects/presse/docs/screens/19-art-test.png
bash "$(dirname "$0")/tap.sh" 5001 "Back" >/dev/null 2>&1
echo SNAPPED
