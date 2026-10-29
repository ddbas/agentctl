import type { AgentInfo, PaneEntry } from "@agentctl/core";
import type { World } from "../support/world.ts";

const pane = (
  paneId: string,
  agent: string,
  session: string,
  windowIndex: number,
  paneIndex: number,
  panePid = 1000 + Number(paneId.slice(1)),
): PaneEntry => ({ paneId, agent, session, windowIndex, paneIndex, panePid });

const state = (
  pane_id: string,
  harness: string,
  status: AgentInfo["status"],
  updated_at: string,
  active_tools = 0,
): AgentInfo => ({ pane_id, harness, status, updated_at, active_tools });

const empty: World = { panes: [], processTable: {}, states: [] };

/** Mixed panes covering every status, sorting keys, and cleanup rules. */
const mixed: World = {
  panes: [
    pane("%1", "claude", "work", 0, 0),
    pane("%2", "pi", "work", 0, 1),
    pane("%3", "claude", "play", 1, 0),
    pane("%4", "node", "play", 1, 1, 400), // resolves to pi via process table
    pane("%5", "zsh", "play", 2, 0), // not an agent
    pane("%6", "claude", "docs", 0, 0), // agent without state
    pane("%7", "claude", "docs", 0, 1), // state says pi -> contradictory, cleaned
  ],
  processTable: { 400: ["pi"] },
  states: [
    state("%1", "claude", "working", "2026-01-01T11:00:00Z", 2),
    state("%2", "pi", "idle", "2026-01-01T10:00:00Z"),
    state("%3", "claude", "blocked", "2026-01-01T11:30:00Z", 1),
    state("%4", "pi", "done", "2026-01-01T09:00:00Z"),
    state("%7", "pi", "working", "2026-01-01T08:00:00Z", 1), // contradictory
    state("%9", "claude", "idle", "2026-01-01T07:00:00Z"), // orphaned
  ],
};

/** One pane with an in-flight tool call, for `agent event` / `agent clear`. */
const working: World = {
  panes: [pane("%1", "claude", "work", 0, 0)],
  processTable: {},
  states: [state("%1", "claude", "working", "2026-01-01T11:00:00Z", 1)],
};

/** One blocked pane (waiting on an elicitation). */
const blocked: World = {
  panes: [pane("%1", "claude", "work", 0, 0)],
  processTable: {},
  states: [state("%1", "claude", "blocked", "2026-01-01T11:00:00Z", 1)],
};

const tmuxMissing: World = {
  ...empty,
  tmuxError: "tmux not found — is tmux installed and on PATH?",
};

const noLogDir: World = { ...empty, noLogDir: true };

export const WORLDS = {
  empty,
  mixed,
  working,
  blocked,
  tmuxMissing,
  noLogDir,
} satisfies Record<string, World>;

export type WorldName = keyof typeof WORLDS;
