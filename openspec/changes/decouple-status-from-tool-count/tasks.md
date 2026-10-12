## 1. Status FSM

- [ ] 1.1 In `crates/agentctl-state/src/status_fsm.rs`, remove the
      `active_tools` field from `Event::RunFinished` and make the wildcard arm
      always return `Some(Done)`; update the module/enum doc comments
- [ ] 1.2 Replace the "nonzero active_tools is a no-op from every state" unit
      test with one asserting `RunFinished` reaches `Done` from every state;
      update remaining `RunFinished { active_tools: 0 }` test call sites

- [ ] 1.3 Add `Done` + `TOOL_CALL_START` transitions (`Working` for
      untyped/custom, `Blocked` for elicitation); add unit tests incl.
      `TOOL_CALL_RESULT` and `Idle` + `TOOL_CALL_START` staying no-ops, and
      update the existing custom/no-type no-op tests (they start from `Working`)

## 2. Dispatch

- [ ] 2.1 In `crates/agentctl-core/src/app.rs` `dispatch_event`, construct
      `Event::RunFinished` without the count; keep computing and persisting
      `new_active_tools`
- [ ] 2.2 Add/adjust `app.rs` tests: `RUN_FINISHED` with nonzero counter →
      `done` with counter preserved; `RUN_FINISHED --active-tools N` → `done`
      with counter `N`; blocked + nonzero counter → `done`
- [ ] 2.3 Add an `app.rs` test: `TOOL_CALL_START` on a `done` pane → `working`
      with counter incremented
- [ ] 2.4 Update any integration tests in `crates/agentctl/tests/hooks.rs` that
      assert a Stop-with-background-tasks pane stays `working`

## 3. Docs and specs

- [ ] 3.1 Update `README.md`: transition table row for `RUN_FINISHED`, the "only
      refuses to fire…" paragraph, and the Claude `<n>` explanation; remove
      stale `permission-prompt`/`blocked_prompts` text
- [ ] 3.2 Run `openspec validate decouple-status-from-tool-count` and fix any
      delta-spec issues

## 4. Verify

- [x] 4.1 Run the full test suite via `mise run` (per AGENTS.md) and confirm
      green
- [ ] 4.2 Optional spike: in a testcontainer, run Claude with a short background
      shell and inspect `events.jsonl` to confirm whether a wake-up turn emits
      `UserPromptSubmit`
