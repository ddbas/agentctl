# radr

This project implements a **tmux-based agent multiplexer**.

## Required One-Time Setup for Pi Agents

If you are running inside pi, install the radr lifecycle hooks so your agent
state is tracked in real time:

```bash
rd install pi
```

This writes `~/.pi/agent/extensions/radr.ts`, a TypeScript extension that
subscribes to pi lifecycle events (`session_start`, `before_agent_start`,
`agent_settled`, `session_shutdown`) and calls `rd state set` / `rd state clear`
automatically. The command is idempotent — re-running it is safe.

## Required One-Time Setup for Claude Agents

If you are running inside Claude Code, install the radr lifecycle hooks so your
agent state is tracked in real time:

```bash
rd install claude
```

This patches `~/.claude/settings.json` to register six hook entries that call
`rd state set` / `rd state clear` at key Claude lifecycle events
(`SessionStart`, `UserPromptSubmit`, `PreToolUse`, `Stop`, `Notification`,
`SessionEnd`). The command is idempotent — re-running it is safe.

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
