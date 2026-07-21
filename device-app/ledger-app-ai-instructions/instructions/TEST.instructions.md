---
description: "Ledger application test writing rules using Ragger, Pytest, and Speculos"
applyTo: "**/*"
---

# Ledger Test Writing Rules

Python is used exclusively for testing Ledger device applications — it is not part of the embedded application.

## Framework

- **Ragger** (Python + Pytest): [github.com/LedgerHQ/ragger](https://github.com/LedgerHQ/ragger)
- **Speculos** emulator: [github.com/LedgerHQ/speculos](https://github.com/LedgerHQ/speculos)
- Test directory and supported devices come from `ledger_app.toml`.

## Test Readability

- **No magic hex:** Do not use raw hex strings (e.g., `bytes.fromhex("050012...")`) for complex APDU payloads.
- Use named variables (`amount`, `derivation_path`, `fee`) and `struct.pack` to construct payloads with clear semantic meaning.
- Use a `CommandSender` abstraction (for example in `application_client/`) to encapsulate APDU construction and response parsing.

## UI Verification

- For critical actions (signing, key export), verify that the device displays the correct information before the user approves.
- Use the Ragger `navigator` to simulate user interaction: button presses on Nano devices, touch events on Stax/Flex/Apex.
- Use `navigator.navigate_and_compare()` to check screen content against reference images (Golden Snapshots).
- Snapshots and tmp snapshots are handled by the framework, NEVER delete them manually, this is USELESS and error prone.

## How to Run Ragger Tests

### Configuration

- The test directory path is defined in `ledger_app.toml` under `[pytest.standalone].directory`. Do NOT hardcode it.
- The supported devices are listed in `[app].devices` inside `ledger_app.toml` in root directory. Every listed device MUST have functional tests.
- The `requirements.txt` is located at `<test_dir>/requirements.txt`.
- Map `ledger_app.toml` device names to pytest `--device` values:

| `ledger_app.toml` | `--device` value |
| :--- | :--- |
| `nanos+` | `nanosp` |
| `nanox` | `nanox` |
| `stax` | `stax` |
| `flex` | `flex` |
| `apex_p` | `apex_p` |

### Docker Environment

- **Image discovery:** Run `docker images | grep ledger` before any `docker run`. Do NOT assume a hardcoded image name.
- **Common image:** `ghcr.io/ledgerhq/ledger-app-builder/ledger-app-dev-tools:latest` (includes builder, Speculos, Python venv).
- **Volume mount:** Mount the project root to `/app` inside the container.
- **Venv activation:** Always start with `source /opt/venv/bin/activate && pip install -r <test_dir>/requirements.txt` before running pytest.
- **Host OS adaptation:** Adapt Docker commands to the host OS (e.g., shell syntax for current directory, variable escaping).

### Test command

Run the tests in the docker environment using pytest. Example for nanox device:
```
pytest <test_dir>/ --tb=short -v --device nanox
```

The test framework uses snapshot navigation to ensure non regression of screens and correctness of displayed elements. The `--golden_run` argument will regenerate the snapshots: use this option conservatively to not silence screen regressions.
Do **NOT** under **ANY** circumstance attempt to manually delete the snapshots or the temporary snapshots.
When iterating on tests, use `-k` to run only a subset of tests (e.g., `-k "test_signing"`) to optimize time. Run option `--collect-only` to explore existing test names.

## Coverage Requirements

Every tested feature must include:
- Happy path
- Error paths (invalid inputs, edge cases, malicious inputs)
- User rejection (where applicable)
- Edge cases (empty data, max-length data, boundary values)
