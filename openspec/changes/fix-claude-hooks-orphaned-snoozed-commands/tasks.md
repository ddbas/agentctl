## 1. Widen radr-command recognition

- [x] 1.1 In `src/commands/hooks.rs`, add `cmd.starts_with("rd agent watch ")`
      and `cmd.starts_with("rd agent stop")` to `is_radr_command`.
- [x] 1.2 Update `is_radr_command`'s doc comment to describe the new forms:
      leaked from a build of `rd` implementing the never-merged
      `add-snoozed-agent-state` change (`new-agent-status-snoozed` branch), with
      no corresponding entry in `RADR_CLAUDE_HOOKS` — recognized purely so the
      existing superseded-entry-removal pass can clean them up.

## 2. Tests

- [x] 2.1 Add a unit test seeding a settings.json with the four orphaned entries
      (`PreToolUse`/`Monitor` → `rd agent watch arm --harness     claude`,
      `PostToolUse`/`Monitor` → `rd agent watch resolve --harness     claude`,
      `Stop` → `rd agent stop --harness claude`, `StopFailure` →
      `rd agent stop --harness claude`) alongside a non-radr hook entry, and
      asserting `install_claude_to` removes all four, leaves the non-radr entry
      untouched, and adds/updates the nine current entries as usual.
- [x] 2.2 Add a unit test asserting `format_claude_summary`'s removed-entries
      list includes all four orphaned entries when passed to
      `install_claude_to`.
- [x] 2.3 Add a unit test seeding the same four orphaned entries and asserting
      `uninstall_claude_to` removes them (mirroring the existing
      `uninstall_claude_*` legacy-form tests).
- [x] 2.4 Add a regression test asserting `is_radr_command` returns `true` for
      both `rd agent watch arm --harness claude` and
      `rd agent watch     resolve --harness claude` (covering both `watch`
      subcommands via the shared prefix) and for
      `rd agent stop --harness claude`.

## 3. Documentation

- [x] 3.1 Update `openspec/specs/claude-state-hooks/spec.md` per the delta spec
      in this change.

## 4. Verify

- [x] 4.1 Run `mise run build` and `mise run test` (or equivalent
      `cargo     build`/`cargo test` via `mise exec`), confirming no
      regressions.
- [x] 4.2 Run `rd install claude` against a copy of the real
      `~/.claude/settings.json` (or a fixture reproducing its orphaned entries)
      and confirm the four orphaned entries are removed and reported in the
      summary, while all other content (including non-radr keys like
      `enabledPlugins`, `permissions`, custom hooks such as `rtk hook claude`
      and `~/.claude/hooks/notify.sh`) is left untouched.
