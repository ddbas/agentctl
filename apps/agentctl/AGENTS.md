# agentctl

Holds the CLI executable and composition root: `commander` wiring, help text,
argument parsing, exit codes, and the one place where concrete adapter packages
are chosen and injected into `core`'s `App`, bundled by Rspack to
`dist/cli.cjs`. It must not contain business logic, state-machine rules or
infrastructure implementations, which belong in `@agentctl/core` and the adapter
packages (`tmux`, `state-fs`, `logging`, `harness-pi`, `harness-claude`)
respectively.
