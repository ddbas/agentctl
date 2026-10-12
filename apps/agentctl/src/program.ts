import {
  type App,
  DEFAULT_COLUMNS,
  DEFAULT_COLUMNS_STR,
  type EventKind,
  FIELDS,
  type Field,
  LOG_LEVELS,
  type LogLevel,
  type ToolCallType,
} from "@agentctl/core";
import { Argument, Command, InvalidArgumentError, Option } from "commander";
import { LONG_HELP } from "./help.ts";
import { VERSION } from "./version.ts";

const HARNESSES = ["pi", "claude"] as const;
const TOOL_CALL_TYPES = ["custom", "elicitation"] as const;

/** Everything the CLI needs from the composition root. */
export interface ProgramDeps {
  /** Called once after parsing, before any command runs. */
  createApp(logLevel: LogLevel, argv: string[]): App;
  /** Writes to stdout (no newline appended). */
  write(text: string): void;
}

/** Accumulating, comma-delimited `--columns` parser (clap `value_delimiter`). */
function parseColumns(value: string, previous: readonly Field[]): Field[] {
  const base = previous === DEFAULT_COLUMNS ? [] : [...previous];
  for (const part of value.split(",")) {
    if (!(FIELDS as readonly string[]).includes(part)) {
      throw new InvalidArgumentError(
        `invalid value '${part}'. Allowed choices are ${FIELDS.join(", ")}.`,
      );
    }
    base.push(part as Field);
  }
  return base;
}

/** Parse a `u32` like clap's `Option<u32>`. */
function parseU32(value: string): number {
  if (!/^\+?\d+$/.test(value) || Number(value) > 0xffff_ffff) {
    throw new InvalidArgumentError(`'${value}' is not a valid number`);
  }
  return Number(value);
}

const idOption = (description: string) =>
  new Option("--id <ID>", description).makeOptionMandatory();

const typeOption = (description: string) =>
  new Option("--type <TYPE>", description).choices(TOOL_CALL_TYPES);

/**
 * Build the commander program. Contains only argument definitions and one
 * `App` call per subcommand.
 */
