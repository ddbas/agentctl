## 1. dv hooks install pi

- [ ] 1.1 Add a `pi` branch to the `dv hooks install <harness>` dispatcher
      (replacing the current exit-1 for `pi`)
- [ ] 1.2 Embed the generated `drovr.ts` extension source as a constant string
      in the `dv` binary
- [ ] 1.3 Resolve `~/.pi/agent/extensions/drovr.ts`; create the
      `~/.pi/agent/extensions/` directory if it does not exist
- [ ] 1.4 If `drovr.ts` already exists and is byte-for-byte identical to the
      embedded content, skip the write and mark the file as already up to date
      (`=`)
- [ ] 1.5 If `drovr.ts` exists but differs from the embedded content, overwrite
      it and mark as updated (`~`)
- [ ] 1.6 If `drovr.ts` does not exist, write the file and mark as new (`+`)
- [ ] 1.7 Print a human-readable summary: the file path and all four event →
      command mappings on first install; `(already up to date)` on idempotent
      re-run

## 2. Tests

- [ ] 2.1 Unit test: install when `~/.pi/agent/extensions/drovr.ts` does not
      exist → file is created with all four event handlers; summary uses `+`
- [ ] 2.2 Unit test: idempotent re-run → no write, summary uses `=`
- [ ] 2.3 Unit test: install when file exists with different content → file is
      overwritten with new content; summary uses `~`
- [ ] 2.4 Unit test: install creates `~/.pi/agent/extensions/` directory when it
      does not exist
- [ ] 2.5 Unit test: generated file content includes handlers for
      `session_start`, `before_agent_start`, `agent_settled`, and
      `session_shutdown` mapped to the correct `dv` commands
- [ ] 2.6 Integration test (testcontainers): `dv hooks install pi` on a
      container with no `~/.pi/agent/extensions/` directory → directory and
      `drovr.ts` are created; file contains all four event handlers
- [ ] 2.7 Integration test (testcontainers): `dv hooks install pi` twice →
      second run is a no-op; file byte-for-byte identical to after the first run

## 3. Documentation

- [ ] 3.1 Update the project README with a "Pi hook integration" section
      covering `dv hooks install pi` as the required one-time setup step and
      listing the four lifecycle events
- [ ] 3.2 Add `dv hooks install pi` to `AGENTS.md` as required setup for
      pi-based agents
