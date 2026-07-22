#!/bin/bash
# Dry-run the piloted demo sequence end to end against fresh emulators, using
# the same commands the user will drive by hand: demo_steps.py for each beat
# (auto-routes through the cockpit on :5050 when it is up) and tap.sh to play
# the finger. Proves the Task 3 flow -- art BEFORE cut, no seal -- works from
# the relay tooling. Does NOT leave state behind: the caller resets with
# emu-up.sh afterward.
set -e
source "$(dirname "$0")/env.sh"
cd /mnt/c/Users/sylve/projects/presse
RAM=docs/art/ram-cover.bin
tap() { bash scripts/tap.sh "$1" "$2" >/dev/null; }

echo "=== fresh emulators ==="
pkill -f "speculos.*--api-port 500[12]" 2>/dev/null || true
sleep 1
rm -rf /tmp/presse-relay
nohup speculos --model flex --display headless --api-port 5001 --apdu-port 0 "$APP_ELF" >/tmp/pc-a.log 2>&1 &
nohup speculos --model flex --display headless --api-port 5002 --apdu-port 0 "$APP_ELF" >/tmp/pc-b.log 2>&1 &
for port in 5001 5002; do
  for _ in $(seq 1 40); do curl -s -o /dev/null "http://127.0.0.1:$port/events" && break; sleep 0.3; done
done
sleep 2

echo "=== art (upload sleeve to A, blank + pre-cut) ==="
python3 relay/demo_steps.py art "$RAM"

echo "=== cut (tap Cut the master on A) ==="
python3 relay/demo_steps.py cut "Random Access Memories" 5 &
PID=$!; sleep 2.5; tap 5001 "Cut the master"; wait $PID

echo "=== pair (tap Words match on both) ==="
python3 relay/demo_steps.py pair &
PID=$!; sleep 3.5; tap 5001 "Words match"; tap 5002 "Words match"; wait $PID

echo "=== press (tap Press this copy on A, Receive it on B) ==="
python3 relay/demo_steps.py press &
PID=$!; sleep 2.5; tap 5001 "Press this copy"; sleep 2.5; tap 5002 "Receive it"; wait $PID

echo "=== art again (now B holds a pressing -> uploads to B too) ==="
python3 relay/demo_steps.py art "$RAM"

echo "=== verify ==="
python3 relay/demo_steps.py verify

echo "=== collection on A (tap Back) ==="
python3 relay/demo_steps.py collection a &
PID=$!; sleep 1.5; tap 5001 "Back"; wait $PID

echo "=== PILOT SEQUENCE OK ==="
pkill -f "speculos.*--api-port 500[12]" 2>/dev/null || true
