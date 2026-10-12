import { describe, expect, it } from "vitest";
import type { AgentInfo, AgentState, PaneEntry } from "./domain.ts";
import {
  colorizeState,
  DEFAULT_COLUMNS,
  DEFAULT_COLUMNS_STR,
  type Field,
  formatPanes,
  type StateMap,
  sortPanes,
} from "./list.ts";

const pane = (
  agent: string,
  session: string,
  w: number,
  p: number,
  id: string,
): PaneEntry => ({
  agent,
  session,
  windowIndex: w,
  paneIndex: p,
  panePid: 0,
  paneId: id,
});

const info = (
  id: string,
  status: AgentState,
  tools = 0,
  updated = "2026-01-01T00:00:00Z",
): AgentInfo => ({
  pane_id: id,
  harness: "pi",
  status,
  updated_at: updated,
  active_tools: tools,
});

const map = (...infos: AgentInfo[]): StateMap =>
  new Map(infos.map((i) => [i.pane_id, i]));

const ids = (panes: PaneEntry[]) => panes.map((p) => p.paneId);

describe("formatPanes", () => {
  it("empty input returns empty string", () => {
    expect(formatPanes([], map(), false, DEFAULT_COLUMNS)).toBe("");
  });

  it("handles wide unicode with tabs", () => {
    const out = formatPanes(
      [
        pane("pi", "dotfiles", 0, 0, "%1"),
        pane("pi", "🌳 agentctl (list-sort)", 0, 0, "%3"),
      ],
      map(),
      false,
      DEFAULT_COLUMNS,
    );
    expect(out.split("\n")).toEqual([
      "pi\tdotfiles\t0.0\tunknown\t",
      "pi\t🌳 agentctl (list-sort)\t0.0\tunknown\t",
      "",
    ]);
  });

  it("shows status from the state map", () => {
    const out = formatPanes(
      [pane("claude", "s", 1, 2, "%1")],
      map(info("%1", "working")),
      false,
      DEFAULT_COLUMNS,
    );
    expect(out).toBe("claude\ts\t1.2\tworking\t\n");
  });

  it("renders tool counts singular/plural/empty", () => {
    const render = (n: number) =>
      formatPanes(
        [pane("pi", "s", 0, 0, "%1")],
        map(info("%1", "working", n)),
        false,
        DEFAULT_COLUMNS,
      );
    expect(render(0)).toBe("pi\ts\t0.0\tworking\t\n");
    expect(render(1)).toBe("pi\ts\t0.0\tworking\t1 tool\n");
    expect(render(3)).toBe("pi\ts\t0.0\tworking\t3 tools\n");
  });

  it("unknown status never shows a tool count", () => {
    const out = formatPanes(
      [pane("pi", "s", 0, 0, "%1")],
      map(info("%1", "unknown", 4)),
      false,
      DEFAULT_COLUMNS,
    );
    expect(out).toBe("pi\ts\t0.0\tunknown\t\n");
  });

  it("DEFAULT_COLUMNS_STR matches DEFAULT_COLUMNS", () => {
    expect(DEFAULT_COLUMNS_STR.split(",")).toEqual([...DEFAULT_COLUMNS]);
  });

  it("supports subsets, custom order, and duplicates", () => {
    const p = [pane("pi", "sess", 3, 4, "%1")];
    const m = map(info("%1", "idle"));
    expect(formatPanes(p, m, false, ["status", "agent"])).toBe("idle\tpi\n");
    expect(formatPanes(p, m, false, ["agent"])).toBe("pi\n");
    expect(formatPanes(p, m, false, ["agent", "agent"])).toBe("pi\tpi\n");
  });

  it("updatedAt renders RFC 3339 with +00:00, or empty without state", () => {
    const p = [pane("pi", "s", 0, 0, "%1")];
    expect(
      formatPanes(
        p,
        map(info("%1", "idle", 0, "2026-01-01T00:00:00Z")),
        false,
        ["updatedAt"],
      ),
    ).toBe("2026-01-01T00:00:00+00:00\n");
    expect(formatPanes(p, map(), false, ["agent", "updatedAt"])).toBe("pi\t\n");
  });

  it("keeps a fixed field count with empty trailing columns", () => {
    const cols: Field[] = ["agent", "activeTools", "updatedAt"];
    const out = formatPanes([pane("pi", "s", 0, 0, "%1")], map(), false, cols);
    expect(out).toBe("pi\t\t\n");
  });

  it("colors every status column when enabled", () => {
    const out = formatPanes(
      [pane("pi", "s", 0, 0, "%1")],
      map(info("%1", "working")),
      true,
      ["status", "agent", "status"],
    );
    expect(out).toBe("\x1b[33mworking\x1b[39m\tpi\t\x1b[33mworking\x1b[39m\n");
  });
});

