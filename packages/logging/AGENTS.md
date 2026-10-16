# @agentctl/logging

Holds the sole implementation of the `Logger` port using `pino`: log
initialization, the log directory, the current log file and log pruning. It is
the only package allowed to import `pino`, and it must not contain agent state,
tmux, harness or business logic.
