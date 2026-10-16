# @agentctl/tmux

Holds the sole implementation of the `Tmux` port: running the tmux binary and
the process table lookup, plus parsing their output into core's types. It must
not contain agent-state persistence, logging, harness installation or any
business logic, which belong in `state-fs`, `logging`, the `harness-*` packages
and `core` respectively.
