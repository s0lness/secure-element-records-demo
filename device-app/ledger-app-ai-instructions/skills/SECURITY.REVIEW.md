---
description: "Full-depth security audit methodology for any Ledger embedded application (C or Rust) — APDU reachability, trust model, memory safety, cryptographic misuse, clear-signing bypass. Use when the user asks to audit, pentest, find vulnerabilities, security review an embedded Ledger app, or mentions APDU fuzzing, exploit PoC, clear-signing bypass, or embedded firmware security."
applyTo: "**/*"
---

# Ledger Embedded Application Security Review — Generic Methodology

Multi-pass security audit for **any** Ledger hardware wallet embedded application (C or Rust). Covers the Ledger-specific trust model, APDU attack surface, clear-signing semantics, and the unique constraints of limited RAM devices. Applies to UTXO apps (Bitcoin, ZCash, ...), account-based apps (Ethereum, Solana, ...), and other C/Rust apps.

---

## Phase 0: Reconnaissance & Architecture Mapping

### 0.1 Identify the Application Boundary

```
Task Progress:
- [ ] 0. Recon & architecture mapping
- [ ] 1. Trust model & threat model
- [ ] 2. APDU dispatcher & entry point enumeration
- [ ] 3. Memory & state analysis
- [ ] 4. Vulnerability hunting (multi-category)
- [ ] 5. APDU reachability verification
- [ ] 6. False-positive elimination pass
- [ ] 7. Exploitability classification
- [ ] 8. PoC / sanitizer confirmation
- [ ] 9. Coherence check & quality gate
- [ ] 10. Final report generation
```

Before auditing, map the architecture:

1. **Read `ledger_app.toml`** — devices, test directory, app flags, SDK (C or Rust)
2. **Read `Makefile` / `Cargo.toml` / `features.mk`** — compiled features, `#define` / `--features` flags, coin variants
3. **Identify the language and SDK**:
   - **C apps**: ISO C99, Clang/LLVM, `BOLOS_SDK`, Makefile-based
   - **Rust apps**: `#![no_std]`, `ledger_device_sdk` crate, custom test harness
4. **Identify APDU dispatcher** — the INS routing mechanism:
   - **C**: `apdu_dispatcher()` switch on `cmd->ins`, or `handleApdu()` with INS switch
   - **Rust**: `Instruction` enum parsed from `comm.next_event()`, match on enum variants
5. **Enumerate all INS codes** — build the full table of APDU commands
6. **Identify coin-specific patterns**:
   - **UTXO-based** : Trusted inputs, multi-phase signing, script validation
   - **Account-based** : Transaction parsing, program/contract dispatch
7. **Identify swap/library mode** — does the app expose `library_main()` or handle `os_lib_call`?
8. **Identify PKI integration** — trusted info provisioning, CAL descriptors, certificate validation

### 0.2 Build the Call Graph

For each INS handler, trace the call chain to leaf functions. Priority:
- Handlers that process **variable-length untrusted data** (sign TX, provide info, set plugin)
- Handlers that **display data to the user** (UI formatting, summary building)
- Handlers that **derive keys or sign** (cryptographic operations)
- Handlers that **parse complex wire formats** (RLP, PSBT, Solana compact format, ABI encoding)

### 0.3 Application Archetypes

| Archetype | Examples | Key Parsing | Crypto | Attack Surface |
|-----------|----------|-------------|--------|----------------|
| **UTXO** | Bitcoin, Litecoin, ZCash | Varint, scripts, segwit, trusted inputs | SECP256K1, SHA256d, RIPEMD160, BLAKE2b | Script injection, amount overflow, trusted input replay |
| **Account/EVM** | Ethereum, Polygon | RLP, ABI, EIP-712, EIP-191 | SECP256K1, Keccak256 | Plugin result overwrite, clear-sign bypass, type confusion |
| **Account/Non-EVM** | Solana, Stellar | Compact arrays, program dispatch | Ed25519, SHA256 | Program ID spoofing, ATA substitution, cross-chain path |
| **Boilerplate** | C/Rust templates | Simple JSON/binary, varint memo | SECP256K1/Ed25519, Keccak256 | Multi-chunk state, memo injection, blind-sign trigger |

