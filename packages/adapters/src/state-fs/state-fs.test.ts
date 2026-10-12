import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { makeTmp, readFixture } from "../test-support.ts";
import { AgentInfoSchema, FileAgentRepository, xdgDataHome } from "./index.ts";

const repo = () => {
  const base = makeTmp();
  return {
    base,
    r: new FileAgentRepository(base, () => "2026-01-01T00:00:00.123456Z"),
  };
};

describe("FileAgentRepository", () => {
  it("write/read round trip and overwrite", () => {
    const { r } = repo();
    r.writeStateWithTools("%12", "claude", "working", 0);
    expect(r.readAgentInfo("%12")?.status).toBe("working");
    r.writeStateWithTools("%12", "claude", "done", 2);
    expect(r.readAgentInfo("%12")).toEqual({
      pane_id: "%12",
      harness: "claude",
      status: "done",
      updated_at: "2026-01-01T00:00:00.123456Z",
      active_tools: 2,
    });
  });

  it("serialises state.json exactly like serde_json::to_string_pretty", () => {
    const { base, r } = repo();
    r.writeStateWithTools("%42", "claude", "blocked", 1);
    expect(
      readFileSync(join(base, "agentctl/agents/%42/state.json"), "utf8"),
    ).toBe(
      `{\n  "pane_id": "%42",\n  "harness": "claude",\n  "status": "blocked",\n  "updated_at": "2026-01-01T00:00:00.123456Z",\n  "active_tools": 1\n}`,
    );
    expect(existsSync(join(base, "agentctl/agents/%42/state.json.tmp"))).toBe(
      false,
    );
  });

  it("read returns undefined for missing or corrupt files", () => {
    const { base, r } = repo();
    expect(r.readAgentInfo("%1")).toBeUndefined();
    mkdirSync(join(base, "agentctl/agents/%1"), { recursive: true });
    writeFileSync(join(base, "agentctl/agents/%1/state.json"), "not json");
    expect(r.readAgentInfo("%1")).toBeUndefined();
    writeFileSync(
      join(base, "agentctl/agents/%1/state.json"),
      '{"pane_id":"%1"}',
    );
    expect(r.readAgentInfo("%1")).toBeUndefined();
  });

  it("delete removes the whole dir and is a no-op when absent", () => {
    const { base, r } = repo();
    r.writeStateWithTools("%1", "pi", "idle", 0);
    r.appendEvent("%1", {
      event: "E",
      id: "1",
      harness: "pi",
      pane_id: "%1",
      timestamp: "t",
    });
    r.deleteState("%1");
    expect(existsSync(join(base, "agentctl/agents/%1"))).toBe(false);
    r.deleteState("%1");
  });

  it("lists all valid infos, skipping invalid ones", () => {
    const { base, r } = repo();
    r.writeStateWithTools("%1", "pi", "idle", 0);
    r.writeStateWithTools("%2", "claude", "working", 1);
    mkdirSync(join(base, "agentctl/agents/%3"), { recursive: true });
    writeFileSync(join(base, "agentctl/agents/%3/state.json"), "{");
    expect(
      r
        .listAgentInfos()
        .map((i) => i.pane_id)
        .sort(),
    ).toEqual(["%1", "%2"]);
    expect(new FileAgentRepository(makeTmp()).listAgentInfos()).toEqual([]);
  });

  it("appends events as JSON lines", () => {
    const { base, r } = repo();
    r.appendEvent("%7", {
      event: "SESSION_STARTED",
      id: "s1",
      harness: "claude",
      pane_id: "%7",
      timestamp: "2026-01-01T00:00:00Z",
    });
    r.appendEvent("%7", {
      event: "TOOL_CALL_START",
      id: "t1",
      type: "custom",
      harness: "claude",
      pane_id: "%7",
      timestamp: "2026-01-01T00:00:01Z",
    });
    const lines = readFileSync(
      join(base, "agentctl/agents/%7/events.jsonl"),
      "utf8",
    ).split("\n");
    expect(lines[0]).toBe(
      '{"event":"SESSION_STARTED","id":"s1","harness":"claude","pane_id":"%7","timestamp":"2026-01-01T00:00:00Z"}',
    );
    expect(JSON.parse(lines[1] as string).type).toBe("custom");
    expect(lines[2]).toBe("");
  });
});

describe("AgentInfoSchema (compatibility with Rust-written files)", () => {
  it("parses Rust-written state.json fixtures", () => {
    const blocked = AgentInfoSchema.parse(
      JSON.parse(readFixture("xdg/agentctl/agents/%42/state.json")),
    );
    expect(blocked).toMatchObject({
      pane_id: "%42",
      harness: "claude",
      status: "blocked",
      active_tools: 1,
    });
    expect(blocked.updated_at).toMatch(
      /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{6}Z$/,
    );
    const pi = AgentInfoSchema.parse(
      JSON.parse(readFixture("xdg/agentctl/agents/%43/state.json")),
    );
    expect(pi).toMatchObject({
      harness: "pi",
      status: "idle",
      active_tools: 1,
    });
  });

  it("round-trips Rust fixtures byte for byte", () => {
    const { r } = repo();
    for (const pane of ["%42", "%43"]) {
      const raw = readFixture(`xdg/agentctl/agents/${pane}/state.json`);
      const info = AgentInfoSchema.parse(JSON.parse(raw));
      expect(JSON.stringify(info, null, 2)).toBe(raw);
    }
    expect(r).toBeDefined();
  });

  it("defaults active_tools to 0, ignores unknown fields, normalises offsets", () => {
    const parsed = AgentInfoSchema.parse({
      pane_id: "%1",
      harness: "pi",
      status: "idle",
      updated_at: "2026-01-01T02:00:00+02:00",
      extra: true,
    });
    expect(parsed.active_tools).toBe(0);
    expect(parsed.updated_at).toBe("2026-01-01T00:00:00Z");
  });

  it("rejects bad status, timestamp and negative counts", () => {
    const base = {
      pane_id: "%1",
      harness: "pi",
      status: "idle",
      updated_at: "2026-01-01T00:00:00Z",
    };
    expect(
      AgentInfoSchema.safeParse({ ...base, status: "sleeping" }).success,
    ).toBe(false);
    expect(
      AgentInfoSchema.safeParse({ ...base, updated_at: "yesterday" }).success,
    ).toBe(false);
    expect(
      AgentInfoSchema.safeParse({ ...base, active_tools: -1 }).success,
    ).toBe(false);
  });

  it("parses the minimal state file the e2e helpers write", () => {
    const info = AgentInfoSchema.parse({
      pane_id: "%1",
      harness: "claude",
      status: "working",
      updated_at: "2026-01-01T00:00:00Z",
    });
    expect(info.active_tools).toBe(0);
  });
});

describe("xdgDataHome", () => {
  it("prefers non-empty XDG_DATA_HOME, else ~/.local/share", () => {
    expect(xdgDataHome({ XDG_DATA_HOME: "/x" }, "/h")).toBe("/x");
    expect(xdgDataHome({ XDG_DATA_HOME: "", HOME: "/h" }, "/h")).toBe(
      "/h/.local/share",
    );
  });
});
