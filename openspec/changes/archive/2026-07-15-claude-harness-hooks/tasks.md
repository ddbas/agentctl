## 1. dv hooks install claude

- [x] 1.1 Add a `claude` branch to the `dv hooks install <harness>` dispatcher
      (replacing the current exit-1 for `claude`)
- [x] 1.2 Read `~/.claude/settings.json`; create it with an empty `{}` body if
      absent
- [x] 1.3 Parse the JSON and merge the six drovr hook entries (SessionStart,
      UserPromptSubmit, PreToolUse, Stop, Notification, SessionEnd) using the
      command strings defined in the design
- [x] 1.4 Ensure idempotency: identify existing drovr entries by their `command`
      string; do not duplicate them
- [x] 1.5 Preserve all existing non-drovr hook entries in the file
- [x] 1.6 Write the updated JSON back to `~/.claude/settings.json` atomically
      (write to a `.tmp` sibling, then rename)
- [x] 1.7 Print a human-readable summary: `+` for newly added entries, `=` for
      entries already present

## 2. Tests

- [x] 2.1 Unit test: install on a clean (empty) settings file → all six entries
      written correctly
- [x] 2.2 Unit test: idempotent re-run → no duplicate entries, file content
      identical to after first run
- [x] 2.3 Unit test: install merges with existing non-drovr hooks → existing
      entries preserved, drovr entries added alongside them
- [x] 2.4 Unit test: `async: true` is set on UserPromptSubmit, PreToolUse, Stop,
      and Notification; absent on SessionStart and SessionEnd
- [x] 2.5 Unit test: summary output correctly uses `+` vs `=` on first and
      second run
- [x] 2.6 Integration test (testcontainers): `dv hooks install claude` on a
      container with no `~/.claude/settings.json` → file is created and contains
      all six entries
- [x] 2.7 Integration test (testcontainers): `dv hooks install claude` twice →
      second run is a no-op; file byte-for-byte identical to after the first run

## 3. Documentation

- [x] 3.1 Update the project README with a "Claude hook integration" section
      covering `dv hooks install claude` as the required one-time setup step and
      listing the six hook events
- [x] 3.2 Add `dv hooks install claude` to `AGENTS.md` as required setup for
      Claude-based agents
