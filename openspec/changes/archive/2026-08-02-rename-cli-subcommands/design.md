## Context

`agentctl` is a small `clap`-based CLI (see `src/main.rs`) with a flat set of
top-level commands: `list`, `set-state`, `clear-state`, and
`hooks install <harness>`. `src/commands/hooks.rs` embeds the pi extension
TypeScript template and the Claude `settings.json` hook definitions, both of
which invoke `agentctl set-state ...` / `agentctl clear-state` as literal
command strings baked into generated files
(`~/.pi/agent/extensions/agentctl.ts`, `~/.claude/settings.json`). `hooks.rs`
also contains migration logic (`is_agentctl_command`,
`get_single_agentctl_command`, the cleanup pass in `install_claude_to`) that
detects and upgrades previously-installed agentctl hook entries when the command
definitions change — this same mechanism must now handle the rename itself.

This change is purely a CLI surface reorganization: no state file schema,
`AgentState` enum, or `agentctl list` behavior changes.

## Goals / Non-Goals

**Goals:**

- Replace `agentctl hooks install <harness>` with `agentctl install <harness>`,
  and broaden its help text to describe installing agentctl generally on a
  harness (not just hooks).
- Replace `agentctl set-state` / `agentctl clear-state` with
  `agentctl state set` / `agentctl state clear` subcommands under a new
  `agentctl state` group.
- Keep all existing flags (`--harness`, `--pane`), state semantics, and file
  formats unchanged.
- Ensure users who already ran the old `agentctl hooks install claude` get their
  Claude `settings.json` hook commands transparently upgraded to the new
  `agentctl state set` / `agentctl state clear` strings the next time they run
  `agentctl install claude`.
- Ensure users who already have the old pi extension get it overwritten with the
  new command strings on next `agentctl install pi` (this already happens today
  via content-diff `FileStatus::Updated` — no special-casing needed there since
  the whole file is regenerated).

**Non-Goals:**

- No new install targets beyond hooks (the "install other things" possibility is
  just the motivating rationale for the rename, not implemented now).
- No new `agentctl state` subcommands beyond `set`/`clear` (e.g. no
  `agentctl state show`).
- No changes to `agent-state`, `list-sessions`, or `list-sorting` capabilities.

## Decisions

### 1. `agentctl install` is a top-level command, not `agentctl install hooks`

The proposal explicitly asks for `agentctl hooks install` → `agentctl install`,
i.e. collapsing the `hooks` group away rather than renaming it to something
else. `install` becomes a direct subcommand of `agentctl` taking a `harness`
argument, matching today's `Install { harness }` shape but without the `Hooks`
wrapper enum. The `HooksSubcommand` enum and `Commands::Hooks` variant are
removed from `src/main.rs`.

Alternative considered: keep `agentctl install hooks <harness>` as a subgroup to
leave room for `agentctl install <other-thing>`. Rejected — the proposal asks
for `agentctl install <harness>` directly, and `Harness` (pi/claude) is already
the natural discriminator; if agentctl needs to install non-hook things later it
can add a separate flag or command rather than nesting further.

### 2. `agentctl state` is a new command group wrapping `set`/`clear`

Add a `Commands::State { subcommand: StateSubcommand }` variant, with
`StateSubcommand::Set { state, harness, pane }` and
`StateSubcommand::Clear { pane }` mirroring today's `SetState`/`ClearState`
variants exactly (same field names/types), just nested one level deeper. This is
the smallest change that satisfies `agentctl state set` / `agentctl state clear`
and mirrors the existing `Hooks { subcommand: HooksSubcommand }` pattern already
used in the codebase, so it's a familiar shape for `clap`.

Alternative considered: rename the existing flat variants in place with
`#[command(name = "state-set")]` etc. Rejected — doesn't produce the
`agentctl state set` two-word subcommand syntax the user asked for; clap
subcommand nesting is the correct mechanism for that.

### 3. Command-string constants in `hooks.rs` are updated in place; migration keys off "starts with `agentctl state`"

`PI_EXTENSION_CONTENT`, `AGENTCTL_CLAUDE_HOOKS[].command`, and
`is_agentctl_command` are updated to use `agentctl state set ...` /
`agentctl state clear`. Since `PI_EXTENSION_CONTENT` is a single embedded string
diffed wholesale (`FileStatus::Updated` when content differs), the pi side needs
no extra migration logic — regenerating the whole file already handles upgrading
old installs.

