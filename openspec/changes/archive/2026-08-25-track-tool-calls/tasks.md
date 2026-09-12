## 1. State schema

- [x] 1.1 Add `active_tools: u32` to `AgentInfo` in `src/state.rs` with
      `#[serde(default)]` so old `state.json` files without the field
      deserialize as `0`.
- [x] 1.2 Extend `write_state_to` / `write_state` (or add a new helper, e.g.
      `write_state_with_tools_to`) so callers can persist an explicit
      `active_tools` value alongside `harness` and `status`, preserving the
      atomic write-then-rename behavior.
- [x] 1.3 Add/update unit tests in `src/state.rs` covering: round-trip of
      `active_tools`, default of `0` for a state file written without it, and
      deserialization of a legacy JSON blob missing the `active_tools` key.

## 2. Event handling

- [x] 2.1 In `src/commands/agent_event.rs::run`, after resolving
      `current_status`/`active_tools` from the pane's existing `AgentInfo`,
      compute the new `active_tools`: `+1` on `EventKind::ToolCallStart`, `-1`
      (saturating, floor `0`) on `EventKind::ToolCallResult`, `0` on
      `EventKind::SessionStarted`, unchanged otherwise.
- [x] 2.2 Write the new `active_tools` value together with the FSM-derived
      `status` in the same `write_state` call so the file is updated once per
      event.
- [x] 2.3 Add unit tests: `TOOL_CALL_START` increments `active_tools`;
      `TOOL_CALL_RESULT` decrements it; `TOOL_CALL_RESULT` on a `0` count stays
      `0`; multiple concurrent `TOOL_CALL_START`s accumulate; `--type custom`
      still increments/decrements; `SESSION_STARTED` resets a nonzero count to
      `0`; an elicitation `TOOL_CALL_START` both flips status to `blocked` and
      increments `active_tools` in the same call.

## 3. Pi extension template

- [x] 3.1 Update `PI_EXTENSION_CONTENT` in `src/commands/hooks.rs` to add
      `pi.on("tool_call", ...)` and `pi.on("tool_result", ...)` handlers that
      call
      `agentctl("agent", "event", "TOOL_CALL_START", "--id", event.toolCallId,     "--harness", "pi")`
      and the `TOOL_CALL_RESULT` equivalent.
- [x] 3.2 Update/add unit tests in `src/commands/hooks.rs` asserting the
      generated extension content includes the `tool_call`/`tool_result`
      handlers, and that re-running `agentctl install pi` over a file generated
      by the previous (pre-tool-call-handler) template is detected as stale and
      overwritten (`~` prefix).

## 4. agentctl list rendering

- [x] 4.1 In `src/commands/list.rs::format_panes`, add a fifth column value per
      pane: `N tool` (singular, `active_tools == 1`) or `N tools` (plural,
      `active_tools > 1`) when the pane has a matching, non-contradictory state
      file with `active_tools > 0`; empty string when `active_tools` is `0`,
      absent, or the pane's state is `unknown`.
- [x] 4.2 Pad the `state` column to its own max width plus two spaces (the same
      delimiter convention already used between the other columns), then append
      the fifth column's value; right-trim each rendered line so rows with an
      empty fifth column have no dangling delimiter/whitespace after `state`.
- [x] 4.3 Add unit tests: singular fifth column (`... working  1 tool`), plural
      fifth column (`... working  3 tools`), empty fifth column at
      `active_tools == 0` (line ends cleanly at `state` with no trailing
      whitespace), empty fifth column for `unknown` state even if a stale
      `active_tools` value is present, and column alignment (via the same
      two-space delimiter) across mixed rows with and without a fifth-column
      value.

## 5. Verification

- [x] 5.1 Run `mise run test` (or the project's equivalent task) and ensure the
      full suite passes, including all new tests above.
- [x] 5.2 Manually smoke-test end-to-end inside a tmux pane:
      `agentctl install pi`, start a pi session, trigger a few tool calls, and
      confirm `agentctl list` shows the expected `working N tool(s)` suffix
      while calls are in flight and reverts to plain `working` once they
      complete.
