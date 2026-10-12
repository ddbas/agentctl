import { describe, expect, it } from "vitest";
import type { AgentInfo, PaneEntry } from "./domain.ts";
import {
  filterAgentPanes,
  resolveFromProcessTable,
  validateStateFiles,
} from "./panes.ts";
import type { AgentRepository } from "./ports.ts";

const pane = (agent: string, id = "%1", pid = 0): PaneEntry => ({
  agent,
  session: "s",
  windowIndex: 0,
  paneIndex: 0,
  panePid: pid,
  paneId: id,
});

describe("filterAgentPanes", () => {
  it("keeps claude and pi, drops others and near matches", () => {
    const out = filterAgentPanes([
      pane("claude"),
      pane("pi"),
      pane("bash"),
      pane("claude-code"),
      pane("pip"),
    ]);
    expect(out.map((p) => p.agent)).toEqual(["claude", "pi"]);
  });
  it("empty input returns empty", () => {
    expect(filterAgentPanes([])).toEqual([]);
  });
});

describe("resolveFromProcessTable", () => {
  it("node pane with pi child becomes pi", () => {
    const panes = [pane("node", "%1", 100)];
    resolveFromProcessTable(panes, new Map([[100, ["pi"]]]));
    expect(panes[0]?.agent).toBe("pi");
  });
  it("node pane without pi child stays node; claude unchanged", () => {
    const panes = [pane("node", "%1", 100), pane("claude", "%2", 200)];
    resolveFromProcessTable(
      panes,
      new Map([
        [100, ["bash"]],
        [200, ["pi"]],
      ]),
    );
    expect(panes.map((p) => p.agent)).toEqual(["node", "claude"]);
  });
});

describe("validateStateFiles", () => {
  const info = (id: string, harness: string): AgentInfo => ({
    pane_id: id,
    harness,
    status: "idle",
    updated_at: "2026-01-01T00:00:00Z",
    active_tools: 0,
  });
  const run = (stateInfo: AgentInfo, live: PaneEntry[]) => {
    const deleted: string[] = [];
    const repo = {
      deleteState: (id: string) => deleted.push(id),
    } as unknown as AgentRepository;
    const map = new Map([[stateInfo.pane_id, stateInfo]]);
    validateStateFiles(repo, map, live);
    return { deleted, map };
  };

  it("deletes orphaned state", () => {
    expect(run(info("%9", "claude"), [pane("claude", "%1")]).deleted).toEqual([
      "%9",
    ]);
  });
  it("deletes contradictory harness", () => {
    const r = run(info("%1", "pi"), [pane("claude", "%1")]);
    expect(r.deleted).toEqual(["%1"]);
    expect(r.map.size).toBe(0);
  });
  it("keeps matching state", () => {
    expect(run(info("%1", "claude"), [pane("claude", "%1")]).deleted).toEqual(
      [],
    );
  });
  it("keeps unknown harness even if command differs", () => {
    expect(run(info("%1", "unknown"), [pane("claude", "%1")]).deleted).toEqual(
      [],
    );
  });
});