---

## Phase 1: Trust Model & Threat Model

### 1.1 Ledger Trust Hierarchy

| Entity | Trust Level | Controls |
|--------|------------|----------|
| **Ledger firmware/OS** | Fully trusted | SDK syscalls, app isolation, memory protection |
| **Exchange app** | Trusted (Ledger-signed) | Calls `library_main()` / `os_lib_call` for swap |
| **PKI certificates** | Semi-trusted | Issued by Ledger backend, per-usage scoped |
| **Companion app / host** | **UNTRUSTED** | Sends APDUs, can be malicious |
| **Blockchain data** | **UNTRUSTED** | RLP, scripts, calldata, instructions — attacker-controlled |

### 1.2 Key Implications

- **APDUs are the primary attack surface.** Any bug reachable only from `library_main()` (Exchange app) is defense-in-depth only — NOT exploitable by external attacker.
- **PKI-gated operations** require a Ledger-issued certificate. Bugs behind PKI checks have reduced exploitability.
- **Plugins/custom programs** receive untrusted calldata — high-priority targets.
- **All wire-format data is attacker-controlled** — all parsing must be robust.

### 1.3 Attacker Model

The attacker is the **companion app / host** sending APDUs to the device over USB/BLE. They can:
- Send arbitrary APDU sequences (any order, any data)
- Craft malicious transactions in any wire format
- Provide forged token/program info (unless PKI-validated)
- Interleave commands to exploit state machines
- Skip or replay multi-chunk steps
- **Cannot** call `library_main()` (only Exchange app can)
- **Cannot** forge PKI certificates (unless Ledger infra is compromised)

### 1.4 Privilege Verification

Verify application privileges adhere to least privilege:
- [ ] Application flags: minimal required permissions only
- [ ] Derivation paths restricted to coin-specific BIP32 prefixes
- [ ] Curves restricted to only those required by the application

---

## Phase 2: APDU Dispatcher & Entry Point Enumeration

### 2.1 Map Every INS Handler

For each handler, document:
```
| INS | Name | Handler function | Stateful? | PKI-gated? | User-confirmation? | Multi-chunk? |
```

### 2.2 Classify Entry Points by Risk

**HIGH RISK** (process untrusted variable-length data):
- Sign transaction handlers (all archetypes)
- Sign message / off-chain message handlers
- Provide token/NFT/trusted info (before PKI validation)

**MEDIUM RISK** (simpler parsing, often PKI-gated):
- Set external plugin / provide instruction descriptor
- Provide network info / dynamic descriptor
- Trusted input generation (Bitcoin)

**LOW RISK** (minimal parsing, no user-facing display):
- Get public key (without display)
- Get app configuration / version
- Get challenge (anti-replay)

### 2.3 Dispatcher Robustness

Verify the dispatcher:
- [ ] Rejects unknown CLA with appropriate status word
- [ ] Rejects unknown INS with appropriate status word
- [ ] Validates P1/P2 before routing (C: manual check; Rust: enum parse rejects invalid)
- [ ] Returns error SW on malformed data — never crashes or hangs

### 2.4 Language-Specific Dispatch Patterns

**C Pattern** — switch/case with manual validation:
```c
switch (cmd->ins) {
    case INS_SIGN_TX:
        if (cmd->p1 > P1_MAX || cmd->p2 > P2_MAX)
            return io_send_sw(SW_INCORRECT_P1_P2);
        return handler_sign_tx(cmd);
}
```

**Rust Pattern** — type-safe enum with compile-time enforcement:
```rust
enum Instruction {
    GetVersion,
    GetPubkey { display: bool },
    SignTx { chunk: u8, more: bool },
}
// Invalid P1/P2 → Err(AppSW::WrongP1P2) at parse time
```

---

## Phase 3: Memory & State Analysis

### 3.1 Global State Inventory

Ledger apps use **global static buffers** (no heap in C, limited Vec in Rust). Map:

