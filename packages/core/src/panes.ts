import type { AgentInfo, PaneEntry } from "./domain.ts";
import type { AgentRepository } from "./ports.ts";

/** Exact process names that are considered known agent harness processes. */
const KNOWN_AGENTS = ["claude", "pi"];

/**
 * Mutate any pane whose `agent === "node"` to `pi` when the pane's shell PID
 * has a direct child whose argv[0] basename is `pi`.
 */
export function resolveFromProcessTable(
  panes: PaneEntry[],
  table: Map<number, string[]>,
): void {
  for (const pane of panes) {
    if (pane.agent !== "node") continue;
    if (table.get(pane.panePid)?.includes("pi")) pane.agent = "pi";
  }
}

/** Keep only panes whose `agent` exactly matches a known harness name. */
export function filterAgentPanes(panes: PaneEntry[]): PaneEntry[] {
  return panes.filter((p) => KNOWN_AGENTS.includes(p.agent));
}

/** Build a `pane_id → AgentInfo` map from every entry known to `repo`. */
export function loadStateMap(repo: AgentRepository): Map<string, AgentInfo> {
  return new Map(repo.listAgentInfos().map((info) => [info.pane_id, info]));
}

/**
 * Delete stale state entries: orphaned (pane not live) or contradictory
 * (live pane's command differs from a non-`unknown` harness).
 */
export function validateStateFiles(
  repo: AgentRepository,
  stateMap: Map<string, AgentInfo>,
  livePanes: PaneEntry[],
): void {
  const live = new Map(livePanes.map((p) => [p.paneId, p.agent]));
  const stale = [...stateMap.values()]
    .filter((info) => {
      const cmd = live.get(info.pane_id);
      return (
        cmd === undefined ||
        (info.harness !== "unknown" && cmd !== info.harness)
      );
    })
    .map((info) => info.pane_id);

  for (const paneId of stale) {
    try {
      repo.deleteState(paneId);
    } catch {
      // best-effort
    }
    stateMap.delete(paneId);
  }
}
