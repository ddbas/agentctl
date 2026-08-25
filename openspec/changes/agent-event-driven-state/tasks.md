## 1. Dependencies and schema rename

- [ ] 1.1 Add `rust-fsm` to `Cargo.toml`
- [ ] 1.2 Rename `AgentInfo.state` field to `status` in `src/state.rs`
      (`#[serde(rename)]` not needed if the Rust field is renamed directly;
      update every constructor/usage site)
- [ ] 1.3 Update `src/state.rs` unit tests and any doc comments referencing
      `state.json`'s `state` field to `status`
- [ ] 1.4 Update `src/commands/list.rs` and its tests for the field rename (no
      behavior change expected, just field name)

## 2. Events log

- [ ] 2.1 Add `src/events_log.rs` with an `EventRecord` type (event name, `id`,
      optional `type`, `harness`, `pane_id`, timestamp) and an
      `append_event_to(base, pane_id, record)` function that appends one JSON
      line to `<base>/radr/agents/<pane-id>/events.jsonl`, creating the file/dir
      if absent
- [ ] 2.2 Unit tests: first event creates the file with one line; second event
      appends without truncating; file/dir created when absent

## 3. Status FSM

- [ ] 3.1 Add `src/status_fsm.rs` defining the `rust-fsm` state machine (states
      `Idle`, `Working`, `Blocked`, `Done`; start `Idle`, end `Done`) with the
      transition table from design.md, including the wildcard
      `* --SESSION_STARTED--> Idle` reset transition
- [ ] 3.2 Implement a pure function
      `apply_event(current_status, event) ->     new_status` that runs the FSM
      and falls back to `current_status` unchanged when no transition matches
- [ ] 3.3 Unit tests for every transition and every documented no-op case in
      `specs/agent-events/spec.md` (idle/working/blocked/done → idle via
      `SESSION_STARTED`, idle→working, done→working, working→done,
      working→blocked via elicitation start, blocked→working via elicitation
      result, custom-type/no-type events are no-ops, unmatched-state no-op)

## 4. rd agent event command

- [ ] 4.1 Add an `AgentEventSubcommand` enum (one variant per event name:
      `SESSION_STARTED`, `RUN_STARTED`, `RUN_FINISHED`, `TOOL_CALL_START`,
      `TOOL_CALL_RESULT`, each with its own `--id`/`--type` args) nested under a
      new `Event { harness, pane, subcommand }` variant of `AgentSubcommand` in
      `src/main.rs`'s clap definitions, declaring `--harness`/`--pane` once with
      `#[arg(global = true)]` so every event subcommand inherits them, matching
      the CLI shape in design.md decision 1
- [ ] 4.2 Add `src/commands/agent_event.rs`: resolves pane ID (flag → env →
      silent exit), reads current status via existing `read_state`/
      `read_agent_info`, computes new status via `status_fsm::apply_event`,
      writes `state.json` via existing atomic-write helper, and appends the
      event via `events_log::append_event_to` (including for `SESSION_STARTED`,
      which is logged like every other event)
- [ ] 4.3 Remove `src/commands/agent_status.rs` and its
      `AgentSubcommand::Status` wiring in `src/main.rs`; do not add a separate
      non-event `rd agent init` command — session bootstrap goes exclusively
      through `rd agent event SESSION_STARTED`
- [ ] 4.4 Wire the `Event` subcommand into `src/commands/mod.rs` and
      `src/main.rs`'s `run()` match arms
- [ ] 4.5 Unit tests for `rd agent event` per `specs/agent-events/spec.md`:
      required `--id` on all five events, rejected `--type` on `RUN_*`/
      `SESSION_STARTED`, accepted/invalid `--type` values on `TOOL_CALL_*`,
      unknown event name rejected, outside-tmux no-op, pane resolution order
      (`--pane` overrides `$TMUX_PANE`), `rd agent init` no longer exists as a
      subcommand, and that `--harness`/`--pane` are accepted by every event
      subcommand both before and after the event name on the command line

## 5. pi integration

- [ ] 5.1 Update the generated `radr.ts` template in `src/commands/hooks.rs` to
      call `rd agent event SESSION_STARTED --id <sessionId> --harness pi` on
      `session_start`, generate a run ID via `crypto.randomUUID()` and call
      `rd agent event RUN_STARTED --id <runId> --harness pi` on
      `before_agent_start`, reuse that run ID for
      `rd agent event RUN_FINISHED --id <runId> --harness pi` on
      `agent_settled`, and keep `rd agent clear` on `session_shutdown`
- [ ] 5.2 Update `rd install pi`'s idempotent-content check and stale-file
      detection to recognize/replace the old
      `rd agent status`/`rd agent     clear` generated content
- [ ] 5.3 Update `tests/agent_commands.rs` / `src/commands/hooks.rs` unit tests
      asserting on the pi extension's generated content and install/upgrade
      summaries

## 6. Claude integration

- [ ] 6.1 Update the nine generated hook entries in `src/commands/hooks.rs` per
      the table in `specs/claude-state-hooks/spec.md`: `SessionStart` → shell
      snippet extracting `session_id` via `jq` then
      `rd agent event     SESSION_STARTED`; `UserPromptSubmit`, `Stop`,
      `StopFailure` → shell snippets extracting `session_id` then
      `rd agent event     RUN_STARTED`/`RUN_FINISHED`;
      `PreToolUse`/`PostToolUse` matched to `AskUserQuestion|ExitPlanMode` →
      shell snippets extracting `tool_use_id` then
      `TOOL_CALL_START`/`TOOL_CALL_RESULT --type     elicitation`; unmatched
      `PostToolUse` and `Notification` matched to `permission_prompt` → shell
      snippets extracting `session_id` and using the synthetic
      `perm-<session_id>` id for `TOOL_CALL_RESULT`/
      `TOOL_CALL_START --type elicitation`; `SessionEnd` → `rd agent clear`
- [ ] 6.2 Extend `rd install claude`/`rd uninstall claude`'s radr-managed
      hook-entry recognition/migration logic to add the new `rd agent status` →
      `rd agent event` migration path alongside the existing
      `set-state`/`state set` migration chain
- [ ] 6.3 Update `tests/agent_commands.rs` / `src/commands/hooks.rs` unit tests
      asserting on generated Claude settings content, install/upgrade/ uninstall
      summaries, and each migration scenario (pre-rename, intermediate, and
      `agent status` legacy forms)

## 7. Cross-cutting verification

- [ ] 7.1 Run the full test suite (`mise run test` or equivalent) and fix any
      remaining references to `rd agent status`, `rd agent init`, or the `state`
      field across `tests/common/mod.rs` and elsewhere
- [ ] 7.2 Manually install both harnesses in a scratch tmux pane, drive a full
      run (including a permission prompt / `AskUserQuestion`), and confirm
      `state.json` shows the expected `status` transitions and `events.jsonl`
      records every dispatched event, including `SESSION_STARTED`
- [ ] 7.3 Update `README`/help text (`rd agent --help`, `Cli` doc comments) to
      describe `rd agent event` (including `SESSION_STARTED`) in place of
      `rd agent status`