describe("colorizeState", () => {
  it("returns plain text when color is off", () => {
    for (const s of [
      "idle",
      "working",
      "done",
      "blocked",
      "unknown",
    ] as const) {
      expect(colorizeState(s, s, false)).toBe(s);
    }
  });

  it("maps states to ANSI colors", () => {
    expect(colorizeState("idle", "x", true)).toBe("\x1b[90mx\x1b[39m");
    expect(colorizeState("working", "x", true)).toBe("\x1b[33mx\x1b[39m");
    expect(colorizeState("blocked", "x", true)).toBe("\x1b[31mx\x1b[39m");
    expect(colorizeState("done", "x", true)).toBe("\x1b[32mx\x1b[39m");
    expect(colorizeState("unknown", "x", true)).toBe("x");
  });
});

describe("sortPanes", () => {
  const panes = () => [
    pane("pi", "b", 1, 0, "%1"),
    pane("claude", "a", 0, 2, "%2"),
    pane("pi", "c", 0, 1, "%3"),
  ];

  it("updatedAt: newest first, missing state last, reverse flips", () => {
    const m = map(
      info("%1", "idle", 0, "2026-01-01T00:00:10Z"),
      info("%2", "idle", 0, "2026-01-01T00:00:30Z"),
    );
    const p = panes();
    sortPanes(p, "updatedAt", false, m);
    expect(ids(p)).toEqual(["%2", "%1", "%3"]);
    sortPanes(p, "updatedAt", true, m);
    expect(ids(p)).toEqual(["%3", "%1", "%2"]);
  });

  it("agent / session / pane ascending, reverse flips", () => {
    const run = (key: Field, reverse = false) => {
      const p = panes();
      sortPanes(p, key, reverse, map());
      return ids(p);
    };
    expect(run("agent")).toEqual(["%2", "%1", "%3"]);
    expect(run("session")).toEqual(["%2", "%1", "%3"]);
    expect(run("pane")).toEqual(["%3", "%2", "%1"]);
    expect(run("agent", true)).toEqual(["%3", "%1", "%2"]);
  });

  it("status: blocked first, unknown last", () => {
    const m = map(info("%1", "idle"), info("%2", "blocked"));
    const p = panes();
    sortPanes(p, "status", false, m);
    expect(ids(p)).toEqual(["%2", "%1", "%3"]);
    sortPanes(p, "status", true, m);
    expect(ids(p)).toEqual(["%3", "%1", "%2"]);
  });

  it("activeTools: busiest first; unknown status counts as 0", () => {
    const m = map(
      info("%1", "working", 1),
      info("%2", "working", 5),
      info("%3", "unknown", 9),
    );
    const p = panes();
    sortPanes(p, "activeTools", false, m);
    expect(ids(p)).toEqual(["%2", "%1", "%3"]);
    sortPanes(p, "activeTools", true, m);
    expect(ids(p)).toEqual(["%3", "%1", "%2"]);
  });

  it("is stable for equal keys", () => {
    const p = [
      pane("pi", "s", 0, 0, "%1"),
      pane("pi", "s", 0, 0, "%2"),
      pane("pi", "s", 0, 0, "%3"),
    ];
    sortPanes(p, "agent", false, map());
    expect(ids(p)).toEqual(["%1", "%2", "%3"]);
  });
});
