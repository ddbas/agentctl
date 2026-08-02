## Context

`rd install <harness>` (in `src/commands/hooks.rs`) supports two harnesses via a
shared `Harness` clap enum defined in `src/main.rs`:

- `pi`: writes a single generated file, `~/.pi/agent/extensions/radr.ts`
  (`install_pi_to`).
- `claude`: patches `~/.claude/settings.json`, merging eight radr-owned hook
  entries into a possibly user-authored JSON document, and migrating pre-rename
  `rd set-state`/`rd clear-state` command strings (`install_claude_to`).

There is currently no removal path. Users must hand-edit these files. We want
`rd uninstall <harness>` to be the mechanical inverse of `install`, reusing the
same `Harness` enum so the two commands can never drift out of sync on which
harnesses are supported.

## Goals / Non-Goals

**Goals:**

- Add `rd uninstall <harness>` accepting the exact same `Harness` values as
  `rd install` by sharing the enum — no separate list to keep in sync.
- `rd uninstall pi` deletes `~/.pi/agent/extensions/radr.ts` if present; no-op
  (exit 0) if absent.
- `rd uninstall claude` removes only the radr-managed hook entries (both current
  `rd state set`/`rd state clear` and recognized pre-rename
  `rd set-state`/`rd clear-state` forms) from `~/.claude/settings.json`, leaving
  all other content (including non-radr hooks under the same event) intact.
  Event arrays that become empty after removal are pruned; the `hooks` key is
  pruned if it becomes empty; the file itself is left in place (not deleted)
  since it may contain other user settings.
- Idempotent: running `uninstall` twice, or on a harness never installed,
  produces no error and no further change on the second run.
- Human-readable summary output mirroring `install`'s `+`/`~`/`=` convention,
  using `-` for "removed" and `=` for "already absent / nothing to do".
- Unit test coverage mirroring the existing `install_pi_to` /
  `install_claude_to` test suites (clean removal, idempotent re-run, no-op when
  absent, preserves non-radr content).

**Non-Goals:**

- No changes to `rd state set` / `rd state clear` semantics or the state file
  format.
- No "uninstall everything" / no-argument form — harness is always required,
  matching `install`'s current UX.
- Not deleting `~/.claude/settings.json` or `~/.pi/agent/extensions/` itself,
  only the radr-owned contents within them.
- No new harnesses are introduced by this change; only structural support so
  future harnesses added to `install` are trivially supported by `uninstall`.

## Decisions

**1. Reuse the existing `Harness` enum verbatim; do not introduce a parallel
type.** `enum Harness { Pi, Claude }` in `src/main.rs` is used as the
`value_enum` for both `Install { harness: Harness }` and the new
`Uninstall { harness: Harness }`. Because both subcommands match on the same
enum, adding a new variant (e.g. `Codex`) forces a compile error in both
`run_install`'s and `run_uninstall`'s `match` unless both are updated (Rust's
exhaustiveness checking on `match` without a wildcard arm). This directly
satisfies the requirement that uninstall support the same harness list as
install, enforced by the compiler rather than by convention/tests alone.

_Alternative considered_: a separate `UninstallHarness` enum. Rejected — purely
duplicative and reintroduces the risk of the two lists silently diverging, which
is the exact problem the proposal calls out.

**2. Colocate uninstall logic in `src/commands/hooks.rs` alongside install.**
The pi and Claude uninstall functions are the direct structural mirror of
`install_pi_to` / `install_claude_to` and share constants
(`PI_EXTENSION_CONTENT` is not needed for pi uninstall, but `RADR_CLAUDE_HOOKS`
and `is_radr_command` are reused directly for Claude uninstall). Keeping them in
the same file avoids duplicating the hook-definition table and the
`is_radr_command` predicate.

_Alternative considered_: a new `src/commands/uninstall.rs` module. Rejected for
now — the amount of shared state (the hook table, the radr-command predicate)
makes a split module mostly re-exports; can be revisited if the file grows too
large.

**3. `rd uninstall pi` deletes the file outright rather than trying to "unwrite"
specific handlers.** Because `radr.ts` is a single fully-generated file owned
entirely by radr (never hand-edited per its own header comment), deletion is
safe and simple. If the file's content doesn't match what radr would generate
(e.g. a user modified it), we still remove it — the header comment on the file
already documents it as generated/managed by radr, so authorial intent is clear.
This matches user expectations of "uninstall removes what install created."

**4. `rd uninstall claude` reuses `is_radr_command` and `RADR_CLAUDE_HOOKS`
event/command pairing to identify entries for removal, not a full-file diff.**
For each event array in `hooks`, remove any array entry whose single hook
command matches `is_radr_command` (covers both current and pre-rename forms),
regardless of whether that exact `(event, command)` pair is in the _current_
`RADR_CLAUDE_HOOKS` table. This ensures uninstall also cleans up
superseded/legacy radr entries (e.g. an old general `PreToolUse` → working hook)
that `install`'s migration logic would otherwise silently remove and replace,
but which `uninstall` should just remove outright. Multi-hook entries (an entry
whose `hooks` array contains a radr command mixed with non-radr commands) are
left untouched — same conservative behavior as `get_single_radr_command` in
`install_claude_to`, since we cannot safely attribute a mixed entry as
radr-owned.

**5. Prune empty structures but never delete the settings file.** After removing
entries, if an event's array becomes empty, remove the event key. If `hooks`
becomes an empty object, remove the `hooks` key. The top-level
`~/.claude/settings.json` file is always preserved (even if it becomes `{}`)
since it's a general Claude settings file the user may add other keys to later,
and deleting it would be surprising/destructive.

**6. Skip writing the file at all if nothing changed (mirrors `install`'s
idempotency-via-mtime approach).** If no radr entries are found,
`uninstall_claude_to` performs no file write, preserving mtime — consistent with
how `install_claude_to` skips the write when `any_changed` is false.

## Risks / Trade-offs

- **[Risk]** A user manually edited `radr.ts` and wants to keep their edits →
  `uninstall pi` deletes it anyway. **Mitigation**: the file's own header ("do
  not edit by hand") sets this expectation; document it in the command's help
  text and CHANGELOG.
- **[Risk]** Removing "superseded" pre-rename Claude entries during uninstall
  could theoretically remove an entry a user added independently that happens to
  match a legacy radr command string exactly. **Mitigation**: the command
  strings (`rd state set ... --harness claude`, `rd set-state ...`,
  `rd clear-state`) are specific enough that an unrelated hook coincidentally
  using them is implausible; this mirrors the exact recognition logic `install`
  already trusts for migration.
- **[Risk]** Future harnesses added only to `install`'s `match` with a wildcard
  arm (`_ => ...`) would silently bypass the compiler's exhaustiveness guarantee
  for `uninstall`. **Mitigation**: explicitly avoid wildcard arms in both
  `run_install` and `run_uninstall`; call this out in a code comment on the
  `Harness` enum.

## Migration Plan

Purely additive CLI surface (`rd uninstall <harness>`); no data migration, no
changes to existing `install`/`state` behavior, no rollback concerns beyond
normal code review and the existing test suite (`cargo test` via `mise`).