export function buildProgram(deps: ProgramDeps, argv: string[]): Command {
  const program = new Command("agentctl")
    .description("📡 agentctl — tmux based agent multiplexer")
    .version(`agentctl ${VERSION}`, "-V, --version", "Print version")
    .helpOption("-h, --help", "Print help")
    .addOption(
      new Option(
        "--log-level <LEVEL>",
        "Log level for the log file: trace, debug, info, warn, or error (default: info). Does not affect stdout/stderr output",
      )
        .choices(LOG_LEVELS)
        .default("info"),
    )
    .exitOverride()
    .configureHelp({ sortOptions: false });

  const app = (command: Command): App => {
    const level = command.optsWithGlobals<{ logLevel: LogLevel }>().logLevel;
    return deps.createApp(level, argv);
  };

  program
    .command("help")
    .description("Show usage information")
    .action(() => deps.write(LONG_HELP));

  program
    .command("list")
    .alias("ls")
    .description(
      "List tmux panes running agent harness processes.\n\nQueries every pane across all tmux sessions via `tmux list-panes -a` and prints one line per pane whose foreground process is a known agent harness (`claude` or `pi`).",
    )
    .option(
      "-s, --server <PATH>",
      "Target a specific tmux server socket file. Forwarded as `tmux -S <path>`. Falls back to the `AGENTCTL_TMUX_SOCKET` environment variable when not set",
    )
    .addOption(
      new Option(
        "--sort-by <KEY>",
        "Sort output by this property. Valid values: agent, session, pane, status, updatedAt, activeTools",
      )
        .choices(FIELDS)
        .default("updatedAt"),
    )
    .option(
      "-r, --reverse",
      "Reverse the sort order. For `updatedAt` (the default key) this means oldest-active first",
    )
    .addOption(
      new Option(
        "-c, --columns <COLUMNS>",
        "Comma-delimited, ordered list of columns to render. Valid values: agent, session, pane, status, updatedAt, activeTools. Order and duplicates are preserved (no deduplication)",
      )
        .argParser(parseColumns)
        .default(DEFAULT_COLUMNS, DEFAULT_COLUMNS_STR),
    )
    .action((opts, command: Command) => {
      deps.write(
        app(command).list(
          opts.server,
          opts.sortBy,
          opts.reverse === true,
          opts.columns,
        ),
      );
    });

  for (const name of ["install", "uninstall"] as const) {
    program
      .command(name)
      .description(
        name === "install"
          ? "Install agentctl on a harness.\n\nWrites the necessary integration files for the chosen harness so that `agentctl list` shows real-time agent state without polling."
          : "Uninstall agentctl from a harness.\n\nRemoves the integration files written by `agentctl install <harness>`, leaving all other user configuration untouched. Idempotent.",
      )
      .addArgument(
        new Argument(
          "<HARNESS>",
          `Harness to ${name} agentctl ${name === "install" ? "on" : "from"}`,
        )
          .choices(HARNESSES)
          .argRequired(),
      )
      .action((harness: string, _opts, command: Command) => {
        const a = app(command);
        deps.write(
          `${name === "install" ? a.install(harness) : a.uninstall(harness)}\n`,
        );
      });
  }

  const agent = program.command("agent").description("Manage agent state");

  const event = agent
    .command("event")
    .description(
      "Dispatch a typed event; the pane's status is derived from the event stream via a finite state machine.\n\nReads `$TMUX_PANE` from the environment to identify the current pane. Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not supplied.",
    )
    .option(
      "--harness <HARNESS>",
      'Harness name to write to the state file (e.g. `claude`). Defaults to `"unknown"` when omitted',
    )
    .option(
      "--pane <PANE_ID>",
      "Override the pane ID instead of reading `$TMUX_PANE`",
    );

  const dispatch = (
    kind: EventKind,
    command: Command,
    id: string,
    type?: ToolCallType,
  ) => {
    const opts = command.optsWithGlobals<{ harness?: string; pane?: string }>();
    app(command).dispatchEvent(kind, id, type, opts.harness, opts.pane);
  };

  event
    .command("SESSION_STARTED")
    .description(
      "A harness session began (or resumed / was cleared). Resets the pane's status to `idle` from any prior status",
    )
    .addOption(
      idOption(
        "Unique id for this session (used to correlate events.jsonl entries)",
      ),
    )
    .action((opts, command: Command) =>
      dispatch({ kind: "SESSION_STARTED" }, command, opts.id),
    );

  event
    .command("RUN_STARTED")
    .description("A run (agent turn) started")
    .addOption(idOption("Unique id for this run"))
    .action((opts, command: Command) =>
      dispatch({ kind: "RUN_STARTED" }, command, opts.id),
    );

  event
    .command("RUN_FINISHED")
    .description("A run (agent turn) finished")
    .addOption(
      idOption(
        "Unique id for this run (matching the corresponding RUN_STARTED)",
      ),
    )
    .addOption(
      new Option(
        "--active-tools <N>",
        "Override the pane's `activeTools` count instead of leaving it untouched",
      ).argParser(parseU32),
    )
    .action((opts, command: Command) =>
      dispatch(
        { kind: "RUN_FINISHED", activeTools: opts.activeTools },
        command,
        opts.id,
      ),
    );

  event
    .command("TOOL_CALL_START")
    .description("A tool call started")
    .addOption(idOption("Unique id for this tool call"))
    .addOption(
      typeOption("Payload type. `elicitation` drives the agent into `blocked`"),
    )
    .action((opts, command: Command) =>
      dispatch({ kind: "TOOL_CALL_START" }, command, opts.id, opts.type),
    );

  event
    .command("TOOL_CALL_RESULT")
    .description("A tool call finished")
    .addOption(
      idOption(
        "Unique id for this tool call (matching the corresponding TOOL_CALL_START)",
      ),
    )
    .addOption(
      typeOption(
        "Payload type. `elicitation` clears a `blocked` status back to `working`",
      ),
    )
    .action((opts, command: Command) =>
      dispatch({ kind: "TOOL_CALL_RESULT" }, command, opts.id, opts.type),
    );

  agent
    .command("clear")
    .description(
      "Whole-session cleanup for the current agent pane.\n\nRemoves the status file and any other per-agent artifacts recorded for the pane. Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not supplied, or when no state file exists for the pane.",
    )
    .option(
      "--pane <PANE_ID>",
      "Override the pane ID instead of reading `$TMUX_PANE`",
    )
    .action((opts, command: Command) => {
      app(command).clear(opts.pane);
    });

  program
    .command("info")
    .description(
      "Print agentctl's runtime paths and metadata.\n\nPrints resolved paths (log directory, current log file) and metadata (version, effective log level) as `key: value` lines. Performs no filesystem writes.",
    )
    .action((_opts, command: Command) => {
      const level = command.optsWithGlobals<{ logLevel: LogLevel }>().logLevel;
      deps.write(app(command).info(level));
    });

  // Bare `agentctl` prints the long help (clap `print_long_help`).
  program.action(() => deps.write(LONG_HELP));

  return program;
}
