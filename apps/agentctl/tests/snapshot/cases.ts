import { FIELDS } from "@agentctl/core";
import type { CliCase } from "./support/run-cli.ts";

type Case = Omit<CliCase, "group">;
const group = (g: string, cases: Case[]): CliCase[] =>
  cases.map((c) => ({ group: g, ...c }));

const pane = { TMUX_PANE: "%1" };

const HELP_TARGETS: string[][] = [
  ["list"],
  ["ls"],
  ["install"],
  ["uninstall"],
  ["agent"],
  ["agent", "event"],
  ["agent", "event", "SESSION_STARTED"],
  ["agent", "event", "RUN_STARTED"],
  ["agent", "event", "RUN_FINISHED"],
  ["agent", "event", "TOOL_CALL_START"],
  ["agent", "event", "TOOL_CALL_RESULT"],
  ["agent", "clear"],
  ["info"],
];

const COLUMN_SETS = [
  "agent",
  "status,agent",
  "pane,session",
  "updatedAt",
  "activeTools,status",
  "agent,agent",
];

export const CASES: CliCase[] = [
  ...group("root", [
    { name: "no-args", args: [] },
    { name: "help-command", args: ["help"] },
    { name: "help-long-flag", args: ["--help"] },
    { name: "help-short-flag", args: ["-h"] },
    { name: "version-short", args: ["-V"] },
    { name: "version-long", args: ["--version"] },
    { name: "help-command-with-topic", args: ["help", "list"] },
    { name: "unknown-command", args: ["bogus"] },
    { name: "unknown-option", args: ["--bogus"] },
  ]),

  ...group(
    "help",
    HELP_TARGETS.map((t) => ({
      name: t.join("-"),
      args: [...t, "--help"],
    })),
  ),

  ...group("list", [
    { name: "empty", args: ["list"], world: "empty" },
    { name: "default", args: ["list"], world: "mixed" },
    { name: "alias-ls", args: ["ls"], world: "mixed" },
    { name: "tty-colors", args: ["list"], world: "mixed", tty: true },
    { name: "tmux-missing", args: ["list"], world: "tmuxMissing" },
    ...FIELDS.flatMap((f) => [
      {
        name: `sort-by-${f}`,
        args: ["list", "--sort-by", f],
        world: "mixed" as const,
      },
      {
        name: `sort-by-${f}-reverse`,
        args: ["list", "--sort-by", f, "-r"],
        world: "mixed" as const,
      },
    ]),
    { name: "reverse-default", args: ["list", "--reverse"], world: "mixed" },
    ...COLUMN_SETS.map((c) => ({
      name: `columns-${c.replace(/,/g, "-")}`,
      args: ["list", "--columns", c],
      world: "mixed" as const,
    })),
    {
      name: "columns-repeated-flag",
      args: ["list", "-c", "agent", "-c", "status"],
      world: "mixed",
    },
    {
      name: "columns-tty-status",
      args: ["list", "-c", "status,agent"],
      world: "mixed",
      tty: true,
    },
    { name: "server-flag", args: ["list", "-s", "/tmp/tmux.sock"] },
    {
      name: "server-env",
      args: ["list"],
      env: { AGENTCTL_TMUX_SOCKET: "/tmp/env.sock" },
    },
    {
      name: "server-flag-beats-env",
      args: ["list", "--server", "/tmp/flag.sock"],
      env: { AGENTCTL_TMUX_SOCKET: "/tmp/env.sock" },
    },
    {
      name: "combined-options",
      args: [
        "list",
        "-s",
        "/tmp/s",
        "--sort-by",
        "status",
        "-r",
        "-c",
        "pane,status",
      ],
      world: "mixed",
    },
    { name: "invalid-sort-by", args: ["list", "--sort-by", "bogus"] },
    { name: "invalid-column", args: ["list", "-c", "agent,bogus"] },
    { name: "missing-server-value", args: ["list", "-s"] },
  ]),

  ...group("install", [
    { name: "install-pi", args: ["install", "pi"] },
    { name: "install-claude", args: ["install", "claude"] },
    { name: "uninstall-pi", args: ["uninstall", "pi"] },
    { name: "uninstall-claude", args: ["uninstall", "claude"] },
    { name: "install-invalid-harness", args: ["install", "bogus"] },
    { name: "install-missing-harness", args: ["install"] },
    { name: "uninstall-invalid-harness", args: ["uninstall", "bogus"] },
    { name: "uninstall-missing-harness", args: ["uninstall"] },
  ]),

  ...group("agent-event", [
    {
      name: "session-started",
      args: ["agent", "event", "SESSION_STARTED", "--id", "s1"],
      world: "working",
      env: pane,
    },
    {
      name: "session-started-new-pane-with-harness",
      args: [
        "agent",
        "event",
        "--harness",
        "claude",
        "SESSION_STARTED",
        "--id",
        "s1",
        "--pane",
        "%8",
      ],
    },
    {
      name: "run-started",
      args: ["agent", "event", "RUN_STARTED", "--id", "r1"],
      world: "working",
      env: pane,
    },
    {
      name: "run-started-pane-flag",
      args: ["agent", "event", "--pane", "%1", "RUN_STARTED", "--id", "r1"],
      world: "working",
    },
    {
      name: "run-finished",
      args: ["agent", "event", "RUN_FINISHED", "--id", "r1"],
      world: "working",
      env: pane,
    },
    {
      name: "run-finished-active-tools",
      args: [
        "agent",
        "event",
        "RUN_FINISHED",
        "--id",
        "r1",
        "--active-tools",
        "3",
      ],
      world: "working",
      env: pane,
    },
    {
      name: "run-finished-active-tools-invalid",
      args: [
        "agent",
        "event",
        "RUN_FINISHED",
        "--id",
        "r1",
        "--active-tools",
        "abc",
      ],
      env: pane,
    },
    {
      name: "run-finished-active-tools-negative",
      args: [
        "agent",
        "event",
        "RUN_FINISHED",
        "--id",
        "r1",
        "--active-tools",
        "-1",
      ],
      env: pane,
    },
    {
      name: "tool-call-start",
      args: ["agent", "event", "TOOL_CALL_START", "--id", "t1"],
      world: "working",
      env: pane,
    },
    {
      name: "tool-call-start-custom",
      args: [
        "agent",
        "event",
        "TOOL_CALL_START",
        "--id",
        "t1",
        "--type",
        "custom",
      ],
      world: "working",
      env: pane,
    },
    {
      name: "tool-call-start-elicitation",
      args: [
        "agent",
        "event",
        "TOOL_CALL_START",
        "--id",
        "t1",
        "--type",
        "elicitation",
      ],
      world: "working",
      env: pane,
    },
    {
      name: "tool-call-start-invalid-type",
      args: [
        "agent",
        "event",
        "TOOL_CALL_START",
        "--id",
        "t1",
        "--type",
        "bogus",
      ],
      env: pane,
    },
    {
      name: "tool-call-result",
      args: ["agent", "event", "TOOL_CALL_RESULT", "--id", "t1"],
      world: "working",
      env: pane,
    },
    {
      name: "tool-call-result-elicitation-unblocks",
      args: [
        "agent",
        "event",
        "TOOL_CALL_RESULT",
        "--id",
        "t1",
        "--type",
        "elicitation",
      ],
      world: "blocked",
      env: pane,
    },
    {
      name: "no-pane-is-noop",
      args: ["agent", "event", "RUN_STARTED", "--id", "r1"],
    },
    {
      name: "missing-id",
      args: ["agent", "event", "RUN_STARTED"],
      env: pane,
    },
    {
      name: "unknown-event",
      args: ["agent", "event", "BOGUS", "--id", "x"],
    },
  ]),

  ...group("agent-clear", [
    { name: "pane-env", args: ["agent", "clear"], world: "working", env: pane },
    {
      name: "pane-flag",
      args: ["agent", "clear", "--pane", "%1"],
      world: "working",
    },
    {
      name: "pane-without-state",
      args: ["agent", "clear", "--pane", "%99"],
      world: "working",
    },
    { name: "no-pane-is-noop", args: ["agent", "clear"], world: "working" },
  ]),

  ...group("info", [
    { name: "default", args: ["info"] },
    { name: "log-level-debug", args: ["info", "--log-level", "debug"] },
    {
      name: "log-level-before-command",
      args: ["--log-level", "trace", "info"],
    },
    { name: "log-level-invalid", args: ["--log-level", "bogus", "info"] },
    { name: "log-level-missing-value", args: ["info", "--log-level"] },
    { name: "no-log-dir", args: ["info"], world: "noLogDir" },
  ]),
];
