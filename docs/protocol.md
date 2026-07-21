# presse protocol v1 (as implemented)

Two roles per ceremony: **master** (device A, artist's plate) and **receiver**
(device B). All traffic goes through an untrusted relay (laptop, later a
phone/reader). The protocol stays secure when the relay lies, drops, replays,
reorders, or substitutes messages. The reference verifier is
tests/presse_client.py (python-ecdsa, shares no code with the device app).

Curve: secp256k1. Hash: SHA-256. MAC/KDF: HMAC-SHA256.
Pubkeys: 65-byte uncompressed SEC1. Signatures: deterministic ECDSA (RFC6979),
DER-encoded, length-prefixed, in a zero-padded 72-byte field.
Multi-byte integers little-endian.

## Keys

- **Device key** `devkey`: TRNG-generated at first use, secret scalar only in
  app NVRAM. Never seed-derived: the owner knows their 24 words, and a
  seed-derived key could re-press off-device. Public part `devpub`.
- **Album key** `albkey`: TRNG-generated at CUT, only in the master's NVRAM.
  `album_id = SHA256(albpub)`. Losing the master destroys the plates, by
  design.

## Certificates

### AlbumCert (177 bytes, signed by albkey)
```
magic      u8[4]   "PRA1"
albpub     u8[65]
title_len  u8
title      u8[32]  (utf-8, zero-padded)
edition    u16     (fixed forever at CUT)
sig_len    u8
sig        u8[72]  (DER, zero-padded; covers bytes 0..104)
```

### PressingCert (178 bytes, signed by albkey)
```
magic      u8[4]   "PRP1"
album_id   u8[32]
number     u16     (1-based, unique, monotonic)
edition    u16
recvpub    u8[65]  (binds the pressing to one device's silicon)
sig_len    u8
sig        u8[72]  (covers bytes 0..105)
```

A verifier accepts a pressing iff: AlbumCert self-verifies, PressingCert
verifies under albpub, album_id == SHA256(albpub), editions match,
1 <= number <= edition, and the presenting device proves live possession of
recvpub via CHALLENGE.

## Pairing (commit-then-reveal ECDH, 4-word SAS)

```
A: eph a, EA        A -> B : C = SHA256("presse-commit" || EA)
B: eph b, EB        B -> A : EB          (B stores C first)
A -> B : EA                              (B checks the hash, aborts hard on mismatch)
both:
  S  = ECDH_x(eph, peer_eph)
  T  = SHA256("presse-sas" || EA || EB)
  K  = HMAC(S, "presse-session" || T)    (session MAC key)
  SAS = HMAC(S, "presse-sas" || T)[0..4] -> 4 words (256-word list)
```

Both devices display the words and wait for a tap. A MITM relay running two
handshakes yields two different S, hence different words: the humans are the
authentication. Grinding is blocked twice: the commitment forbids choosing an
ephemeral after seeing the peer's, and pairing attempts are capped at 8 per
power cycle, so brute-forcing the 32-bit SAS online is out of reach.

After SAS confirmation, every ceremony payload carries
`HMAC(K, [ins, seq] || payload)` with per-direction sequence numbers. Any MAC
failure, SAS rejection, or power cycle kills the session.

## APDU map (CLA 0xB5)

```
0x01 GET_INFO       -> flags(1) devpub(65) edition(2) counter(2) title_len(1) title(32)
0x10 CUT            data = edition(2) title(1..32)     [UI] -> AlbumCert
0x21 PAIR_COMMIT    (master)                    -> C(32)
0x22 PAIR_RESPOND   (receiver) data=C(32)       -> EB(65)
0x23 PAIR_REVEAL    (master)   data=EB(65)      -> EA(65)
0x24 PAIR_FINISH    (receiver) data=EA(65)      -> ok
0x25 PAIR_SAS       (both)                 [UI] -> sas(4)
0x30 GET_ALBUM      (master, paired)            -> AlbumCert || mac(32)
0x31 PRESS_REQUEST  (receiver, paired)          -> devpub(65) || mac(32)
0x32 PRESS_OFFER    (master)   data=devpub||mac [UI] -> PressingCert || mac
0x33 PRESS_LOAD_ALBUM (receiver) data=AlbumCert||mac  -> ok (staged)
0x34 PRESS_ACCEPT   (receiver) data=PressingCert||mac [UI] -> ok (stored)
0x40 GET_BUNDLE     p1=0 PressingCert, p1=1 its AlbumCert (public)
0x41 CHALLENGE      data=nonce(32) -> sig_len(1) || DER sig by devkey
                    over SHA256("presse-verify" || nonce)
0x50 RESET_MASTER   [UI, scary] -> wipes the master
```
[UI] = blocks on an explicit user confirmation on the device screen.
Album + pressing certs travel in separate APDUs because both together exceed
the 255-byte APDU data limit.

## Press semantics

- The master decrements its NVM counter (AtomicStorage, power-loss atomic)
  BEFORE the certificate leaves the device: a power cut burns a number, never
  duplicates one. Numbers are monotonic, never reused; no un-press.
- `number = edition - counter + 1`, refused at counter == 0 ("sold out").
- A receiver holds at most one pressing (v1).

## Threat model status

- Relay key substitution -> different SAS words, humans abort. (tested, M4)
- Fooled humans + tampered/rebound cert -> dies on ECDSA verify, since the
  signature covers recvpub and the MITM lacks the album key. (tested, M4)
- Replay -> sequence counters. Commitment cheating -> hard abort. (tested, M4)
- Over-pressing by the honest app: impossible (counter in silicon).
  Enforcement against a MODIFIED app rests on attestation (BOLOS endorsement),
  NOT in v1: v1's fallback is fraud-evidence (two certs with the same number
  are mutually incriminating). Open question tracked in docs/m5-hardware.md.
- Cloning a receiver = extracting a key from the SE: the explicit bet.