| Pattern | C Apps | Rust Apps |
|---------|--------|-----------|
| **Global context** | `G_context` struct with union fields | `TxContext` struct passed as `&mut` |
| **State discriminant** | `appState` / `transactionState` enum | Enum variant or `state` field |
| **Transaction buffer** | `raw_tx[MAX_LEN]` static array | `Vec<u8>` (heap, but bounded) |
| **Multi-hash** | Union of `cx_sha256_t` / `cx_blake2b_t` / `cx_keccak_t` | SDK hash structs |
| **Plugin/program state** | Per-plugin context struct | N/A (no plugin in Rust boilerplate) |
| **NVM settings** | `N_storage` with `nvm_write()` | `AtomicStorage` in `.nvm_data` section |

### 3.2 State Machine Coherence

For multi-APDU flows (sign TX, trusted input, EIP-712):
- Can the attacker **skip steps** (send chunk 3 without chunk 1)?
- Can the attacker **replay steps** (send the same chunk twice)?
- Can the attacker **interleave flows** (start sign TX, then provide token info)?
- Is state **properly cleared on error/abort**?
- Is the **state discriminant checked** before union/variant access?
- Do multi-step operations **enforce correct ordering**?

### 3.3 Archetype-Specific State Concerns

**UTXO (Bitcoin)**:
- Trusted input state machine (11 states) — order enforcement critical
- SegWit hash cache reuse across inputs — cache poisoning possible?
- Change output validation — path must be BIP44 change (index 0 or 1)
- Multi-input signing — `alreadySignedInputs` counter integrity

**Account (Ethereum)**:
- `shared_context` union — `appState` as discriminant
- Plugin result persistence across `provide_parameter` calls
- EIP-712 recursive type state — cycle detection needed

**Account (Solana)**:
- `ApduState` progression: Uninitialized → PayloadInProgress → PayloadComplete
- `g_trusted_info` — persists across APDUs, used at signing time
- Off-chain message header validation — format byte determines display mode

**Rust**:
- `TxContext` lifetime — borrowed references to swap params
- Vec accumulation — `MAX_TRANSACTION_LEN` enforcement per chunk
- BSS sharing with Exchange app — heap forbidden in `check_address`/`get_printable_amount`

### 3.4 Memory Constraints & Rules

The RAM is limited. Verify:

| Rule | C Check | Rust Check |
|------|---------|------------|
| **Dynamic allocation** | SDK pool allocator (`mem_alloc`) available — OOM returns NULL, check all returns; prefer static global buffers | Vec allowed but bounded; forbidden in swap callbacks (BSS sharing) |
| **No recursion** | Forbidden — prevents stack overflow | Avoid deep call stacks (same constrained stack) |
| **No float/double** | Forbidden — use fixed-point or SDK `cx_math_*` | Same (no_std, no FPU) |
| **Prefer memmove/memset** | Over manual byte loops | Use slice operations |
| **Validate lengths before copy** | `dataLength` checked against expected before any memcpy | `.get()` or explicit bounds before indexing |
| **String safety** | `strlcpy` or explicit bounds | `&str` / bounded `ArrayString` |

---

## Phase 4: Vulnerability Hunting — Multi-Category

### 4.1 Memory Safety

| Pattern | Where to Look | C Risk | Rust Risk |
|---------|---------------|--------|-----------|
| **OOB read/write** | Wire format parsing | HIGH | LOW (bounds checked) |
| **Integer overflow/underflow** | Length fields, counter decrements, amount arithmetic | HIGH | MEDIUM (wrapping) |
| **Truncation** | `uint32_t → uint16_t`, `u64 → u8` in length fields | HIGH | MEDIUM (`as` casts) |
| **Stack overflow** | Recursive descent (EIP-712, nested ABI arrays) | HIGH | HIGH (same stack) |
| **Buffer underrun** | Decrement before zero-check | HIGH | LOW (Option/Result) |
| **Uninitialized memory** | Union fields, conditional init | MEDIUM | LOW (compiler enforced) |

