import type { App, EventKind } from "@agentctl/core";
import { CommanderError } from "commander";
import { describe, expect, it } from "vitest";
import { LONG_HELP } from "./help.ts";
import { buildProgram } from "./program.ts";

interface Call {
  method: string;
  args: unknown[];
}

function harness() {
  const calls: Call[] = [];
  const out: string[] = [];
  const rec =
    (method: string, ret?: string) =>
    (...args: unknown[]) => {
      calls.push({ method, args });
      return ret;
    };
  const app = {
    list: rec("list", "LIST\n"),
    install: rec("install", "INSTALLED"),
    uninstall: rec("uninstall", "UNINSTALLED"),
    dispatchEvent: rec("dispatchEvent"),
    clear: rec("clear"),
    info: rec("info", "INFO\n"),
  } as unknown as App;
  const levels: string[] = [];
  const program = buildProgram(
    {
      createApp: (level) => {
        levels.push(level);
        return app;
      },
      write: (t) => out.push(t),
    },
    ["agentctl"],
  );
  // Keep usage errors from writing to the test's stderr.
  const silence = (cmd: typeof program) => {
    cmd.configureOutput({ writeErr: () => {}, writeOut: () => {} });
    for (const c of cmd.commands) silence(c);
  };
  silence(program);
  const run = (...args: string[]) =>
    program.parse(["node", "agentctl", ...args]);
  return { run, calls, out, levels };
}

const rejects = (...args: string[]) => {
  const h = harness();
  expect(() => h.run(...args)).toThrow(CommanderError);
  expect(h.calls).toEqual([]);
};

describe("list", () => {
  it("passes defaults to App.list", () => {
    const h = harness();
    h.run("list");
    expect(h.calls[0]).toEqual({
      method: "list",
      args: [
        undefined,
        "updatedAt",
        false,
        ["agent", "session", "pane", "status", "activeTools"],
      ],
    });
    expect(h.out).toEqual(["LIST\n"]);
    expect(h.levels).toEqual(["info"]);
  });

  it("supports the ls alias and all flags", () => {
    const h = harness();
    h.run(
      "ls",
      "-s",
      "/tmp/x.sock",
      "--sort-by",
      "status",
      "-r",
      "-c",
      "status,agent",
      "-c",
      "pane",
      "--log-level",
      "debug",
    );
    expect(h.calls[0]?.args).toEqual([
      "/tmp/x.sock",
      "status",
      true,
      ["status", "agent", "pane"],
    ]);
    expect(h.levels).toEqual(["debug"]);
  });

  it("rejects invalid --columns and --sort-by values", () => {
    rejects("list", "--columns", "bogus");
    rejects("list", "--sort-by", "foobar");
    rejects("list", "--sort-by", "state");
    rejects("list", "--sort-by", "updated-at");
  });

  it("columns and sort-by share an identical vocabulary", () => {
    for (const v of [
      "agent",
      "session",
      "pane",
      "status",
      "updatedAt",
      "activeTools",
    ]) {
      const h = harness();
      h.run("list", "--sort-by", v);
      h.run("list", "--columns", v);
      expect(h.calls).toHaveLength(2);
    }
  });
});

describe("install / uninstall", () => {
  it("dispatch to the app and print the summary", () => {
    const h = harness();
    h.run("install", "pi");
    h.run("uninstall", "claude");
    expect(h.calls.map((c) => [c.method, c.args[0]])).toEqual([
      ["install", "pi"],
      ["uninstall", "claude"],
    ]);
    expect(h.out).toEqual(["INSTALLED\n", "UNINSTALLED\n"]);
  });

  it("rejects unknown harnesses", () => {
    rejects("install", "aider");
    rejects("uninstall", "nope");
    rejects("install");
  });
});

describe("agent event", () => {
  const event = (h: ReturnType<typeof harness>) =>
    h.calls[0]?.args as [
      EventKind,
      string,
      string | undefined,
      string | undefined,
      string | undefined,
    ];

  it("maps each event with --harness/--pane before or after the event name", () => {
    const h = harness();
    h.run(
      "agent",
      "event",
      "--harness",
      "claude",
      "RUN_STARTED",
      "--id",
      "r1",
      "--pane",
      "%4",
    );
    expect(event(h)).toEqual([
      { kind: "RUN_STARTED" },
      "r1",
      undefined,
      "claude",
      "%4",
    ]);
  });

  it("parses --type and --active-tools", () => {
    const h = harness();
    h.run(
      "agent",
      "event",
      "TOOL_CALL_START",
      "--id",
      "t",
      "--type",
      "elicitation",
    );
    expect(event(h)).toEqual([
      { kind: "TOOL_CALL_START" },
      "t",
      "elicitation",
      undefined,
      undefined,
    ]);
    const h2 = harness();
    h2.run(
      "agent",
      "event",
      "RUN_FINISHED",
      "--id",
      "r",
      "--active-tools",
      "3",
    );
    expect(event(h2)[0]).toEqual({ kind: "RUN_FINISHED", activeTools: 3 });
  });

  it("requires --id and rejects bad input", () => {
    rejects("agent", "event", "RUN_STARTED");
    rejects("agent", "event", "FLYING", "--id", "x");
    rejects("agent", "event", "RUN_STARTED", "--id", "r", "--type", "custom");
    rejects(
      "agent",
      "event",
      "TOOL_CALL_START",
      "--id",
      "r",
      "--type",
      "weird",
    );
    rejects(
      "agent",
      "event",
      "RUN_FINISHED",
      "--id",
      "r",
      "--active-tools",
      "x",
    );
    rejects(
      "agent",
      "event",
      "RUN_FINISHED",
      "--id",
      "r",
      "--active-tools",
      "-1",
    );
  });
});

describe("misc commands", () => {
  it("agent clear forwards --pane", () => {
    const h = harness();
    h.run("agent", "clear", "--pane", "%2");
    expect(h.calls[0]).toEqual({ method: "clear", args: ["%2"] });
  });

  it("info passes the effective log level", () => {
    const h = harness();
    h.run("info", "--log-level", "warn");
    expect(h.calls[0]).toEqual({ method: "info", args: ["warn"] });
    expect(h.out).toEqual(["INFO\n"]);
  });

  it("help and no-arg print the long help without creating an app", () => {
    const h = harness();
    h.run("help");
    h.run();
    expect(h.out).toEqual([LONG_HELP, LONG_HELP]);
    expect(h.levels).toEqual([]);
  });

  it("rejects invalid --log-level", () => {
    rejects("info", "--log-level", "loud");
  });
});
