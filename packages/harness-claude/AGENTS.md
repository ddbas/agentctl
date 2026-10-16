# @agentctl/harness-claude

Holds the sole implementation of the `Harness` port for Claude Code: installing
and uninstalling the agentctl plugin under `~/.claude/skills/agentctl/`, and the
plugin manifest and hooks assets it embeds via `?raw` from `src/assets/`. It
must not contain code for any other harness, nor state persistence, tmux or
business logic.