#### C-Specific Dangerous Patterns:
```c
// No bounds check before access
uint8_t len = data[offset]; // offset >= dataLength?

// Integer truncation
uint32_t raw = U4BE(data, 0);
uint16_t len = (uint16_t)raw; // Truncation if raw > 65535

// Underflow without zero-check
--context->remaining; // remaining == 0?

// Deprecated exception API — leaks state on throw
THROW(EXCEPTION); // FORBIDDEN — use non-throwing SDK alternatives
```

#### Rust-Specific Dangerous Patterns:
```rust
// Silent truncation via as-cast
let len = raw_value as u16; // Attacker controls raw_value

// Panic on untrusted input
let tx: Tx = from_slice(&data).unwrap(); // Panics on malformed

// Unsafe without justification
unsafe { ptr::read(addr) } // Missing SAFETY comment, unverified invariants
```

#### Expected Safe Patterns:
```c
// C: bounds check THEN access
if (dataLength < offset + expected_size) return SW_WRONG_DATA_LENGTH;

// C: explicit_bzero after crypto
bip32_derive_ecdsa_sign_hash_256(...);
explicit_bzero(&private_key, sizeof(private_key));

// C: non-throwing crypto
cx_err_t err = cx_hash_no_throw(&ctx, CX_LAST, ...);
if (err != CX_OK) return SW_SECURITY_ERROR;
```
```rust
// Rust: error propagation, no unwrap
let path = Bip32Path::try_from(data).map_err(|_| AppSW::WrongApduLength)?;

// Rust: checked arithmetic
let total = amount.checked_add(fee).ok_or(AppSW::TxParsingFail)?;

// Rust: bounds-checked accumulation
if ctx.raw_tx.len() + data.len() > MAX_TRANSACTION_LEN {
    return Err(AppSW::TxWrongLength);
}
```

### 4.2 Clear-Signing Bypass (THE critical vuln class for wallets)

The attacker's goal: make the user sign something **different from what's displayed**.

The UI is the fundamental part of the embedded application. All sensitive operations (signing, public key export) must be preceded by explicit user validation. The screen must accurately represent the buffer being signed.

| Pattern | Description | All Archetypes |
|---------|-------------|----------------|
| **Hash/length mismatch** | Displayed message length ≠ hashed length | ✓ |
| **Truncated display** | Only first N bytes shown, rest signed blindly | ✓ |
| **Wrong address format** | Incorrect encoding (Base58 vs hex vs Bech32) | ✓ |
| **Value overflow** | Amount wraps, displayed value < actual value | ✓ |
| **Missing fields** | Critical fields not displayed (fee, nonce, chainId) | ✓ |
| **Blind signing** | Must be disabled by default behind a settings flag | ✓ |

#### UTXO-Specific:
| Pattern | Description |
|---------|-------------|
| **Trusted input forgery** | HMAC bypass → attacker provides wrong amount |
| **Change address substitution** | Non-change path accepted as change → steals funds |
| **Script mismatch** | Displayed address doesn't match signed script |
| **Multi-input amount sum** | Total input amount overflow across inputs |
| **SegWit/legacy confusion** | Wrong hashing algorithm applied |

#### Account-Specific (EVM):
| Pattern | Description |
|---------|-------------|
| **Proxy substitution** | Name for implementation shown for proxy address |
| **EIP-712 type confusion** | Malformed typed data parsed as valid |
| **RLP length manipulation** | Oversized RLP field bypasses display |

#### Account-Specific (Solana):
| Pattern | Description |
|---------|-------------|
| **Program ID spoofing** | Unknown program displays as known (e.g., System) |
| **ATA owner substitution** | Attacker's ATA shown as victim's |
| **Cross-program invocation** | Nested instructions not fully displayed |
| **Fee hiding in user mode** | High compute budget not shown without expert mode |

### 4.3 Cryptographic Misuse

Secrets and crypto rules — all crypto through SDK functions, never custom implementations:

