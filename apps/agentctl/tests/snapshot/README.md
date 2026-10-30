# CLI snapshot tests

Snapshot every CLI command's output (stdout, stderr, and exit code). No Docker,
no real tmux, no filesystem.

Everything in this directory is **test-only** code.

- `cli.snapshot.test.ts` — runs every case, asserts against `__snapshots__/`.
  Holds the `vi.mock()` declarations for the infrastructure packages.
- `cases.ts` — the table of CLI invocations (one snapshot file per case).
- `coverage.test.ts` — fails if a command has no case.
- `mocks/` — in-memory fakes that replace the adapter packages
  (`@agentctl/tmux`, `@agentctl/state-fs`, `@agentctl/logging`, harnesses).
- `fixtures/` — named "worlds" (tmux panes + agent state) the fakes serve.
- `support/` — helpers (`run-cli.ts` runs the real `src/cli.ts` in-process).
- `__snapshots__/` — the reviewable output. Review diffs here in PRs.

Run everything: `mise run test`.

Run only these tests:

```sh
mise exec -- pnpm --filter agentctl exec vitest run tests/snapshot
```

Accept intended output changes:

```sh
mise exec -- pnpm --filter agentctl exec vitest run tests/snapshot -u
```
