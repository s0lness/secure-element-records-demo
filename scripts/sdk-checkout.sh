#!/bin/bash
# Switch the C secure-sdk checkout to a given API_LEVEL branch (default 25), then rebuild.
set -e
BRANCH="API_LEVEL_${1:-25}"
cd ~/ledger-secure-sdk
git fetch --depth 1 origin "$BRANCH"
git checkout -q FETCH_HEAD
echo "secure-sdk now at: $(git log --oneline -1)"