| Pattern | Impact | Where to Check |
|---------|--------|----------------|
| **Key not cleared** | Private key material in RAM after use | After `bip32_derive_*`, `eddsa_sign_*` |
| **Deprecated THROW in crypto** | Exception leaks partial state — use non-throwing SDK | C apps with `cx_*` calls that can throw |
| **Wrong curve/derivation** | Sign with wrong key | Curve parameter validation |
| **Missing structure validation** | Sign attacker-controlled message without prefix | Message signing handlers |
| **Cross-chain path allowed** | App signs for other coin's derivation | BIP44 coin-type enforcement |
| **Nonce reuse** | Deterministic nonce generation missing | Must use RFC6979 (ECDSA) or Ed25519 (inherent) |
| **Secrets exported/shown** | Seed-derived secrets must never leave device | Key export handlers |

### 4.4 APDU Protocol Violations

| Pattern | Impact | All Languages |
|---------|--------|---------------|
| **Missing length validation** | OOB access | C: `data[offset]` without check; Rust: index panic |
| **State confusion** | Handler doesn't check current state | ✓ |
| **Partial init on error** | Half-initialized state persists | ✓ |
| **P1/P2 not validated** | Unexpected values accepted | C only (Rust enum enforces) |
| **Multi-chunk skip** | Chunk N accepted without chunk 0 | ✓ |
| **Chunk replay** | Same chunk processed twice | ✓ |

### 4.5 Program-Specific Patterns

**Solana Programs**:
| Pattern | Impact |
|---------|--------|
| **Account index OOB** | `instruction->accounts[i]` where `i >= accounts_length` |
| **Program ID comparison** | Only `memcmp` first N bytes instead of full 32 |
| **Instruction data underflow** | `data_length < expected` not checked |
| **PDA derivation bypass** | Nonce search allows attacker-controlled seed |

**Bitcoin Scripts**:
| Pattern | Impact |
|---------|--------|
| **Script length overflow** | `scriptRemaining` underflows past zero |
| **Opcode injection** | Unexpected opcodes in redeemScript |
| **Multisig threshold bypass** | M-of-N validation circumvented |
| **P2SH nesting depth** | Recursive script-within-script |

### 4.6 PKI / Certificate Handling

| Pattern | Impact |
|---------|--------|
| **Wrong usage constant** | Feature accepts certificate meant for different purpose |
| **Missing chain validation** | Self-signed cert accepted |
| **Usage conflation** | Two features share same usage constant |
| **Challenge replay** | Stale challenge accepted |
| **TLV parsing OOB** | Malformed TLV triggers buffer overflow |

### 4.7 Swap / Library Mode Patterns

| Pattern | Impact |
|---------|--------|
| **Amount validation bypass** | Swap amount ≠ transaction amount |
| **Recipient substitution** | Swap recipient ≠ transaction recipient |
| **UI bypass without validation** | Skips display but doesn't validate swap params |
| **BSS corruption (Rust)** | Heap allocation in check_address/get_printable_amount |
| **Fee manipulation** | Swap fee ≠ actual transaction fee |

### 4.8 Code Quality Items Impacting Security

Flag these as they indirectly increase vulnerability surface:
- **Magic numbers**: Prefer enums, macros, or `sizeof`-based expressions over raw numeric literals
- **High cyclomatic complexity**: Functions >~100 lines or deeply nested — harder to audit, more error-prone
- **Compiler/clippy warnings**: Must compile without warnings; warnings must not be silenced without documented reason

---

## Phase 5: APDU Reachability Verification

**This is the critical differentiator from generic code review.**

For EVERY finding rated HIGH or above, trace the COMPLETE path:

```
APDU IN (CLA=0xE0, INS=0xXX, P1, P2, data)
  → dispatcher
    → handler function
      → intermediate calls
        → VULNERABLE CODE
```

### 5.1 Reachability Questions

1. **Which INS code reaches this code?** Follow the call graph backward.
2. **Is there a `library_main()` / swap-only path?** If yes → NOT exploitable externally.
3. **Are there guards (PKI check, settings flag, state requirement) before the vulnerable code?**
4. **What state must be set up first?** (prior APDUs needed to reach the state)
5. **Can the attacker actually provide the triggering input via APDU data?**
6. **Is there a length/bounds check upstream that prevents the triggering condition?**

### 5.2 Reachability Classification

