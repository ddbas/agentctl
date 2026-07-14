## Why

`claude-harness-hooks` added the first concrete harness integration, proving the
lifecycle-event → `AgentState` pipeline end-to-end. The **pi coding agent**
(`pi`) is the second harness used in this project and deserves the same
first-class treatment: real-time state in `dv list` without any polling.

Pi ships a TypeScript extension system — not a subprocess-hook system like
Claude's — so the integration takes a different shape: `dv hooks install pi`
generates and deploys a TypeScript extension file to
`~/.pi/agent/extensions/drovr.ts`. The extension subscribes to pi lifecycle
events and shells out to `dv set-state` / `dv clear-state` exactly as the Claude
hooks do.

## What Changes

- Implement the `pi` handler for `dv hooks install pi`. The command writes a
  TypeScript extension file to `~/.pi/agent/extensions/drovr.ts` that subscribes
  to four pi lifecycle events and calls `dv set-state` / `dv clear-state`. It is
  idempotent: if the file already exists and is byte-for-byte identical to what
  would be generated, it reports no change.
- Map each pi event to the appropriate canonical `AgentState`:
  - `session_start` → `dv set-state idle --harness pi`
  - `before_agent_start` → `dv set-state working --harness pi`
  - `agent_settled` → `dv set-state done --harness pi`
  - `session_shutdown` → `dv clear-state`

Unlike Claude, pi does not expose a built-in permission-prompt event, so the
`blocked` state is not mapped in this integration.

## Capabilities

### New Capabilities

- `pi-state-hooks`: Pi-specific extension generation and the
  `dv hooks install pi` command that writes `~/.pi/agent/extensions/drovr.ts`.

### Modified Capabilities

_(none — no existing spec-level requirements change)_

## Impact

- **CLI binary (`dv`)**: adds the `pi` branch of `dv hooks install`, replacing
  the current "no harnesses supported" error with a working implementation for
  `pi`.
- **Pi extension file** (`~/.pi/agent/extensions/drovr.ts`): written once by the
  user via `dv hooks install pi`; no automatic mutation.
- **No breaking changes** to any existing commands or data formats.
