import type { PaneEntry, Tmux } from "@agentctl/core";
import { world } from "../support/world.ts";

class FakeTmux implements Tmux {
  listPanes(): PaneEntry[] {
    if (world().tmuxError) throw new Error(world().tmuxError);
    // Copy: core mutates panes (node -> pi resolution).
    return world().panes.map((p) => ({ ...p }));
  }

  processTable(): Map<number, string[]> {
    return new Map(
      Object.entries(world().processTable).map(([k, v]) => [Number(k), v]),
    );
  }
}

/** Replacement module for `@agentctl/tmux`. */
export const tmuxMock = { ExecTmux: FakeTmux };