| Category | APDU Reachable? | Exploitability |
|----------|----------------|----------------|
| Direct APDU handler, no guards | **Yes** | High |
| APDU handler behind PKI check | **Yes** (reduced) | Medium (needs cert) |
| APDU handler behind settings flag | **Yes** (conditional) | Medium (needs user enable) |
| Only via `library_main()` / Exchange | **No** | Defense-in-depth only |
| Requires OOM / impossible state | **No** | Theoretical only |
| Guarded by upstream validation | **No** | False positive |
| Rust: panics on malformed (not bypass) | **No** (availability only) | Not exploitable |

---

## Phase 6: False-Positive Elimination

### 6.1 Common False-Positive Patterns in Ledger Apps

| False Positive Pattern | Why It's Not a Bug |
|------------------------|-------------------|
| `mem_alloc` returns → "memory leak" | SDK pool allocator, reset between operations |
| `handle_check_address` OOB → "CRITICAL" | Only callable from Exchange (trusted) |
| Incremental parser OOB → "overflow" | Guarded by feed-based `can_decode()` check |
| Recursive types → "stack overflow" | Cycle detection exists (membership check) |
| Union access → "type confusion" | State discriminant (`appState`) checked |
| Swap flows missing validation → "bypass" | Exchange app is trusted |
| `CX_ASSERT` / `LEDGER_ASSERT` → "crash" | Intentional abort on crypto failure |
| Plugin `RESULT_ERROR` → "continues" | Checked by caller (`result_check()`) |
| Rust `Vec` growth → "OOM" | Bounded by `MAX_TRANSACTION_LEN` check |
| Rust `as` cast → "truncation" | Value provably within target range |
| Bitcoin varint 0xFF → "assertion" | Intentional rejection (64-bit unsupported) |

### 6.2 Verification Protocol

For EACH finding, before including in report:

1. **Read the actual source** — not just grep output or function signatures
2. **Check the caller** — what values can it actually pass?
3. **Check guards upstream** — is there a length check N lines above?
4. **Trace the state machine** — can this state actually be reached?
5. **Verify the trust model** — who controls this input?
6. **Check language guarantees** — does Rust's type system prevent this?

Only include findings **confirmed by evidence** (sanitizer output, demonstrable APDU sequence, or rigorous source-level proof). No speculative reports.

### 6.3 Elimination Criteria

A finding is a **false positive** if ANY of:
- The triggering input is validated/bounded by caller before reaching vulnerable code
- The code path is unreachable from any APDU (only from trusted Exchange app)
- The state required is impossible to reach via normal or malicious APDU sequences
- The "vulnerability" results in **rejection/crash** (not bypass) — feature broken, not security broken
- The allocated buffer is always large enough due to compile-time or SDK guarantees
- The Rust type system (borrow checker, bounds checks) prevents the condition at compile time
- The HMAC/PKI check upstream prevents attacker-controlled input from reaching the code

---

## Phase 7: Exploitability Classification

### 7.1 Severity Levels

| Level | Criteria | Blocks merge? |
|-------|----------|---------------|
| **🟥 CRITICAL** | Clear-signing bypass, user signs different from displayed, arbitrary code execution, data corruption | YES |
| **🟧 HIGH** | APDU-reachable corruption/logic error requiring specific conditions. State corruption. Missing test coverage for key security path. Code/doc/test desynchronization on security-critical behavior. | YES |
| **🟨 WARNING** | Code quality issue, defense-in-depth concern, bug not APDU-reachable. Style, naming, minor gaps. | NO |
| **ℹ️ INFO** | Observation or suggestion. No action required. | NO |

**Verdict**: FAIL requires ≥1 CRITICAL or HIGH. WARNING/INFO alone → PASS with observations.

### 7.2 Exploitability Factors

Rate each confirmed finding on:
- **Attack vector**: APDU (remote via host) vs library_main (trusted only)
- **Complexity**: Single APDU vs multi-step state setup
- **User interaction**: None (transparent) vs requires user approval of corrupt display
- **Impact**: What does the attacker gain? (key leak, wrong address, wrong amount, crash)
- **Scope**: Single transaction vs persistent compromise

---

## Phase 8: PoC / Sanitizer Confirmation

