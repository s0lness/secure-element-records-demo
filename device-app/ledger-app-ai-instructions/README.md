# Ledger App AI Instructions

Reusable AI instruction files for Ledger embedded application repositories.

## Usage

Add this repository as a submodule in your application:

```bash
cd app-example/
git submodule add <repo-url> ledger-app-ai-instructions
```

### Option 1: Symlink for GitHub Copilot

Create a symbolic link so that `.github/instructions/` points to the submodule's instruction files:

```bash
cd app-example/.github/
ln -sf ../ledger-app-ai-instructions/instructions instructions
```

### Option 2: CLAUDE.md for Claude

Create a `CLAUDE.md` at the root of your application that references the submodule:

```markdown
@ledger-app-ai-instructions/CLAUDE.md
```

## Files

| File | Scope | Purpose |
|---|---|---|
| `EMBEDDED.instructions.md` | `*.c, *.h, *.rs` | Cross-language embedded constraints (hardware, security, UI) |
| `C.instructions.md` | `*.c, *.h` | C-specific rules, toolchain, build workflow |
| `RUST.instructions.md` | `*.rs` | Ledger-specific Rust deviations (custom test harness, `no_std`) |
| `TEST.instructions.md` | `*.py` | Test writing rules (Ragger, Speculos, snapshots) |
| `REVIEW.instructions.md` | `*` | Code review checklist (security, coherence, quality gate) |
| `SECURITY.REVIEW.instructions.md` | `*` | Full-depth security audit methodology |

## Customization

These instructions cover rules generic across all Ledger embedded applications. For application-specific instructions (custom APDU definitions, specific parsing logic):

- **GitHub Copilot**: use `.github/copilot-instructions.md`
- **Claude Code**: in your `CLAUDE.md`, add additional `@path/to/file.md` imports for app-specific instructions
