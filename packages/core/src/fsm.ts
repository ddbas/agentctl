import type { AgentState, FsmEvent } from "./domain.ts";

/**
 * Status FSM. States are idle | working | blocked | done. `unknown` is never a
 * target of any transition but is accepted as a starting state (a missing or
 * corrupt state file behaves as "no matching transition").
 */
export const INITIAL_STATE: AgentState = "idle";

/** Returns the next state, or `undefined` when no transition matches. */
export function transition(
  state: AgentState,
  input: FsmEvent,
): AgentState | undefined {
  // Wildcard reset: SESSION_STARTED always moves to idle.
  if (input.kind === "SESSION_STARTED") return "idle";
  // Wildcard: RUN_FINISHED is authoritative from any status.
  if (input.kind === "RUN_FINISHED") return "done";

  const elicitation =
    (input.kind === "TOOL_CALL_START" || input.kind === "TOOL_CALL_RESULT") &&
    input.type === "elicitation";

  switch (state) {
    case "idle":
      return input.kind === "RUN_STARTED" ? "working" : undefined;
    case "done":
      if (input.kind === "RUN_STARTED") return "working";
      // A harness-initiated turn self-corrects at its first tool call.
      if (input.kind === "TOOL_CALL_START")
        return elicitation ? "blocked" : "working";
      return undefined;
    case "working":
      return input.kind === "TOOL_CALL_START" && elicitation
        ? "blocked"
        : undefined;
    case "blocked":
      return input.kind === "TOOL_CALL_RESULT" && elicitation
        ? "working"
        : undefined;
    default:
      return undefined;
  }
}

/** Apply `event`, falling back to `current` when no transition matches. */
export function applyEvent(current: AgentState, event: FsmEvent): AgentState {
  return transition(current, event) ?? current;
}