### 8.1 C Apps: ASan/UBSan Harness

Write minimal C test harnesses that **call the target function directly**:

```c
// test_finding.c — in working folder, NEVER modify target source
#include "stubs.h"
#include "../../src/path/to/target.c"

int main(void) {
    // Set global state to REACHABLE value (justify reachability)
    // Call the function with triggering input
    // ASan/UBSan will detect the issue
    return 0;
}
```

```bash
clang -fsanitize=address,undefined -fno-omit-frame-pointer -g \
    test_case.c target.c [stubs.c] -o test_case
./test_case
```

A crash or sanitizer diagnostic **confirms** the finding. If the test does not trigger, discard or refine.

### 8.2 Rust Apps: Miri / Cargo Test

```rust
#[test]
fn test_truncation_exploit() {
    let malicious_length: u32 = 0x00010001; // 65537 → truncated to 1
    let result = parse_message_length(malicious_length);
    // Verify the vulnerability manifests
}
```

Run: `cargo +nightly miri test` (unsafe) or `cargo test` (logic bugs)

### 8.3 Python APDU PoC (Ragger/Speculos)

Write PoC tests using named variables (no raw magic hex), with a `CommandSender` abstraction when available:

```python
"""PoC: [Finding title]"""
from ragger.backend import SpeculosBackend

def test_vulnerability_poc(backend: SpeculosBackend):
    # Named variables for semantic clarity
    derivation_path = bytes.fromhex("058000002c8000003c800000000000000000000000")
    msg_length = (0x00010001).to_bytes(4, 'big')  # 65537 → truncated to 1
    message = b"A"

    payload = derivation_path + msg_length + message
    rapdu = backend.exchange(cla=0xE0, ins=0x08, p1=0x00, p2=0x00, data=payload)
    # Observe: hash prefix says "65537" but only 1 byte processed
```

### 8.4 Confirmation Requirements

| Severity | Required Evidence |
|----------|-------------------|
| CRITICAL | Mandatory ASan/Miri PoC + APDU PoC on Speculos |
| HIGH | ASan/Miri PoC OR APDU PoC (at least one) |
| WARNING | Source-level justification sufficient |
| INFO | Observation only |

---

## Phase 9: Coherence Check & Quality Gate

Beyond pure security, verify overall coherence — a desynchronized codebase hides bugs.

### 9.1 Code ↔ Documentation Coherence

- [ ] INS codes in code match those described in documentation (`doc/APDU.md` or equivalent)
- [ ] P1/P2 values and semantics match between code and docs
- [ ] Error status words match between code and docs
- [ ] CLA in header matches documentation

### 9.2 Code ↔ Tests Coherence

- [ ] Python tests cover the logic actually implemented (not outdated protocol)
- [ ] Error paths are tested (invalid inputs, edge cases, malicious inputs)
- [ ] User rejection tested where applicable
- [ ] New features have corresponding test coverage

### 9.3 Tests ↔ Documentation Coherence

- [ ] Tests respect the protocol defined in documentation
- [ ] Test APDU payloads match documented format

### 9.4 Test Quality

- [ ] Every device listed in `ledger_app.toml` has functional tests
- [ ] Tests use named variables and `struct.pack` — no raw magic hex for complex payloads
- [ ] UI verification uses `navigate_and_compare()` for signing/key export flows
- [ ] Edge cases tested: empty data, max-length data, boundary values

### 9.5 Desynchronization = Finding

If code implements behavior X but documentation says Y, or tests validate Z:
- If security-relevant → classify as 🟧 HIGH
- If not security-relevant → classify as 🟨 WARNING

---

## Phase 10: Final Report Generation

### 10.1 Report Structure

