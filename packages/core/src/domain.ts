/** Agent state domain types — the content shape of `state.json` / `events.jsonl`. */

export const AGENT_STATES = [
  "idle",
  "working",
  "done",
  "blocked",
  "unknown",
] as const;

/** The canonical set of states an agent can be in. */
export type AgentState = (typeof AGENT_STATES)[number];

/** Canonical sort priority: blocked < done < working < idle < unknown. */
const PRIORITY: Record<AgentState, number> = {
  blocked: 0,
  done: 1,
  working: 2,
  idle: 3,
  unknown: 4,
};

export function compareAgentState(a: AgentState, b: AgentState): number {
  return PRIORITY[a] - PRIORITY[b];
}

/** Content of a `state.json` file for one agent pane. */
export interface AgentInfo {
  /** Tmux pane ID (e.g. `%12`), used as both directory name and correlation key. */
  pane_id: string;
  /** Agent harness name (e.g. `claude`, `pi`), or `"unknown"` when not set. */
  harness: string;
  status: AgentState;
  /** RFC 3339 timestamp (UTC) of the last state change. */
  updated_at: string;
  /** Count of tool calls currently in flight for this pane. */
  active_tools: number;
}

export type ToolCallType = "custom" | "elicitation";

/** One line of `events.jsonl`: a single dispatched event. */
export interface EventRecord {
  /** e.g. `SESSION_STARTED`, `RUN_STARTED`, `TOOL_CALL_START`. */
  event: string;
  /** Caller-supplied unique id (session id, run id, or tool-call id). */
  id: string;
  /** Only present for `TOOL_CALL_START` / `TOOL_CALL_RESULT`. */
  type?: ToolCallType;
  harness: string;
  pane_id: string;
  /** RFC 3339 timestamp (UTC) of when the event was dispatched. */
  timestamp: string;
}

/** A single tmux pane entry returned by `tmux list-panes -a`. */
export interface PaneEntry {
  /** Foreground process name; `node` panes are resolved to `pi` when possible. */
  agent: string;
  session: string;
  windowIndex: number;
  paneIndex: number;
  /** PID of the pane's shell process (used to resolve `node` → `pi`). */
  panePid: number;
  /** Tmux pane ID (e.g. `%12`). */
  paneId: string;
}

/** Events accepted by `agentctl agent event`, as FSM inputs. */
export type FsmEvent =
  | { kind: "SESSION_STARTED" }
  | { kind: "RUN_STARTED" }
  | { kind: "RUN_FINISHED" }
  | { kind: "TOOL_CALL_START"; type?: ToolCallType }
  | { kind: "TOOL_CALL_RESULT"; type?: ToolCallType };

/** Event requested via the CLI, before it is turned into an FSM input. */
export type EventKind =
  | { kind: "SESSION_STARTED" }
  | { kind: "RUN_STARTED" }
  /** `activeTools`, when set, overrides the pane's counter. */
  | { kind: "RUN_FINISHED"; activeTools?: number }
  | { kind: "TOOL_CALL_START" }
  | { kind: "TOOL_CALL_RESULT" };
