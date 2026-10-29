import type { PaneEntry, Tmux } from "@agentctl/core";
import { shared } from "../support/world.ts";

class FakeTmux implements Tmux {
  listPanes(server?: string): PaneEntry[] {
    const { world, recorded } = shared();
    recorded.effects.push(
      `tmux list-panes -a (server: ${server ?? "<default>"})`,
    );
    if (world.tmuxError) throw new Error(world.tmuxError);
    // Copy: core mutates panes (node -> pi resolution).
    return world.panes.map((p) => ({ ...p }));
  }

  processTable(): Map<number, string[]> {
    const { world, recorded } = shared();
    recorded.effects.push("ps -A (process table)");
    return new Map(
      Object.entries(world.processTable).map(([k, v]) => [Number(k), v]),
    );
  }
}

/** Replacement module for `@agentctl/tmux`. */
export const tmuxMock = { ExecTmux: FakeTmux };
