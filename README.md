# Project Name to be discuessed

currently we have following part:

- mora: a morden editor aim to replace coldnew-emacs
- serana: my personal agent
- mishell: bash syntax shell for mora

Serana docs:

- `docs/architecture.md`: current module layout and dependency graph
- `docs/serana-refactor-tutorial.md`: where to add or change Serana features

Codex device sign-in:

```bash
cargo run -p serana -- login codex
```

This delegates to `codex login --device-auth`; install the Codex CLI first.
