import { describe, expect, it } from "vitest";
import { App } from "./app.ts";
import type { AgentInfo, EventRecord } from "./domain.ts";
import type { AgentRepository, Harness, Runtime, Tmux } from "./ports.ts";

class MemoryRepo implements AgentRepository {
  infos = new Map<string, AgentInfo>();
  events: EventRecord[] = [];
  readAgentInfo(id: string) {
    return this.infos.get(id);
  }
  writeStateWithTools(
    paneId: string,
    harness: string,
    status: AgentInfo["status"],
    activeTools: number,
  ) {
    this.infos.set(paneId, {
      pane_id: paneId,
      harness,
      status,
      updated_at: "2026-01-01T00:00:00Z",
      active_tools: activeTools,
    });
  }
  deleteState(id: string) {
    this.infos.delete(id);
  }
  listAgentInfos() {
    return [...this.infos.values()];
  }
  appendEvent(_id: string, record: EventRecord) {
    this.events.push(record);
  }
}

const runtime = (env: Record<string, string> = {}): Runtime => ({
  env: (n) => env[n],
  now: () => new Date("2026-01-01T00:00:00.000Z"),
  stdoutIsTerminal: false,
  version: "9.9.9",
  logDir: () => "/home/u/logs",
  currentLogFile: (d) => `${d}/agentctl.log.2026-01-01`,
  tildeCollapse: (p) => p.replace("/home/u", "~"),
});

const tmux: Tmux = { listPanes: () => [], processTable: () => new Map() };

function setup(env: Record<string, string> = {}, harnesses: Harness[] = []) {
  const repo = new MemoryRepo();
  return {
    repo,
    app: new App({ repo, tmux, harnesses, runtime: runtime(env) }),
  };
}

const dispatch = (
  app: App,
  kind: Parameters<App["dispatchEvent"]>[0],
  id = "x",
  type?: "custom" | "elicitation",
  pane = "%1",
) => app.dispatchEvent(kind, id, type, "pi", pane);

describe("dispatchEvent", () => {
  it("SESSION_STARTED writes idle and logs the event", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "SESSION_STARTED" }, "s1");
    expect(repo.infos.get("%1")?.status).toBe("idle");
    expect(repo.events).toEqual([
      {
        event: "SESSION_STARTED",
        id: "s1",
        harness: "pi",
        pane_id: "%1",
        timestamp: "2026-01-01T00:00:00Z",
      },
    ]);
  });

  it("RUN_STARTED moves idle to working", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "SESSION_STARTED" });
    dispatch(app, { kind: "RUN_STARTED" });
    expect(repo.infos.get("%1")?.status).toBe("working");
  });

  it("is a silent no-op without a pane", () => {
    const { repo, app } = setup();
    app.dispatchEvent(
      { kind: "SESSION_STARTED" },
      "s1",
      undefined,
      undefined,
      undefined,
    );
    expect(repo.infos.size).toBe(0);
  });

  it("falls back to TMUX_PANE; --pane overrides it", () => {
    const { repo, app } = setup({ TMUX_PANE: "%7" });
    app.dispatchEvent(
      { kind: "SESSION_STARTED" },
      "s1",
      undefined,
      undefined,
      undefined,
    );
    app.dispatchEvent(
      { kind: "SESSION_STARTED" },
      "s1",
      undefined,
      undefined,
      "%8",
    );
    expect([...repo.infos.keys()].sort()).toEqual(["%7", "%8"]);
  });

  it("tool calls increment and decrement (saturating) active_tools", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "TOOL_CALL_RESULT" });
    expect(repo.infos.get("%1")?.active_tools).toBe(0);
    for (const id of ["t1", "t2", "t3"])
      dispatch(app, { kind: "TOOL_CALL_START" }, id);
    expect(repo.infos.get("%1")?.active_tools).toBe(3);
    dispatch(app, { kind: "TOOL_CALL_RESULT" });
    expect(repo.infos.get("%1")?.active_tools).toBe(2);
  });

  it("SESSION_STARTED resets active_tools", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "TOOL_CALL_START" });
    dispatch(app, { kind: "SESSION_STARTED" });
    expect(repo.infos.get("%1")?.active_tools).toBe(0);
  });

  it("elicitation blocks status and increments active_tools", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "SESSION_STARTED" });
    dispatch(app, { kind: "RUN_STARTED" });
    dispatch(app, { kind: "TOOL_CALL_START" }, "t1", "elicitation");
    const i = repo.infos.get("%1");
    expect(i?.status).toBe("blocked");
    expect(i?.active_tools).toBe(1);
    expect(repo.events.at(-1)?.type).toBe("elicitation");
  });

  it("RUN_FINISHED overrides or preserves active_tools and drives done", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "TOOL_CALL_START" });
    dispatch(app, { kind: "TOOL_CALL_START" });
    dispatch(app, { kind: "RUN_FINISHED" });
    expect(repo.infos.get("%1")).toMatchObject({
      status: "done",
      active_tools: 2,
    });
    dispatch(app, { kind: "RUN_FINISHED", activeTools: 0 });
    expect(repo.infos.get("%1")?.active_tools).toBe(0);
  });

  it("TOOL_CALL_START on a done pane moves to working", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "RUN_FINISHED" });
    dispatch(app, { kind: "TOOL_CALL_START" });
    expect(repo.infos.get("%1")).toMatchObject({
      status: "working",
      active_tools: 1,
    });
  });
});

describe("clear", () => {
  it("removes state; absent or no pane is a no-op", () => {
    const { repo, app } = setup();
    dispatch(app, { kind: "SESSION_STARTED" });
    app.clear("%1");
    expect(repo.infos.size).toBe(0);
    app.clear("%99");
    app.clear(undefined);
  });
});

describe("install / uninstall / info", () => {
  const fake: Harness = {
    name: "fake",
    install: () => "installed fake",
    uninstall: () => "removed fake",
  };

  it("dispatches to the matching harness by name", () => {
    const { app } = setup({}, [fake]);
    expect(app.install("fake")).toBe("installed fake");
    expect(app.uninstall("fake")).toBe("removed fake");
  });

  it("unknown harness is an error", () => {
    const { app } = setup();
    expect(() => app.install("nope")).toThrow("unknown harness: nope");
  });

  it("info reports version, log level and tilde-collapsed paths", () => {
    const { app } = setup();
    expect(app.info("debug")).toBe(
      "version: 9.9.9\nlog level: debug\nlog directory: ~/logs\ncurrent log file: ~/logs/agentctl.log.2026-01-01\n",
    );
  });
});
