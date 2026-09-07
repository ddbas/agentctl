## 1. Update AGENTCTL_CLAUDE_HOOKS constant

- [x] 1.1 Change `SessionStart` matcher from `"startup"` to
      `"startup|resume|clear"` in `AGENTCTL_CLAUDE_HOOKS`
- [x] 1.2 Remove the general `PreToolUse` (no matcher) →
      `agentctl set-state working` entry
- [x] 1.3 Add `PostToolUse` (no matcher) →
      `agentctl set-state working --harness claude` [async] entry
- [x] 1.4 Add `PreToolUse` with `matcher: "AskUserQuestion|ExitPlanMode"` →
      `agentctl set-state blocked --harness claude` [async] entry
- [x] 1.5 Add `StopFailure` (no matcher) →
      `agentctl set-state done --harness claude` [async] entry

## 2. Update install summary output

- [x] 2.1 Update `format_claude_summary` (and `print_*` output) to reflect the
      new 8-entry hook set (was 6)

## 3. Update unit tests

- [x] 3.1 Update `install_claude_clean_file_writes_all_six_entries` test: rename
      to `..._all_eight_entries` and assert all 8 events are present
      (`SessionStart`, `UserPromptSubmit`, `PostToolUse`, `PreToolUse`, `Stop`,
      `StopFailure`, `Notification`, `SessionEnd`)
- [x] 3.2 Update `install_claude_idempotent_no_duplicates` test to assert 8
      event keys with no duplicates
- [x] 3.3 Update `install_claude_async_flags_are_correct` test: add
      `PostToolUse`, `PreToolUse` (AskUserQuestion), `StopFailure` to the
      async=true assertions; remove old `PreToolUse` (general) assertion
- [x] 3.4 Update `extension_content_has_all_four_handlers` and summary format
      tests to reflect the new constant
- [x] 3.5 Add test: `SessionStart` entry has matcher `"startup|resume|clear"`
- [x] 3.6 Add test: `PreToolUse` entry has matcher
      `"AskUserQuestion|ExitPlanMode"`
- [x] 3.7 Add test: `PostToolUse` entry has no matcher and command
      `agentctl set-state working --harness claude`
- [x] 3.8 Add test: `StopFailure` entry has no matcher, command
      `agentctl set-state done --harness claude`, and `async: true`

## 4. Verify and build

- [x] 4.1 Run `mise run test` (or `cargo test`) — all tests pass
- [x] 4.2 Run `mise run build` (or `cargo build`) — binary compiles clean
- [x] 4.3 Run `agentctl hooks install claude` against a scratch `settings.json`
      and confirm all 8 entries appear with correct matchers, commands, and
      async flags

## 5. Smart cleanup in installer

- [x] 5.1 Add `is_agentctl_command(cmd: &str) -> bool` and
      `get_single_agentctl_command(entry: &Value) -> Option<&str>` private
      helpers
- [x] 5.2 Add `HookStatus` enum (`New`, `Updated`, `Unchanged`) to replace
      `Vec<bool>` status tracking
- [x] 5.3 Add cleanup pass in `install_claude_to` before the add-loop: iterate
      all event arrays, remove agentctl entries whose (event, command) pair is
      no longer in `AGENTCTL_CLAUDE_HOOKS`, update matchers that differ; collect
      `removed: Vec<(String,String)>` and
      `updated_commands: HashSet<(String,String)>`
- [x] 5.4 Update `format_claude_summary` signature to
      `(statuses: &[HookStatus], removed: &[(String, String)])` and output `~`
      for `Updated`, `+` for `New`, `=` for `Unchanged`, and `-` lines for
      removed entries
- [x] 5.5 Add tests: stale `SessionStart` matcher is updated, superseded
      `PreToolUse(*)` → working entry is removed, `HookStatus::Updated` renders
      as `~` in summary
- [x] 5.6 Run full test suite; run `agentctl hooks install claude` and confirm
      `~` on `SessionStart` and `+` on new entries for an already-installed
      settings file
