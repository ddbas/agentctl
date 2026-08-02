## Why

The current `rd hooks install <harness>` name suggests the top-level command is
only about "hooks," but installing hooks is really the first case of a broader
"install radr onto a harness" concept — future harnesses may need more than
hooks installed. Separately, `rd set-state` / `rd clear-state` are flat,
unrelated-looking commands even though they both operate on the same state
resource. Grouping them under `rd state set` / `rd state clear` makes the CLI's
command tree clearer and leaves room to grow (e.g. `rd state show` later)
without renaming anything again.

## What Changes

- **BREAKING**: Rename `rd hooks install <harness>` to `rd install <harness>`.
  The `hooks` subcommand is removed; `install` becomes a top-level command.
- Update the `rd install` command/help description to describe it generically as
  "installing radr on a harness" (currently hook-specific wording), while
  keeping today's behavior (writing hook files) as its only implementation.
- **BREAKING**: Rename `rd set-state <state>` to `rd state set <state>`, and
  `rd clear-state` to `rd state clear`. Both become subcommands of a new
  `rd state` command group; flags (`--harness`, `--pane`) and behavior are
  unchanged.
- Update the generated pi extension (`~/.pi/agent/extensions/radr.ts`) and the
  Claude hook entries registered by `rd install` to invoke the new command names
  (`rd state set ...`, `rd state clear`).
- Update the existing-hooks migration/idempotency detection (in
  `src/commands/hooks.rs`) so it recognizes both old- and new-style commands
  when deciding whether an installed hook is "radr's" and needs updating vs.
  left alone (so users on older installs get cleanly upgraded to the new command
  names on re-install).
- Update all documentation, help text, and comments referencing the old command
  names.

## Capabilities

### New Capabilities

(none — this is a rename/reorganization of existing commands, no new behavior)

### Modified Capabilities

- `pi-state-hooks`: Requirements referencing `rd set-state`, `rd clear-state`,
  and `rd hooks install pi` are updated to reference `rd state set`,
  `rd state clear`, and `rd install pi` instead. Generated extension content
  changes accordingly.
- `claude-state-hooks`: Requirements referencing `rd set-state`,
  `rd clear-state`, and `rd hooks install claude` are updated to reference
  `rd state set`, `rd state clear`, and `rd install claude` instead. Generated
  hook command strings change accordingly.

## Impact

- `src/main.rs`: CLI arg parsing — remove `SetState`/`ClearState`/`Hooks` top
  level variants and `HooksSubcommand::Install`; add `Install` top-level command
  and a `State` command group with `Set`/`Clear` subcommands.
- `src/commands/set_state.rs`, `src/commands/clear_state.rs`: entry points
  invoked from the new `rd state set` / `rd state clear` wiring (may be
  renamed/moved under a `commands/state/` module for clarity).
- `src/commands/hooks.rs`: generated pi extension template and Claude hook entry
  command strings; the "is this a radr-managed hook" detection logic; install
  install/upgrade CLI messaging.
- `tests/hooks.rs`, `tests/state_commands.rs`: update to exercise/assert the new
  command names.
- No changes to the state file schema, `AgentState` enum, or `rd list` behavior.
