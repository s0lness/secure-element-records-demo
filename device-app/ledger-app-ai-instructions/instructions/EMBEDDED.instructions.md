---
description: "Ledger embedded platform constraints shared by C and Rust applications"
applyTo: "**/*"
---

# Ledger Embedded Platform Rules

These rules apply to all embedded code (C and Rust) running on Ledger devices.

## Application Privileges

- Application flags must abide by the principle of least privilege.
- Derivation paths must be restricted to coin-specific BIP32 prefixes.
- Curves usage must be restricted to only those required by the application.

## User Interface and Clear Signing

- The UI is the fundamental part of the embedded application, NOT a cosmetic side. Ensure all sensitive operations (signing, public key export) are preceded by an explicit user validation screen. Flag any "blind signing" patterns or flows where the screen doesn't accurately represent the buffer being signed.
- If blind signing is implemented, it must be disabled by default behind a settings flag.
- Critical and important information must be clear signed using a user-friendly format. The user must not be confused or tricked by the application workflow or displayed information.

## Security and Availability

- Security strictly overrides availability; the application MUST always "fail closed."
- When encountering an unexpected issue, the application must refuse to proceed with the standard flow and return an error.
- It is NEVER acceptable to silence a security issue to preserve availability. Even a crash is preferable to executing in an unverified or compromised state. This BANS eager patterns such as dubious fallbacks, default values, clamping, etc.

## Memory and Runtime

- The RAM is limited to around 24 kilobytes. Ensure that the code is optimized for low memory usage and does not contain unnecessary allocations or unnecessarily large data structures.
- Remember that the RAM is reset on every power cycle.

## Secrets and Cryptography

- Ensure sensitive data such as private keys are explicitly cleaned from memory as soon as possible after usage.
- Secrets derived from the seed must not be stored, exported, or shown to the user.
- Cryptographic calls must be made through the SDK's functions, not implemented in application code. Ensure that all cryptographic operations are performed using these functions and that they are used correctly to maintain security and performance.
- Structure integrity of messages must be verified before signature. Never allow signing of attacker-controlled messages.

## APDU Handling

- APDUs are the sole entry point of the application. Ensure the code treats the incoming APDUs as untrusted input and implements proper validation and error handling to prevent potential security vulnerabilities. Look for robust parsing of APDU commands, validation of input data, and appropriate responses to invalid or malicious requests.

## Comments

- Comments are TIMELESS and must remain true and useful years later; do not use them to describe how things **were**, nor to forward information to the AI caller.
- Avoid comments that describe what the code is doing, unless it's doing something non-obvious. Focus on the intent and rationale behind the code rather than restating the code's functionality. Basic code understanding should come from proper variable and function names.
- Comments should stay concise and dense in information; multiline comments should be rare.
- Comments should respect separation of concerns; do not bring outside knowledge where it does not belong.
