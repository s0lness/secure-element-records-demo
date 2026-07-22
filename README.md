# Enclave Records

![Enclave Records demo](docs/demo.gif)

*An artist cuts a master, two Ledger Flex pair by comparing four words, a numbered copy is pressed onto the receiver, and anyone verifies it offline.*
▶ [Watch the video (pausable)](docs/demo.mp4)

<!-- Maintainer: to get an inline HTML5 player with play/pause/scrub controls,
     open this README in the github.com editor and DRAG docs/demo.mp4 into it;
     GitHub uploads it and rewrites the link as an embedded player. A committed
     mp4 referenced by path (as above) renders only as a download link. The GIF
     stays for zero-click inline autoplay. -->

Finite editions of digital works, enforced by silicon. An artist device "cuts
a master" of an album (edition size and press counter captive in a secure
element), then "presses" numbered copies onto other devices through an
untrusted relay. Anyone can verify a copy offline: certificate chain +
live challenge-response, no server, no chain, no trust in the middleman.

Runs on two Ledger Flex (or two emulated ones: everything below works with
zero hardware).

## Why this is cool

Streaming turned every song, book and film into a rental. This makes a digital
work ownable again, as a numbered object with real scarcity:

- **The scarcity is physical, not promised.** The edition size lives inside a
  tamper-resistant secure element. Once an artist cuts a master of 5, even they
  cannot press a sixth. No server enforces it; no one can quietly mint more.
- **You hold one specific copy.** "4 of 5", bound to your device's key, provable
  on the spot by a tap. The files can leak everywhere; being one of the five
  cannot be copied.
- **No blockchain, no account, no server.** A copy verifies offline, forever,
  against nothing but a signature. The object outlives the company that made it:
  nothing to shut down, nothing that phones home.
- **It behaves like an object.** Hand it over and it is transferred, like a
  record or a Game Boy cartridge. The cover art travels with the pressing; the
  lineage belongs to the object, not to a ledger.

A working prototype of that idea, on hardware you can buy today.

## How it works

```mermaid
sequenceDiagram
    actor AH as Artist
    participant A as Flex A (master)
    participant R as Laptop (untrusted relay)
    participant B as Flex B (receiver)
    actor BH as Collector

    Note over A: CUT
    AH->>A: upload sleeve, cut album, edition of 5
    A->>A: TRNG album key, seal sleeve hash and edition into a signed AlbumCert

    Note over A,B: PAIR, commit-reveal ECDH through the relay
    A->>R: commitment
    R->>B: commitment
    B->>R: ephemeral key
    R->>A: ephemeral key
    A->>R: reveal
    R->>B: reveal
    Note over A,B: both screens show the SAME 4 words
    AH-->>BH: compare words out loud
    AH->>A: tap Words match
    BH->>B: tap Words match
    Note over A,B: a lying relay makes the words differ, humans abort

    Note over A,B: PRESS
    B->>R: request, device pubkey B
    R->>A: request
    A->>A: counter 5 to 4 in silicon, sign PressingCert bound to pubkey B
    A->>R: PressingCert
    R->>B: PressingCert
    BH->>B: tap Receive

    Note over B: VERIFY, offline, no network
    BH->>B: challenge, a random nonce
    B->>BH: signature by device key and cert chain
    Note over BH: GENUINE, pressing 1 of 5, bound to this device
```

```mermaid
flowchart LR
    AC["AlbumCert: album key, edition size, sleeve hash"]
    PC["PressingCert: number N of M, bound to device key"]
    DEV["the holding secure element proves it owns the bound key, live"]
    PLATES["lose the master, plates destroyed"]
    AC -->|signs| PC
    PC -->|challenge-response| DEV
    AC -.->|album key lives only in the master chip| PLATES
```

## The ceremony

1. **Cut** - Flex A confirms "Cut master of *Random Access Memories*, edition of 5".
   The edition size is fixed forever; losing the device destroys the plates.
2. **Pair** - the two devices run a commit-then-reveal ECDH through the relay;
   both screens show the same 4 words. The humans compare them out loud: a
   man-in-the-middle relay cannot make the two screens agree.
3. **Press** - A signs "pressing 1 of 5, bound to device B's key" and its
   counter decrements in silicon, atomically, before the certificate leaves.
   At 0: sold out, forever.
4. **Verify** - offline: chain verification plus a nonce the holder's secure
   element signs live.

See [docs/protocol.md](docs/protocol.md) for the full protocol and threat
model, and [docs/screens/](docs/screens) for a captured run.

## Layout

- `device-app/` - the Ledger app (Rust, `#![no_std]`, NBGL), targets Flex
- `tests/` - pytest over one or two Speculos instances, including adversarial
  relay tests (MITM key substitution, replay, SAS grinding, cert tampering)
- `relay/` - the untrusted relay: emulator cockpit (two clickable screens +
  live APDU wire on `:5050`), step-by-step ceremony driver, HID transport for
  real devices
- `scripts/` - build (WSL/aarch64-friendly), emulators, sideloading, captures

## Run it

Toolchain (Linux/WSL): rustup + `cargo-ledger` + clang + `gcc-arm-none-eabi`,
the [ledger-secure-sdk](https://github.com/LedgerHQ/ledger-secure-sdk) checked
out at `API_LEVEL_26` (`FLEX_SDK` env var), Speculos + pytest in a venv.
Adapt `scripts/env.sh` to your paths, then:

```
scripts/build.sh        # cargo ledger build flex
scripts/test.sh         # 11 tests, ~30 s, two emulated Flex
scripts/emu-up.sh       # two persistent emulators (:5001, :5002)
scripts/cockpit.sh      # clickable dual-screen cockpit + APDU wire (:5050)
python3 relay/demo_steps.py cut   # then: pair, press, verify
```

Real hardware: [docs/m5-hardware.md](docs/m5-hardware.md) (sideloading via
`ledgerctl`, USB-to-WSL passthrough, same ceremony on two physical Flex).

## Limitations

- **No remote attestation (v1).** The "edition can never exceed N" claim is
  enforced against everyone except a malicious operator running a *modified*
  app: nothing yet proves to a third party that the album key is captive in
  unmodified code. The fallback is fraud-evidence (two certs with the same
  number are mutually incriminating, and a transparency log makes that instant).
  Closing the gap needs BOLOS endorsement, tracked in the docs.
- **The trust root is the silicon.** Verification trusts Ledger's secure
  element and our published app hash. Extracting a key from the secure element
  would break the guarantees: that is the explicit bet.
- **The cover is public, not secret.** The sleeve travels to the receiver
  through the untrusted relay; integrity comes from the signed hash, not
  secrecy. Fine for artwork, not a model for private payloads.
- **Losing the master ends the edition.** The album key lives only in the
  master's chip and is never backed up, so a lost or wiped master can never
  press again. Deliberate ("the plates are destroyed"), but it is a real
  constraint.
- **One pressing per receiver, sideload-only (v1).** A device holds a single
  pressing, and the app is installed by sideloading, not from Ledger's catalog.
  This is a lab prototype, not a shippable product.