```markdown
# Security Review — [App Name] — Final Report

> **Date**: YYYY-MM-DD
> **App Language**: C / Rust
> **Archetype**: UTXO / Account-EVM / Account-Non-EVM / Boilerplate
> **Scope**: [files/LOC reviewed]
> **Verdict**: PASS / FAIL (FAIL if any HIGH or CRITICAL)
> **False positives eliminated**: N/M

## 🟥 CRITICAL (N)
## 🟧 HIGH (N) — all APDU-reachable
## 🟨 WARNING (N)
## ℹ️ INFO (N)

## APDU Exploitability Table
| Finding | INS | Reachable? | Exploitability |

## Coherence Check Results
| Check | Status | Notes |

## False Positives Eliminated
| # | Original classification | Reason for elimination |

## Methodology
[Phases completed, areas reviewed, limitations]
```

### 10.2 Per-Finding Template

```markdown
### HN. [Short title]

- **Severity**: 🟥/🟧/🟨/ℹ️
- **File**: `relative/path.c:line` in `function_name()`
- **Language**: C / Rust
- **APDU**: `INS_XXX (0xNN)`, P1=, P2=
- **CWE**: CWE-NNN (Name)
- **Exploitability**: High / Medium / Low
- **Trigger conditions**: [entry point, state, input]

**Description**: [What the bug is, with code snippet]

**Test case**: `test_findingN.c` / `test_findingN.py`
**Sanitizer output**: [key lines] (if applicable)

**Fix**: [Minimal code change]
```

### 10.3 Verdict Criteria

- **FAIL**: At least one CRITICAL or HIGH finding that is APDU-reachable
- **PASS with observations**: Only WARNING/INFO findings
- **PASS**: No findings

If overall quality is too low (excessive complexity, many warnings, significant desync), state so explicitly with supporting evidence.

---

## Appendix A: CWE Mapping for Ledger Apps

| CWE | Ledger Context | Archetypes |
|-----|----------------|------------|
| CWE-120 (Buffer Overflow) | Wire format parsing without length check | All (C) |
| CWE-125 (OOB Read) | Plugin calldata / instruction data beyond bounds | All |
| CWE-191 (Integer Underflow) | Counter decrement without zero check | All (C) |
| CWE-681 (Incorrect Conversion) | U4BE → uint16_t truncation, `as` cast in Rust | All |
| CWE-457 (Uninitialized) | Plugin result not set on all paths, union field | C |
| CWE-476 (NULL Dereference) | mem_alloc return unchecked | C |
| CWE-843 (Type Confusion) | Union access without state check | C |
| CWE-347 (Improper Verification) | PKI usage constant wrong/shared | All |
| CWE-345 (Insufficient Verification) | Display data ≠ signed data | All |
| CWE-346 (Origin Validation) | Cross-chain derivation path allowed | All |
| CWE-327 (Broken Crypto) | Custom crypto instead of SDK, wrong curve | All |
| CWE-312 (Cleartext Storage) | Private key not zeroed after use | All |
| CWE-362 (Race Condition) | State machine interleaving between APDUs | All |

---

## Appendix B: Multi-Chunk Protocol Security Checklist

| Check | How to Verify |
|-------|---------------|
| **Chunk 0 resets state** | First chunk clears all prior accumulated data |
| **Chunk order enforced** | Cannot send chunk N without chunk 0..N-1 first |
| **P2 "more" flag respected** | App waits for last chunk before processing |
| **Total size bounded** | Accumulated length checked against MAX before extending |
| **Interleaving rejected** | Starting a new INS clears in-progress state |
| **Error clears state** | Parse failure resets to initial state |
| **Signature only on last chunk** | Crypto operations only after full payload received |

---

## Appendix C: Docker Environment for PoC

```bash
# Discover available image
docker images | grep ledger

# C app build + test
docker run --rm -v $(pwd):/app ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest \
  bash -c "source /opt/venv/bin/activate && \
           pip install -r tests/requirements.txt && \
           BOLOS_SDK=\$NANOX_SDK make -j && \
           pytest tests/ --device nanox -k test_poc"

# Rust app build + test
docker run --rm -v $(pwd):/app ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest \
  bash -c "source /opt/venv/bin/activate && \
           pip install -r tests/requirements.txt && \
           cargo ledger build nanox && \
           pytest tests/ --device nanox -k test_poc"

# ASan harness (host-side, not device)
clang -fsanitize=address,undefined -fno-omit-frame-pointer -g test_poc.c -o test_poc
./test_poc
```

