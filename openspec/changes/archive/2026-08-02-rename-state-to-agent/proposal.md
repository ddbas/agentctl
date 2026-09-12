## Why

`agentctl` mixes two distinct audiences under a flat subcommand list: commands a
human/orchestrator uses to observe and control agents (`list`, and future
`tail`/`send`), and commands an agent harness invokes about itself to report
status (`state set`/`state clear`, and future activity logging). Today only
`state` hints at the agent-facing side, but it sits at the same top level as
`list`, so the CLI's structure doesn't communicate which commands are meant for
humans versus which are meant to be called from hooks/extensions. Additionally,
`state set`/`state clear` read as bare verbs with no object — that's tolerable
today because `state` is the only agent-facing group, but it breaks down once a
second agent-facing command (activity logging) sits alongside it. We want a
single, obvious namespace for "things an agent says about itself" that reads
clearly and is easy to extend without further top-level clutter.

## What Changes

- **BREAKING**: Rename the `agentctl state` subcommand namespace to
  `agentctl agent`, and reshape it into three flat sibling commands rather than
  a `set`/`clear` pair:
  - `agentctl state set <state> [--harness <h>] [--pane <id>]` becomes
    `agentctl agent status <state> [--harness <h>] [--pane <id>]`. `status`
    takes the state value directly as a positional argument — it has exactly one
    operation (record the current status), so no nested verb is needed.
  - `agentctl state clear [--pane <id>]` becomes
    `agentctl agent clear [--pane <id>]`. `clear` is **whole-session cleanup**:
    it removes the status file today, and is the designated place to remove any
    other per-agent artifacts (e.g. a future activity log) introduced later — it
    is a sibling of `status`, not an operation nested under it.
  - `agentctl agent log <text>` is reserved as the future home for agent
    activity logging (not implemented in this change) — establishing the sibling
    shape now (`status` / `log` / `clear`) avoids a second breaking rename
    later.
- Update the pi extension (`agentctl install pi`) so newly-installed/reinstalled
  extensions call `agentctl agent status <state>` / `agentctl agent clear`
  instead of `agentctl state set` / `agentctl state clear`.
- Update the Claude hook definitions (`agentctl install claude`) so
  newly-registered hook commands use `agentctl agent status <state>` /
  `agentctl agent clear`.
- Extend `agentctl install`/`agentctl uninstall` migration logic so pre-existing
  `agentctl state set` / `agentctl state clear` command strings (including the
  older pre-rename `agentctl set-state`/`agentctl clear-state` forms already
  migrated) are recognized and rewritten to `agentctl agent status` /
  `agentctl agent clear` on the next install run, and recognized for removal on
  uninstall.
- No change to `agentctl list`, `agentctl install`, or `agentctl uninstall` —
  they remain top-level, human/orchestrator-facing commands.
- Establish `agentctl agent` as the namespace for future agent-self-reporting
  commands, with `status` / `log` / `clear` as flat siblings, keeping the CLI
  split clean as it grows.

## Capabilities

### New Capabilities

(none — this restructures existing capabilities rather than introducing new
behavior)

### Modified Capabilities

- `claude-state-hooks`: the eight Claude hook entries SHALL invoke
  `agentctl agent status <state> ...` / `agentctl agent clear` instead of
  `agentctl state set ...` / `agentctl state clear`, and the install/uninstall
  migration logic SHALL recognize the prior
  `agentctl state set`/`agentctl state clear` command strings as agentctl-owned
  for upgrade and removal purposes.
- `pi-state-hooks`: the pi extension's lifecycle handlers SHALL invoke
  `agentctl agent status <state> ...` / `agentctl agent clear` instead of
  `agentctl state set ...` / `agentctl state clear`.

## Impact

- `src/main.rs`: rename `Commands::State` → `Commands::Agent`; replace
  `StateSubcommand::{Set, Clear}` with `AgentSubcommand::{Status, Clear}` (and
  reserve a future `Log` variant), where `Status` takes the state value as a
  positional argument rather than a nested `set` subcommand.
- `src/commands/set_state.rs`, `src/commands/clear_state.rs`: no internal logic
  changes expected, only the CLI wiring that invokes them (may be renamed to
  `commands/agent_status.rs`/`commands/agent_clear.rs` for clarity, see
  design.md).
- `src/commands/hooks.rs`: update `PI_EXTENSION_CONTENT`,
  `AGENTCTL_CLAUDE_HOOKS` command strings, and the `is_agentctl_command` /
  migration recognition logic to cover `agentctl state set ...` /
  `agentctl state clear` (previous form) and `agentctl agent status ...` /
  `agentctl agent clear` (new form).
- Tests: `tests/state_commands.rs`, `tests/hooks.rs`, `tests/help_command.rs`,
  and hook-related unit tests in `src/commands/hooks.rs` need updated
  expectations for the new command strings and CLI help text.
- Any external documentation or user-facing help text referencing
  `agentctl state set`/`agentctl state clear` needs updating.
- This is a breaking CLI change for anyone who has already run
  `agentctl install pi`/`agentctl install claude` or scripted
  `agentctl state ...` directly; the migration logic in `agentctl install`
  mitigates this for hook-registered usages, but manual scripts calling
  `agentctl state ...` will need updating.
