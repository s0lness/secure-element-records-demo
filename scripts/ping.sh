#!/bin/bash
# Boot the built app in Speculos and send GET_VERSION. Exit 0 iff it answers 9000.
source "$(dirname "$0")/env.sh"
PORT="${1:-5000}"
rm -f /tmp/ping-resp.json
speculos --model flex --display headless --api-port "$PORT" "$APP_ELF" >/tmp/speculos-ping.log 2>&1 &
SPID=$!
trap 'kill $SPID 2>/dev/null' EXIT
for i in $(seq 1 60); do
  sleep 0.5
  if curl -s --max-time 3 "http://127.0.0.1:$PORT/apdu" -d '{"data":"e003000000"}' -o /tmp/ping-resp.json 2>/dev/null; then
    if [ -s /tmp/ping-resp.json ]; then break; fi
  fi
done
echo "--- response:"; cat /tmp/ping-resp.json 2>/dev/null; echo
if grep -q 9000 /tmp/ping-resp.json 2>/dev/null; then
  echo PING_OK
else
  echo PING_FAIL
  echo "--- speculos log:"; tail -30 /tmp/speculos-ping.log
  exit 1
fi
