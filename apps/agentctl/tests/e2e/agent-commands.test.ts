import { existsSync, readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { useTmuxEnv } from "./support/tmux-env.ts";

const read = (p: string) => readFileSync(p, "utf8");

describe("agentctl agent event", () => {
  const env = useTmuxEnv();

  it("RUN_STARTED writes state file", () => {
    const e = env();
    // A real harness dispatches SESSION_STARTED before RUN_STARTED.
    const session = e.agentctl([
      "agent",
      "event",
      "SESSION_STARTED",
      "--id",
      "s1",
      "--harness",
      "claude",
      "--pane",
      "%42",
    ]);
    expect(session.status).toBe(0);

    const out = e.agentctl([
      "agent",
      "event",
      "RUN_STARTED",
      "--id",
      "r1",
      "--harness",
      "claude",
      "--pane",
      "%42",
    ]);
    expect(out.status, out.stderr).toBe(0);

    expect(existsSync(e.stateFilePath("%42"))).toBe(true);
    const contents = read(e.stateFilePath("%42"));
    expect(contents).toContain("working");
    expect(contents).toContain("claude");

    expect(existsSync(e.eventsFilePath("%42"))).toBe(true);
    expect(read(e.eventsFilePath("%42"))).toContain("RUN_STARTED");
  });

  it("SESSION_STARTED defaults harness to unknown", () => {
    const e = env();
    const out = e.agentctl([
      "agent",
      "event",
      "SESSION_STARTED",
      "--id",
      "s1",
      "--pane",
      "%10",
    ]);
    expect(out.status).toBe(0);
    const contents = read(e.stateFilePath("%10"));
    expect(contents).toContain('"unknown"');
    expect(contents).toContain("idle");
  });

  it("exits silently when no pane", () => {
    const e = env();
    const out = e.agentctl(
      ["agent", "event", "SESSION_STARTED", "--id", "s1"],
      {
        env: { TMUX_PANE: undefined },
      },
    );
    expect(out.status).toBe(0);
    expect(out.stdout).toBe("");
    expect(out.stderr).toBe("");
    const agentsDir = join(e.xdgPath, "agentctl", "agents");
    expect(!existsSync(agentsDir) || readdirSync(agentsDir).length === 0).toBe(
      true,
    );
  });

  it("unknown event name exits non-zero", () => {
    const out = env().agentctl([
      "agent",
      "event",
      "FLYING",
      "--id",
      "x",
      "--pane",
      "%1",
    ]);
    expect(out.status).not.toBe(0);
  });

  it("--type on RUN_STARTED exits non-zero", () => {
    const out = env().agentctl([
      "agent",
      "event",
      "RUN_STARTED",
      "--id",
      "r1",
      "--type",
      "custom",
      "--pane",
      "%1",
    ]);
    expect(out.status).not.toBe(0);
  });

  it("missing --id exits non-zero and writes nothing", () => {
    const e = env();
    const out = e.agentctl(["agent", "event", "RUN_STARTED", "--pane", "%77"]);
    expect(out.status).not.toBe(0);
    expect(existsSync(e.stateFilePath("%77"))).toBe(false);
  });

  it("full lifecycle transitions through expected statuses", () => {
    const e = env();
    const pane = "%55";
    const dispatch = (...args: string[]) =>
      e.agentctl(["agent", "event", ...args, "--pane", pane]);

    expect(dispatch("SESSION_STARTED", "--id", "s1").status).toBe(0);
    expect(read(e.stateFilePath(pane))).toContain("idle");

    expect(dispatch("RUN_STARTED", "--id", "r1").status).toBe(0);
    expect(read(e.stateFilePath(pane))).toContain("working");

    expect(
      dispatch("TOOL_CALL_START", "--id", "t1", "--type", "elicitation").status,
    ).toBe(0);
    expect(read(e.stateFilePath(pane))).toContain("blocked");

    expect(
      dispatch("TOOL_CALL_RESULT", "--id", "t1", "--type", "elicitation")
        .status,
    ).toBe(0);
    expect(read(e.stateFilePath(pane))).toContain("working");

    expect(dispatch("RUN_FINISHED", "--id", "r1").status).toBe(0);
    expect(read(e.stateFilePath(pane))).toContain("done");
  });
});

describe("agentctl agent clear", () => {
  const env = useTmuxEnv();

  it("removes existing state file", () => {
    const e = env();
    const w = e.agentctl([
      "agent",
      "event",
      "RUN_FINISHED",
      "--id",
      "r1",
      "--pane",
      "%42",
    ]);
    expect(w.status).toBe(0);
    expect(existsSync(e.stateFilePath("%42"))).toBe(true);

    const out = e.agentctl(["agent", "clear", "--pane", "%42"]);
    expect(out.status, out.stderr).toBe(0);
    expect(existsSync(e.stateFilePath("%42"))).toBe(false);
    expect(existsSync(e.eventsFilePath("%42"))).toBe(false);
  });

  it("absent file exits silently", () => {
    expect(env().agentctl(["agent", "clear", "--pane", "%99"]).status).toBe(0);
  });

  it("no pane exits silently", () => {
    const out = env().agentctl(["agent", "clear"], {
      env: { TMUX_PANE: undefined },
    });
    expect(out.status).toBe(0);
  });
});
