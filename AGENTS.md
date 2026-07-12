# drovr

This project implements a **tmux-based agent multiplexer**.

## Running Tasks

Always invoke tasks and CLIs via `mise` — never call `cargo`, `npm`, or other
tools directly unless the tool is not available through mise.

```bash
mise list                  # list tools managed by mise
mise exec <tool> -- <cmd>  # run a tool through mise
mise tasks                 # list all available tasks
mise run <task>            # run a task
```

## Testing

All new features MUST include unit and/or integration tests.

All E2E and integration tests MUST use
[testcontainers](https://crates.io/crates/testcontainers) so they are fully
isolated from the host environment (filesystem, git config, hooks, env vars,
etc.). See `openspec/specs/e2e-test-infrastructure/spec.md` for details.
