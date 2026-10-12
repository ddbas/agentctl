import type {
  AgentState,
  EventKind,
  EventRecord,
  FsmEvent,
  ToolCallType,
} from "./domain.ts";
import { applyEvent } from "./fsm.ts";
import { type Field, formatPanes, sortPanes } from "./list.ts";
import {
  filterAgentPanes,
  loadStateMap,
  resolveFromProcessTable,
  validateStateFiles,
} from "./panes.ts";
import type {
  AgentRepository,
  Harness,
  Logger,
  LogLevel,
  Runtime,
  Tmux,
} from "./ports.ts";
import { formatTimestamp } from "./time.ts";

export interface AppDeps {
  repo: AgentRepository;
  tmux: Tmux;
  harnesses: Harness[];
  runtime: Runtime;
  logger?: Logger;
}

function eventName(kind: EventKind): string {
  return kind.kind;
}

/**
 * Application/use-case layer. Depends only on ports; the composition root
 * supplies the concrete implementations.
 */
export class App {
  constructor(private readonly deps: AppDeps) {}

  /** `agentctl list` — enumerate, resolve state, sort, and render. */
  list(
    server: string | undefined,
    sortBy: Field,
    reverse: boolean,
    columns: readonly Field[],
  ): string {
    const { repo, tmux, runtime } = this.deps;
    const socket = server ?? runtime.env("AGENTCTL_TMUX_SOCKET");

    const panes = tmux.listPanes(socket);
    if (panes.some((p) => p.agent === "node")) {
      resolveFromProcessTable(panes, tmux.processTable());
    }
    const agentPanes = filterAgentPanes(panes);

    const stateMap = loadStateMap(repo);
    validateStateFiles(repo, stateMap, panes);

    sortPanes(agentPanes, sortBy, reverse, stateMap);
    return formatPanes(agentPanes, stateMap, runtime.stdoutIsTerminal, columns);
  }

  install(harnessName: string): string {
    return this.findHarness(harnessName).install();
  }

  uninstall(harnessName: string): string {
    return this.findHarness(harnessName).uninstall();
  }

  private findHarness(name: string): Harness {
    const harness = this.deps.harnesses.find((h) => h.name === name);
    if (!harness) throw new Error(`unknown harness: ${name}`);
    return harness;
  }

  private resolvePane(pane: string | undefined): string | undefined {
    const id = pane ?? this.deps.runtime.env("TMUX_PANE");
    return id ? id : undefined;
  }

  /**
   * `agentctl agent event <NAME>` — derive the pane's status through the FSM
   * and append the raw event to `events.jsonl`. Silent no-op outside tmux.
   */
  dispatchEvent(
    kind: EventKind,
    id: string,
    toolCallType: ToolCallType | undefined,
    harness: string | undefined,
    pane: string | undefined,
  ): void {
    const { repo, runtime } = this.deps;
    const paneId = this.resolvePane(pane);
    if (!paneId) return;

    const harnessStr = harness ?? "unknown";
    const current = repo.readAgentInfo(paneId);
    const currentStatus: AgentState = current?.status ?? "unknown";
    const currentActive = current?.active_tools ?? 0;

    let newActive: number;
    switch (kind.kind) {
      case "SESSION_STARTED":
        newActive = 0;
        break;
      case "TOOL_CALL_START":
        newActive = currentActive + 1;
        break;
      case "TOOL_CALL_RESULT":
        newActive = Math.max(0, currentActive - 1);
        break;
      case "RUN_FINISHED":
        newActive = kind.activeTools ?? currentActive;
        break;
      default:
        newActive = currentActive;
    }

    const event: FsmEvent =
      kind.kind === "TOOL_CALL_START" || kind.kind === "TOOL_CALL_RESULT"
        ? { kind: kind.kind, type: toolCallType }
        : { kind: kind.kind };
    const newStatus = applyEvent(currentStatus, event);

    repo.writeStateWithTools(paneId, harnessStr, newStatus, newActive);

    const record: EventRecord = {
      event: eventName(kind),
      id,
      ...(toolCallType === undefined ? {} : { type: toolCallType }),
      harness: harnessStr,
      pane_id: paneId,
      timestamp: formatTimestamp(runtime.now()),
    };
    repo.appendEvent(paneId, record);
  }

  /** `agentctl agent clear` — whole-session cleanup for the pane. */
  clear(pane: string | undefined): void {
    const paneId = this.resolvePane(pane);
    if (!paneId) return;
    this.deps.repo.deleteState(paneId);
  }

  /** `agentctl info` — `key: value` lines; performs no filesystem writes. */
  info(logLevel: LogLevel): string {
    const { runtime } = this.deps;
    const logDir = runtime.logDir();
    if (!logDir) throw new Error("failed to resolve log directory");
    const currentLogFile = runtime.currentLogFile(logDir);
    return `version: ${runtime.version}\nlog level: ${logLevel}\nlog directory: ${runtime.tildeCollapse(logDir)}\ncurrent log file: ${runtime.tildeCollapse(currentLogFile)}\n`;
  }
}