For Claude, `is_agentctl_command` currently recognizes `agentctl set-state ` /
`agentctl clear-state` as "this is a agentctl-managed command" so the cleanup
pass can detect and remove/update stale entries. It must now also recognize the
_old_ command forms so that a settings.json written by a pre-rename `agentctl`
binary is correctly identified as agentctl-managed and cleaned up (the existing
"superseded command" removal path already handles commands present in an old
install but no longer in `AGENTCTL_CLAUDE_HOOKS`, since after the rename none of
the old-style `agentctl set-state ...` strings appear in the new
`AGENTCTL_CLAUDE_HOOKS` table — they get treated as superseded and removed, then
the new `agentctl state set ...` entries get added by the normal add-or-skip
pass). So: `is_agentctl_command` is updated to match on _either_ the old
prefixes (`agentctl set-state `, `agentctl clear-state`) _or_ the new ones
(`agentctl state set `, `agentctl state clear`), ensuring both older and newer
existing installs are recognized and cleaned up correctly across repeated
upgrades.

Alternative considered: bump a version marker in the settings file to force a
one-time hard reset. Rejected — the existing superseded-entry removal mechanism
already generalizes to this case with a one-line change to
`is_agentctl_command`, no new state needed.

### 4. Help text for `install`

Update the doc comment on the `Install` variant (and any prose in `hooks.rs`)
from being hook-specific ("Install hooks for a harness...") to framing it as
"Install agentctl on a harness" with a note that today this means installing
hook integrations, but may include more in the future. Module-level doc comment
in `hooks.rs` (`//! agentctl hooks — hook registration...`) is updated similarly
since the module still implements installation, just no longer under a `hooks`
CLI namespace.

## Risks / Trade-offs

- **[Risk]** Existing users' shell aliases/scripts calling
  `agentctl hooks install`, `agentctl set-state`, or `agentctl clear-state` will
  break (this is an intentional **BREAKING** change per the proposal). →
  **Mitigation**: none needed within the CLI itself (no deprecation aliases
  requested); call out clearly in the proposal/changelog. Could add hidden
  `#[command(alias = ...)]` compatibility aliases later if real users are
  affected, but not in scope now.
- **[Risk]** Missing an old-command-string variant in `is_agentctl_command`
  could leave stale entries un-migrated in `~/.claude/settings.json`. →
  **Mitigation**: cover with a dedicated migration test (pre-populate
  settings.json with old-style `agentctl set-state`/`agentctl clear-state`
  commands, run `install_claude_to`, assert the old entries are removed and new
  `agentctl state set`/`agentctl state clear` entries are present) mirroring the
  existing stale-matcher migration test.
- **[Risk]** Renaming module files (`set_state.rs`/`clear_state.rs`) purely for
  naming consistency could create a larger diff than necessary. →
  **Mitigation**: keep the existing file names/module paths as-is; only
  `main.rs` wiring and `hooks.rs` content strings need to change. Renaming
  modules is optional/cosmetic and can be skipped to minimize risk.

## Migration Plan

1. Update `src/main.rs` CLI definitions (`Install`, `State`/`StateSubcommand`),
   removing `Hooks`/`HooksSubcommand`/`SetState`/`ClearState`.
2. Update `src/commands/hooks.rs`: embedded pi extension content, Claude hook
   command strings, `is_agentctl_command`, doc comments, and any printed help/
   summary text that references old command names.
3. Update existing unit tests in `src/commands/hooks.rs` and integration tests
   in `tests/hooks.rs` / `tests/state_commands.rs` to assert the new command
   names, and add a migration test for old→new Claude hook command upgrade.
4. Run the full test suite via `mise run test` (or the project's configured test
   task) to confirm nothing else references the old command names.
5. No rollback complexity beyond reverting the commit — no persisted state
   format changes, so rollback is a plain code revert.

## Open Questions

- Should `agentctl install` / `agentctl state` ship hidden aliases for the old
  command names during a transition period? Not requested by the user; assuming
  no for now, but flagging in case backward compatibility becomes a concern.
