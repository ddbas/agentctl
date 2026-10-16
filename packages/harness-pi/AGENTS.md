# @agentctl/harness-pi

Holds the sole implementation of the `Harness` port for the pi agent: installing
and uninstalling the agentctl extension at `~/.pi/agent/extensions/agentctl.ts`,
and the extension source it embeds via `?raw` from `src/assets/`. It must not
contain code for any other harness, nor state persistence, tmux or business
logic.
