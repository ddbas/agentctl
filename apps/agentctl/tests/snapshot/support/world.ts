import type { AgentInfo, PaneEntry } from "@agentctl/core";

/** Fixed "now" returned by the mocked clock and the fake repository. */
export const FIXED_NOW = "2026-01-01T12:00:00Z";
export const FAKE_HOME = "/home/test";

/** Everything the fake adapters serve / record for one CLI invocation. */
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

/** Side effects recorded by the fakes, in call order. */
export interface Recorded {
  effects: string[];
}

interface Shared {
  world: World;
  recorded: Recorded;
}

const KEY = "__agentctlSnapshotWorld";
type G = typeof globalThis & { [KEY]?: Shared };

// Stored on `globalThis` because `vi.resetModules()` re-evaluates modules, but
// the fakes and the runner must still see the same object.
export function setShared(shared: Shared): void {
  (globalThis as G)[KEY] = shared;
}

export function shared(): Shared {
  const s = (globalThis as G)[KEY];
  if (!s) throw new Error("snapshot world not initialised");
  return s;
}
