#!/bin/bash
S=$(find ~/.cargo/registry/src -maxdepth 2 -name "ledger_device_sdk-1.36.0" -type d | head -1)
echo "SDK=$S"
ls "$S/src"
echo "===ECDH hits==="
grep -rn "ecdh" "$S/src" -l
echo "===random==="
grep -rn "pub fn" "$S/src/random.rs" 2>/dev/null | head
echo "===hash/hmac modules==="
ls "$S/src/hash" 2>/dev/null; ls "$S/src/hmac" 2>/dev/null
grep -rn "pub struct Sha2\|pub struct Sha256\|Sha2_256" "$S/src" | head -5
echo "===nvm==="
grep -n "pub struct\|pub fn" "$S/src/nvm.rs" 2>/dev/null | head -15
