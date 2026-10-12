import pc from "picocolors";
import {
  type AgentInfo,
  type AgentState,
  compareAgentState,
  type PaneEntry,
} from "./domain.ts";
import { timestampMillis, toRfc3339 } from "./time.ts";

/** Shared vocabulary of properties accepted by `--sort-by` and `--columns`. */
export const FIELDS = [
  "agent",
  "session",
  "pane",
  "status",
  "updatedAt",
  "activeTools",
] as const;
export type Field = (typeof FIELDS)[number];

export const DEFAULT_COLUMNS: readonly Field[] = [
  "agent",
  "session",
  "pane",
  "status",
  "activeTools",
];

export const DEFAULT_COLUMNS_STR = DEFAULT_COLUMNS.join(",");

export type StateMap = Map<string, AgentInfo>;

/** Color: idle → bright black, working → yellow, blocked → red, done → green. */
export function colorizeState(
  state: AgentState,
  text: string,
  useColor: boolean,
): string {
  if (!useColor) return text;
  const c = pc.createColors(true);
  switch (state) {
    case "idle":
      return c.gray(text);
    case "working":
      return c.yellow(text);
    case "blocked":
      return c.red(text);
    case "done":
      return c.green(text);
    default:
      return text;
  }
}

function statusOf(pane: PaneEntry, stateMap: StateMap): AgentState {
  return stateMap.get(pane.paneId)?.status ?? "unknown";
}

/** `active_tools` of a pane; panes with no state or `unknown` status count 0. */
function activeToolsOf(pane: PaneEntry, stateMap: StateMap): number {
  const info = stateMap.get(pane.paneId);
  return info && info.status !== "unknown" ? info.active_tools : 0;
}

function cellValue(
  pane: PaneEntry,
  status: AgentState,
  stateMap: StateMap,
  field: Field,
): string {
  switch (field) {
    case "agent":
      return pane.agent;
    case "session":
      return pane.session;
    case "pane":
      return `${pane.windowIndex}.${pane.paneIndex}`;
    case "status":
      return status;
    case "updatedAt": {
      const info = stateMap.get(pane.paneId);
      return info ? toRfc3339(info.updated_at) : "";
    }
    case "activeTools": {
      if (status === "unknown") return "";
      const n = activeToolsOf(pane, stateMap);
      return n === 0 ? "" : n === 1 ? "1 tool" : `${n} tools`;
    }
  }
}

/**
 * Format panes as tab-delimited plain text, one line per pane, rendering
 * exactly `columns` in order. Empty string when there are no panes.
 */
export function formatPanes(
  panes: readonly PaneEntry[],
  stateMap: StateMap,
  useColor: boolean,
  columns: readonly Field[],
): string {
  let output = "";
  for (const pane of panes) {
    const status = statusOf(pane, stateMap);
    const cells = columns.map((field) => {
      const cell = cellValue(pane, status, stateMap, field);
      return field === "status" ? colorizeState(status, cell, useColor) : cell;
    });
    output += `${cells.join("\t")}\n`;
  }
  return output;
}

function compareStrings(a: string, b: string): number {
  // Rust compares strings bytewise (UTF-8), which is code point order.
  const ac = Array.from(a, (ch) => ch.codePointAt(0) ?? 0);
  const bc = Array.from(b, (ch) => ch.codePointAt(0) ?? 0);
  const n = Math.min(ac.length, bc.length);
  for (let i = 0; i < n; i++) {
    const d = (ac[i] ?? 0) - (bc[i] ?? 0);
    if (d !== 0) return d;
  }
  return ac.length - bc.length;
}

/**
 * Sort panes in place by `key`. Natural direction is ascending except for
 * `updatedAt` and `activeTools` (descending); `reverse` flips it. Stable.
 */
export function sortPanes(
  panes: PaneEntry[],
  key: Field,
  reverse: boolean,
  stateMap: StateMap,
): void {
  panes.sort((a, b) => {
    switch (key) {
      case "status":
        return compareAgentState(statusOf(a, stateMap), statusOf(b, stateMap));
      case "agent":
        return compareStrings(a.agent, b.agent);
      case "session":
        return compareStrings(a.session, b.session);
      case "pane":
        return a.windowIndex - b.windowIndex || a.paneIndex - b.paneIndex;
      case "updatedAt": {
        const ta = stateMap.get(a.paneId);
        const tb = stateMap.get(b.paneId);
        const ma = ta
          ? timestampMillis(ta.updated_at)
          : Number.NEGATIVE_INFINITY;
        const mb = tb
          ? timestampMillis(tb.updated_at)
          : Number.NEGATIVE_INFINITY;
        if (ma === mb) return 0;
        return mb > ma ? 1 : -1;
      }
      case "activeTools":
        return activeToolsOf(b, stateMap) - activeToolsOf(a, stateMap);
      default:
        return 0;
    }
  });
  if (reverse) panes.reverse();
}
