import type {
  AgentInfo,
  AgentState,
  EventRecord,
  PaneEntry,
} from "./domain.ts";

/** Persistence port for agent state (`state.json` / `events.jsonl`). */
export interface AgentRepository {
  /** Read the full `AgentInfo` for `paneId`, if it exists and is valid. */
  readAgentInfo(paneId: string): AgentInfo | undefined;
  /** Atomically write (or overwrite) the state for `paneId`. */
  writeStateWithTools(
    paneId: string,
    harness: string,
    status: AgentState,
    activeTools: number,
  ): void;
  /** Delete the state for `paneId`. No-op when absent. */
  deleteState(paneId: string): void;
  listAgentInfos(): AgentInfo[];
  appendEvent(paneId: string, record: EventRecord): void;
}

/** tmux / process-table port. */
export interface Tmux {
  /**
   * `tmux list-panes -a`. Empty when no server is running; throws an error
   * containing "tmux not found" when the binary is missing.
   */
  listPanes(server?: string): PaneEntry[];
  /** Parent-PID → argv[0] basenames of its direct children (best-effort). */
  processTable(): Map<number, string[]>;
}

/** Contract a harness integration implements for install / uninstall. */
export interface Harness {
  readonly name: string;
  /** Install and return a human-readable summary. */
  install(): string;
  /** Idempotent uninstall; returns a human-readable summary. */
  uninstall(): string;
}

export type LogLevel = "trace" | "debug" | "info" | "warn" | "error";
export const LOG_LEVELS: readonly LogLevel[] = [
  "trace",
  "debug",
  "info",
  "warn",
  "error",
];

/** Logging port; only the logging adapter knows the underlying library. */
export interface Logger {
  debug(msg: string): void;
  info(msg: string): void;
  warn(msg: string): void;
  error(msg: string): void;
}

/** Ambient process facts the core needs, injected by the composition root. */
export interface Runtime {
  /** Environment variable lookup. */
  env(name: string): string | undefined;
  now(): Date;
  /** Whether stdout is a terminal (controls colored output). */
  stdoutIsTerminal: boolean;
  version: string;
  /** Resolved log directory, if one can be determined. */
  logDir(): string | undefined;
  currentLogFile(dir: string): string;
  /** Replace the home directory prefix with `~`. */
  tildeCollapse(path: string): string;
}
