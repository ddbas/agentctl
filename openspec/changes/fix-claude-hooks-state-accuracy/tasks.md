## 1. Update DROVR_CLAUDE_HOOKS constant

- [ ] 1.1 Change `SessionStart` matcher from `"startup"` to
      `"startup|resume|clear"` in `DROVR_CLAUDE_HOOKS`
- [ ] 1.2 Remove the general `PreToolUse` (no matcher) → `dv set-state working`
      entry
- [ ] 1.3 Add `PostToolUse` (no matcher) →
      `dv set-state working --harness claude` [async] entry
- [ ] 1.4 Add `PreToolUse` with `matcher: "AskUserQuestion|ExitPlanMode"` →
      `dv set-state blocked --harness claude` [async] entry
- [ ] 1.5 Add `StopFailure` (no matcher) → `dv set-state done --harness claude`
      [async] entry

## 2. Update install summary output

- [ ] 2.1 Update `format_claude_summary` (and `print_*` output) to reflect the
      new 8-entry hook set (was 6)

## 3. Update unit tests

- [ ] 3.1 Update `install_claude_clean_file_writes_all_six_entries` test: rename
      to `..._all_eight_entries` and assert all 8 events are present
      (`SessionStart`, `UserPromptSubmit`, `PostToolUse`, `PreToolUse`, `Stop`,
      `StopFailure`, `Notification`, `SessionEnd`)
- [ ] 3.2 Update `install_claude_idempotent_no_duplicates` test to assert 8
      event keys with no duplicates
- [ ] 3.3 Update `install_claude_async_flags_are_correct` test: add
      `PostToolUse`, `PreToolUse` (AskUserQuestion), `StopFailure` to the
      async=true assertions; remove old `PreToolUse` (general) assertion
- [ ] 3.4 Update `extension_content_has_all_four_handlers` and summary format
      tests to reflect the new constant
- [ ] 3.5 Add test: `SessionStart` entry has matcher `"startup|resume|clear"`
- [ ] 3.6 Add test: `PreToolUse` entry has matcher
      `"AskUserQuestion|ExitPlanMode"`
- [ ] 3.7 Add test: `PostToolUse` entry has no matcher and command
      `dv set-state working --harness claude`
- [ ] 3.8 Add test: `StopFailure` entry has no matcher, command
      `dv set-state done --harness claude`, and `async: true`

## 4. Verify and build

- [ ] 4.1 Run `mise run test` (or `cargo test`) — all tests pass
- [ ] 4.2 Run `mise run build` (or `cargo build`) — binary compiles clean
- [ ] 4.3 Run `dv hooks install claude` against a scratch `settings.json` and
      confirm all 8 entries appear with correct matchers, commands, and async
      flags
