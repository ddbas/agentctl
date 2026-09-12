## 1. CLI surface

- [x] 1.1 Add `Uninstall { harness: Harness }` variant to the `Commands` enum in
      `src/main.rs`, reusing the existing `Harness` value enum, with doc
      comments describing behavior per harness.
- [x] 1.2 Wire `Commands::Uninstall { harness }` in `run()` to a new
      `commands::hooks::run_uninstall(harness)` entry point.
- [x] 1.3 Ensure `match harness { ... }` in both `run_install` and
      `run_uninstall` has no wildcard (`_`) arm, so adding a future `Harness`
      variant fails to compile until both are updated; add a code comment on the
      `Harness` enum documenting this invariant.

## 2. Pi uninstall

- [x] 2.1 Implement `run_uninstall` dispatch for `Harness::Pi` calling a new
      `uninstall_pi()` (default path) which delegates to a testable
      `uninstall_pi_to(&Path)`.
- [x] 2.2 `uninstall_pi_to`: delete the file if it exists; if absent, no-op.
      Never remove the parent `extensions/` directory.
- [x] 2.3 Print a summary line indicating removal (`-`) or nothing-to-do (`=`),
      following the style of `print_pi_summary`.
- [x] 2.4 Unit test: uninstall removes an existing `agentctl.ts` file.
- [x] 2.5 Unit test: uninstall is a no-op (exit 0, no error) when the file does
      not exist.
- [x] 2.6 Unit test: uninstall removes the file even when its content differs
      from `PI_EXTENSION_CONTENT` (hand-modified or stale).
- [x] 2.7 Unit test: uninstall leaves the `extensions/` directory in place after
      removing the only file in it.

## 3. Claude uninstall

- [x] 3.1 Implement `run_uninstall` dispatch for `Harness::Claude` calling a new
      `uninstall_claude()` (default path) which delegates to a testable
      `uninstall_claude_to(&Path)`.
- [x] 3.2 `uninstall_claude_to`: read and parse `~/.claude/settings.json` (if
      absent, treat as nothing to remove and exit 0 without writing).
- [x] 3.3 For each event array under `hooks`, remove any entry whose single hook
      command matches `is_agentctl_command` (reuse existing predicate, covering
      both current and pre-rename command forms); leave multi-command entries
      untouched.
- [x] 3.4 Prune event keys whose array becomes empty; prune the `hooks` key if
      it becomes an empty object. Never delete the settings file itself.
- [x] 3.5 Skip the file write entirely (preserve mtime) when no agentctl entries
      were found, mirroring `install_claude_to`'s idempotency approach.
- [x] 3.6 Print a summary listing each removed `(event, command)` pair (prefixed
      `-`), or a nothing-to-remove message (prefixed `=`) when the set is empty.
- [x] 3.7 Unit test: uninstall removes all eight agentctl-managed entries from a
      freshly-installed settings file.
- [x] 3.8 Unit test: uninstall preserves a non-agentctl hook entry under the
      same event as a agentctl entry.
- [x] 3.9 Unit test: uninstall recognizes and removes pre-rename
      `agentctl set-state ...` / `agentctl clear-state` legacy entries.
- [x] 3.10 Unit test: uninstall prunes an event key that becomes empty and
      prunes the `hooks` key entirely when no events remain.
- [x] 3.11 Unit test: uninstall is a no-op (no write, mtime unchanged) when
      `~/.claude/settings.json` has no agentctl entries.
- [x] 3.12 Unit test: uninstall is a no-op when `~/.claude/settings.json` does
      not exist at all.
- [x] 3.13 Unit test: running uninstall twice in a row produces no further
      change on the second run (idempotency).
- [x] 3.14 Unit test: uninstall does not modify a hook entry whose `hooks` array
      mixes a agentctl command with a non-agentctl command.

## 4. Docs and polish

- [x] 4.1 Update `src/commands/hooks.rs` module doc comment (or split into a
      dedicated module doc) to describe `uninstall` alongside `install`.
- [x] 4.2 Run `mise run` equivalent lint/test/format tasks (e.g.
      `mise run test`, `mise run lint`) and fix any issues.
- [x] 4.3 Manually verify `agentctl uninstall pi` and
      `agentctl uninstall claude --help` output reads clearly via
      `agentctl uninstall --help`.
