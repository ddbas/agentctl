# agentctl

This project implements a tmux-based agent controller. It's goal is to provide a
simple, consistent interface for running agents in tmux, no matter their
underlying harness (e.g. pi, claude, etc.).

## Overview

This project is a Rust monorepo. All crates can be found under the `crates/`
directory.

## `mise`

The project uses `mise` to manage tools, environment variables and tasks.

```bash
mise list                  # list tools managed by mise
mise exec -- <tool>        # run a tool through mise, e.g. mise exec -- cargo build
mise tasks                 # list all available tasks
mise run <task>            # run a task
```

## Git Hooks

This project uses git hooks (managed by `lefthook`) to evaluate code before
commits.

## Rules

- You MUST always call tools via `mise exec -- <tool>`. Never call `cargo` or
  other tools directly (unless the tool is not available through mise; confirm
  with `mise list`).
- You MUST never skip git hooks. If a hook fails, you MUST fix the issue
  reported.
- You MUST always write unit and/or integration tests for new features. All E2E
  and integration tests MUST use `testcontainers` so they are fully isolated
  from the host environment (filesystem, git config, hooks, env vars, etc.).
