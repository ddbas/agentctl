## Context

`agentctl` currently has a single, flat `Commands` enum in `src/main.rs` mixing
two audiences: human/orchestrator-facing commands (`list`, `install`,
`uninstall`) and the agent-self-reporting namespace (`state set`,
`state clear`), which is invoked from generated hook integrations
(`agentctl install pi`/`agentctl install claude`) rather than typed by a person.
As more agent-self-reporting commands are added (e.g. activity logs), the flat
structure would keep adding top-level nouns that don't obviously belong with
`list`/`install`/`uninstall`. This is the second rename in this namespace's
history: `agentctl set-state`/`agentctl clear-state` were previously renamed to
`agentctl state set`/`agentctl state clear`, and `hooks.rs` already contains
migration logic recognizing and rewriting that older form. This change adds one
more generation on top of that existing migration mechanism. It also reshapes
the namespace itself: `set`/`clear` become `status`/`clear` (with `log`
reserved), because `set`/`clear` read as bare verbs with no object once a second
agent-facing command exists alongside them, and because `clear`'s actual scope
(whole-session cleanup, not just "clear the status") is clearer when it's a
sibling command rather than a suffix on `state`/`status`.

## Goals / Non-Goals

**Goals:**

- Rename the `state` subcommand namespace to `agent` and reshape it into three
  flat siblings — `agentctl agent status <state>`, `agentctl agent log <text>`
  (reserved, not implemented here), `agentctl agent clear` — with no behavioral
  change to the underlying set/clear logic, arguments, or state file format.
  `status` takes the state value as a positional argument rather than a nested
  `set` verb, since it has exactly one operation; `clear` is whole-session
  cleanup (status file today, any future per-agent artifacts going forward), so
  it is a sibling of `status`/`log`, not nested under `status`.
- Update `agentctl install pi` and `agentctl install claude` to
  generate/register hook commands using
  `agentctl agent status <state>`/`agentctl agent clear`.
- Extend the existing hook migration/recognition logic in `hooks.rs` so it
  treats `agentctl state set`/`agentctl state clear` (the previous generation)
  the same way it already treats `agentctl set-state`/`agentctl clear-state`
  (the generation before that): recognized for uninstall, and rewritten in place
  on `agentctl install`.
- Keep `agentctl list`, `agentctl install`, `agentctl uninstall` as top-level
  commands, unaffected in shape.

**Non-Goals:**

- No new agent-facing commands (e.g. logging) are introduced by this change — it
  only prepares the namespace (`agentctl agent ...`) for them.
- No change to the `AgentState` enum, state file schema, or `read_state`/
  `delete_state`/stale-validation logic in `src/state.rs` — none of that code
  path references the CLI subcommand name.
- No change to `agentctl list` output, sorting, or orchestrator-facing behavior.

## Decisions

### 1. Rename at the clap enum level: `Commands::State` → `Commands::Agent`, `set`/`clear` → `status`/`clear`

`main.rs` defines `Commands::State { subcommand: StateSubcommand }` with
`StateSubcommand::{Set, Clear}`. Rename to
`Commands::Agent { subcommand: AgentSubcommand }` with
`AgentSubcommand::{Status, Clear}` (leaving room for a future
`AgentSubcommand::Log`). The doc comment `/// Manage agent state.` becomes the
help text for `agentctl agent`.

`Status` takes the state value as a **positional argument**, not a nested `set`
subcommand — `agentctl agent status idle` rather than
`agentctl agent status set idle` — because `status` has exactly one operation
today. `Clear` keeps its current shape (`--pane` override, no positional value)
but its doc comment is updated to describe it as whole-session cleanup rather
than "clear the state," since it is the sibling responsible for removing all
per-agent artifacts, not just the status file. No change to the underlying field
types (`state`, `harness`, `pane`) or to `commands::set_state::run` /
`commands::clear_state::run`'s internal logic — only the dispatch arm in `run()`
and the clap arg shape (nested subcommand → positional) need updating.

**Alternative considered:** keep the `status set <state>` shape (nested verb)
for symmetry with a possible future second `status` operation (e.g.
`status show`). Rejected as premature: there is no concrete second operation
planned, and `agentctl agent status idle` is shorter and reads naturally today;
if a second operation is ever needed, `status` can grow a nested subcommand at
that point without breaking `agentctl agent status <state>` becoming ambiguous,
since clap can distinguish a known subcommand name from an arbitrary state value
as long as state values never collide with subcommand names
(`idle`/`working`/`done`/ `blocked` vs. e.g. `show`).

**Alternative considered:** keep `commands/set_state.rs` and
`commands/clear_state.rs` file names as-is rather than renaming to
`commands/agent_status.rs`/`commands/agent_clear.rs`. Chosen to rename them to
`agent_status.rs`/`agent_clear.rs` this time (unlike a pure `set`→`status` name
swap, which would still leave the old name of the underlying function mismatched
with the new CLI verb) — see design.md task breakdown. If a future
`agentctl agent log` command is added, it gets its own new module (e.g.
`commands/agent_log.rs`) rather than retrofitting these.

### 2. Three-generation migration chain in `hooks.rs`

`hooks.rs` already has a notion of "is this string a agentctl-managed command,
possibly in an old form" via
`is_agentctl_command`/`get_single_agentctl_command`, used by both the install
migration pass and the uninstall removal pass. That recognition currently covers
two forms per logical command: the current
`agentctl state set ...`/`agentctl state clear` and the legacy
`agentctl set-state ...`/`agentctl clear-state`. This change adds a third form
on top: `agentctl agent status ...`/`agentctl agent clear` (new canonical)
recognizes and supersedes both older forms. The recognition set becomes, for
"record state X": `agentctl agent status X --harness <h>` (current),
`agentctl state set X --harness <h>` (superseded),
`agentctl set-state X --harness <h>` (superseded); for "clear":
`agentctl agent clear` (current), `agentctl state clear` (superseded),
`agentctl clear-state` (superseded).

