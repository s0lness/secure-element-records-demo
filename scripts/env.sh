#!/bin/bash
# presse WSL build environment. Source me. (LF endings, run inside WSL Ubuntu.)
source ~/.cargo/env
export FLEX_SDK=~/ledger-secure-sdk
export PATH=~/venv-ledger/bin:$PATH   # python3 -> venv (ledgerblue, speculos)
export APP_DIR=/mnt/c/Users/sylve/projects/presse/device-app
export APP_ELF=$APP_DIR/target/flex/release/presse
