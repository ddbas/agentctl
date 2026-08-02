## 1. rd hooks install pi

- [x] 1.1 Add a `pi` branch to the `rd hooks install <harness>` dispatcher
      (replacing the current exit-1 for `pi`)
- [x] 1.2 Embed the generated `radr.ts` extension source as a constant string in
      the `rd` binary
- [x] 1.3 Resolve `~/.pi/agent/extensions/radr.ts`; create the
      `~/.pi/agent/extensions/` directory if it does not exist
- [x] 1.4 If `radr.ts` already exists and is byte-for-byte identical to the
      embedded content, skip the write and mark the file as already up to date
      (`=`)
- [x] 1.5 If `radr.ts` exists but differs from the embedded content, overwrite
      it and mark as updated (`~`)
- [x] 1.6 If `radr.ts` does not exist, write the file and mark as new (`+`)
- [x] 1.7 Print a human-readable summary: the file path and all four event →
      command mappings on first install; `(already up to date)` on idempotent
      re-run

## 2. Tests

- [x] 2.1 Unit test: install when `~/.pi/agent/extensions/radr.ts` does not
      exist → file is created with all four event handlers; summary uses `+`
- [x] 2.2 Unit test: idempotent re-run → no write, summary uses `=`
- [x] 2.3 Unit test: install when file exists with different content → file is
      overwritten with new content; summary uses `~`
- [x] 2.4 Unit test: install creates `~/.pi/agent/extensions/` directory when it
      does not exist
- [x] 2.5 Unit test: generated file content includes handlers for
      `session_start`, `before_agent_start`, `agent_settled`, and
      `session_shutdown` mapped to the correct `rd` commands
- [x] 2.6 Integration test (testcontainers): `rd hooks install pi` on a
      container with no `~/.pi/agent/extensions/` directory → directory and
      `radr.ts` are created; file contains all four event handlers
- [x] 2.7 Integration test (testcontainers): `rd hooks install pi` twice →
      second run is a no-op; file byte-for-byte identical to after the first run

## 3. Documentation

- [x] 3.1 Update the project README with a "Pi hook integration" section
      covering `rd hooks install pi` as the required one-time setup step and
      listing the four lifecycle events
- [x] 3.2 Add `rd hooks install pi` to `AGENTS.md` as required setup for
      pi-based agents