`AGENTCTL_CLAUDE_HOOKS` command strings and `PI_EXTENSION_CONTENT` are updated
to emit the new `agentctl agent status ...`/`agentctl agent clear` forms. The
install migration pass (which already diffs against `AGENTCTL_CLAUDE_HOOKS`/
`PI_EXTENSION_CONTENT` and removes/replaces superseded entries) picks up the new
generation automatically once the recognition set and the generated content are
both updated — no new migration-specific code path is needed beyond widening
what's recognized as "agentctl, but stale."

**Alternative considered:** a dedicated one-time `agentctl migrate` command that
users run explicitly. Rejected because the existing pattern (recognize + rewrite
on the next `agentctl install`) already works for the prior rename and requires
no extra user action — hooks self-heal the next time
`agentctl install <harness>` runs, which happens automatically whenever
`agentctl` is upgraded and reinstalled, or can be documented as a manual step.

### 3. No compatibility alias for `agentctl state ...` as a direct CLI invocation

`agentctl state set idle` typed directly (not via a hook) will simply become an
unrecognized subcommand after this change, rather than being kept as a
deprecated alias. Chosen for simplicity: the primary callers of
`agentctl state ...` are hook-generated command strings (covered by the
migration path above), not humans typing it interactively. Anyone with scripts
hard-coding `agentctl state ...` needs to update them, called out as a breaking
change in the proposal.

**Alternative considered:** add `#[command(alias = "state")]` on the `Agent`
variant so `agentctl state set ...` keeps working. Rejected because it would
leave the CLI surface permanently cluttered with a legacy name, undermining the
motivation for the rename (a clean, purpose-signaling namespace), and it isn't
needed for the hook self-healing path since that goes through
`agentctl install`, not direct invocation.

## Risks / Trade-offs

- **[Risk]** Anyone with a shell alias, script, or muscle memory typing
  `agentctl state set ...` directly will get a CLI parse error after upgrading.
  → **Mitigation**: called out explicitly as a breaking change in the proposal;
  `agentctl --help`/`agentctl agent --help` will surface the new name
  immediately.
- **[Risk]** Widening `is_agentctl_command` recognition to three generations
  increases the chance of a false-positive match against a user's own unrelated
  hook command that happens to look similar. → **Mitigation**: existing tests
  already assert exact-string matching per generation; the same precision is
  preserved when adding the third generation, and multi-command hook entries are
  already left untouched regardless of generation (see "Uninstall does not touch
  multi-command hook entries").
- **[Trade-off]** Not aliasing `agentctl state` means a one-time coordinated
  release: users must re-run `agentctl install <harness>` (or reinstall) to pick
  up hooks using the new command name; until they do, their existing installed
  hooks keep working (they still call the old, now-unrecognized-
  by-humans-but-still-valid-binary-if-present `agentctl state set`/`clear` —
  this only becomes a problem if the old subcommand is fully removed from the
  binary, which this change does). → **Mitigation**: this is the same trade-off
  already accepted for the `set-state`→`state set` rename; documented in release
  notes and the migration scenarios cover the upgrade path once
  `agentctl install` is re-run.

## Migration Plan

1. Update `src/main.rs`: rename `Commands::State`/`StateSubcommand` to
   `Commands::Agent`/`AgentSubcommand`; replace `Set`/`Clear` variants with
   `Status` (positional state value) / `Clear`, updating the `run()` match arms
   and any doc comments describing `Clear` as whole-session cleanup.
2. Optionally rename `src/commands/set_state.rs` / `src/commands/clear_state.rs`
   to `src/commands/agent_status.rs` / `src/commands/agent_clear.rs` for naming
   consistency with the new CLI verbs (internal logic unchanged).
3. Update `src/commands/hooks.rs`: update `AGENTCTL_CLAUDE_HOOKS` command
   strings and `PI_EXTENSION_CONTENT` to use
   `agentctl agent status`/`agentctl agent clear`; widen
   `is_agentctl_command`/`get_single_agentctl_command` (or equivalent
   recognition logic) to recognize `agentctl state set`/`agentctl state clear`
   as a superseded, migratable/removable form alongside the existing
   `agentctl set-state`/`agentctl clear-state` recognition.
4. Update tests: `tests/state_commands.rs`, `tests/hooks.rs`,
   `tests/help_command.rs`, and the unit tests inside `src/commands/hooks.rs` to
   expect `agentctl agent status ...`/`agentctl agent clear` in generated
   content/help text, and add migration test cases for the `agentctl state ...`
   → `agentctl agent status ...`/`agentctl agent clear` upgrade path (mirroring
   the existing `agentctl set-state` → `agentctl state set` migration tests).
5. Run the full test suite (`mise run test` or project equivalent) to confirm no
   regressions.
6. No data migration is needed — the state file schema and on-disk format are
   untouched by this change.

Rollback: revert the commit(s); since this is a pure CLI/wiring rename with no
persisted-format changes, rollback is a straightforward code revert with no data
cleanup required.

## Open Questions

- Should `agentctl agent` eventually gain a `--help`-visible note pointing users
  at the renamed command if they type the old `state` subcommand name, or is a
  plain "unrecognized subcommand" clap error sufficient? (Leaning: plain error
  is fine, consistent with Decision 3.)
