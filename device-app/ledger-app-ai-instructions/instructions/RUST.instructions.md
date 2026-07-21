---
description: 'Rust programming language coding conventions and best practices'
applyTo: '**/*'
---

# Ledger Embedded Rust Rules

## General Instructions

- Always prioritize readability, safety, and maintainability.
- Use strong typing and leverage Rust's ownership system for memory safety.
- Break down complex functions into smaller, more manageable functions.
- For algorithm-related code, include explanations of the approach used.
- Write code with good maintainability practices, including comments on why certain design decisions were made.
- Handle errors gracefully using `Result<T, E>` and provide meaningful error messages.
- For external dependencies, mention their usage and purpose in documentation.
- Use consistent naming conventions following [RFC 430](https://github.com/rust-lang/rfcs/blob/master/text/0430-finalizing-naming-conventions.md).
- Write idiomatic, safe, and efficient Rust code that follows the borrow checker's rules.
- Ensure code compiles without warnings.

## Patterns to Follow

- Use modules (`mod`) and public interfaces (`pub`) to encapsulate logic.
- Handle errors properly using `?`, `match`, or `if let`.
- Implement traits to abstract services or external dependencies.
- Prefer enums over flags and states for type safety.
- Use builders for complex object creation.
- Use iterators instead of index-based loops as they're often faster and safer.
- Use `&str` instead of `String` for function parameters when you don't need ownership.
- Prefer borrowing and zero-copy operations to avoid unnecessary allocations.

### Ownership, Borrowing, and Lifetimes

- Prefer borrowing (`&T`) over cloning unless ownership transfer is necessary.
- Use `&mut T` when you need to modify borrowed data.
- Explicitly annotate lifetimes when the compiler cannot infer them.

## Patterns to Avoid

- Don't use `unwrap()` or `expect()` unless failure is truly impossible or represents an unrecoverable state.
- Avoid panics in library code—return `Result` instead.
- Avoid deeply nested logic—refactor with functions or combinators.
- Dependencies must be checked with `cargo audit` to ensure no known impactful vulnerabilities are present.
- Avoid `unsafe` unless required and fully documented. All `unsafe` blocks must be justified in a comment.

## Error Handling

- Use `Result<T, E>` for recoverable errors and `panic!` only for unrecoverable errors.
- Prefer `?` operator over `unwrap()` or `expect()` for error propagation.
- Use `Option<T>` for values that may or may not exist.
- Provide meaningful error messages and context.
- Validate function arguments and return appropriate errors for invalid input.

## How to build

### Configuration

- Supported devices are listed in `[app].devices` inside `ledger_app.toml`.
- Rust apps are built per device with [`cargo-ledger`](https://github.com/LedgerHQ/cargo-ledger). The target device is selected by its cargo target name:

  | `ledger_app.toml` device | cargo target |
  | :--- | :--- |
  | `nanos+` | `nanosplus` |
  | `nanox` | `nanox` |
  | `stax` | `stax` |
  | `flex` | `flex` |
  | `apex_p` | `apex_p` |

- The toolchain is pinned in `rust-toolchain.toml`. Building `core`/`alloc` for the device target requires the unstable `build-std` feature, configured in `.cargo/config.toml`.
- `.cargo/config.toml` may declare a default cargo target; commands without an explicit target build for it.
- The application must compile without errors or warnings. Run `cargo fmt --check` and `cargo clippy` before submitting; warnings must not be silenced without a documented reason.
- Heap is 8192 bytes by default. Override via `HEAP_SIZE` env var; allowed values: 2048, 4096, 8192, 16384, 24576.

### Docker Environment

- **Image discovery:** Run `docker images | grep ledger` before any `docker run`. Do NOT assume a hardcoded image name.
- **Common image:** `ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest` (includes the Rust toolchain, `cargo-ledger`, Speculos, enforcer). `cargo build` on the host will not produce a runnable binary — there is no host target.
- **Volume mount:** Mount the project root to `/app` inside the container.
- **Host OS adaptation:** Adapt Docker commands to the host OS (e.g., shell syntax for current directory, variable escaping).

### Build command

Run `cargo ledger build <TARGET>`, replacing `<TARGET>` with the cargo target for the device (output: `target/<TARGET>/release/<app-name>`):

```bash
cargo ledger build nanosplus       # one of: nanox | nanosplus | stax | flex | apex_p
```

Enable debug logging with the `debug` feature:

```bash
cargo ledger build nanosplus -- --features debug
```

### Documentation

- Write clear and concise comments for each function, struct, enum, and complex logic.
- Ensure functions have descriptive names and include comprehensive documentation.
- Document all public APIs with rustdoc (`///` comments) following the [API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Use `#[doc(hidden)]` to hide implementation details from public documentation.
- Document error conditions, panic scenarios, and safety considerations.
- Examples should use `?` operator, not `unwrap()` or deprecated `try!` macro.

## Project Organization

- Use semantic versioning in `Cargo.toml`.
- Include comprehensive metadata: `description`, `license`, `repository`, `keywords`, `categories`.
- Use feature flags for optional functionality.
- Organize code into modules using `mod.rs` or named files.
- Keep `main.rs` or `lib.rs` minimal - move logic to modules.

