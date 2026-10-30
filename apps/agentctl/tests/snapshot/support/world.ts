import type { AgentInfo, PaneEntry } from "@agentctl/core";

/** Fixed "now" returned by the mocked clock and the fake repository. */
export const FIXED_NOW = "2026-01-01T12:00:00Z";
export const FAKE_HOME = "/home/test";

/** Everything the fake adapters serve for one CLI invocation. */
export interface World {
  panes: PaneEntry[];
  /** parent pid -> child argv[0] basenames */
  processTable: Record<number, string[]>;
  states: AgentInfo[];
  /** When set, `tmux.listPanes` throws this message. */
  tmuxError?: string;
  /** When true, `logDir()` resolves to `undefined`. */
  noLogDir?: boolean;
}

const KEY = "__agentctlSnapshotWorld";
type G = typeof globalThis & { [KEY]?: World };

// Stored on `globalThis` because `vi.resetModules()` re-evaluates modules, but
// the fakes and the runner must still see the same object.
export function setWorld(world: World): void {
  (globalThis as G)[KEY] = world;
}

export function world(): World {
  const w = (globalThis as G)[KEY];
  if (!w) throw new Error("snapshot world not initialised");
  return w;
}
