mod commands;
#[allow(dead_code)]
mod events_log;
mod logging;
#[allow(dead_code)]
mod state;
mod status_fsm;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rd",
    about = "📡 rd — tmux based agent multiplexer",
    long_about = "📡 rd — tmux based agent multiplexer\n\nSpot and manage AI agent processes running across your tmux sessions.",
    version,
    propagate_version = true,
    // Disable built-in `help` subcommand so we can define our own.
    disable_help_subcommand = true,
)]
struct Cli {
    /// Log level for the log file: trace, debug, info, warn, or error
    /// (default: info). Does not affect stdout/stderr output.
    #[arg(long, global = true, default_value = "info", value_name = "LEVEL")]
    log_level: logging::LogLevel,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show usage information.
    Help,

    /// List tmux panes running agent harness processes.
    ///
    /// Queries every pane across all tmux sessions via `tmux list-panes -a`
    /// and prints one line per pane whose foreground process is a known agent
    /// harness (`claude` or `pi`).  By default, output has five left-aligned
    /// columns: `<agent>  <session>  <window>.<pane>  <status>  <activeTools>`.
    /// The rendered columns can be customized via `--columns`/`-c`.
    ///
    /// `node` panes are resolved to `pi` when the pane's shell has a direct
    /// child process whose argv[0] basename is `pi`.
    ///
    /// When no agents are found, no output is produced and the command exits
    /// successfully.  If tmux is not running, the list is treated as empty.
    #[command(alias = "ls")]
    List {
        /// Target a specific tmux server socket file.
        /// Forwarded as `tmux -S <path>`.  Falls back to the
        /// `RD_TMUX_SOCKET` environment variable when not set.
        #[arg(short, long, value_name = "PATH")]
        server: Option<String>,

        /// Sort output by this property.
        /// Valid values: agent, session, pane, status, updatedAt, activeTools.
        #[arg(
            long = "sort-by",
            value_name = "KEY",
            default_value_t = commands::list::Field::UpdatedAt
        )]
        sort_by: commands::list::Field,

        /// Reverse the sort order.
        /// For `updatedAt` (the default key) this means oldest-active first.
        #[arg(short = 'r', long)]
        reverse: bool,

        /// Comma-delimited, ordered list of columns to render.
        /// Valid values: agent, session, pane, status, updatedAt, activeTools.
        /// Order and duplicates are preserved (no deduplication).
        #[arg(
            short = 'c',
            long = "columns",
            value_name = "COLUMNS",
            value_delimiter = ',',
            default_value = commands::list::DEFAULT_COLUMNS_STR
        )]
        columns: Vec<commands::list::Field>,
    },

    /// Install radr on a harness.
    ///
    /// Writes the necessary integration files for the chosen harness so that
    /// `rd list` shows real-time agent state without polling. Today this means
    /// installing hook integrations, but may include more in the future.
    Install {
        /// Harness to install radr on.
        #[arg(value_name = "HARNESS")]
        harness: Harness,
    },

    /// Uninstall radr from a harness.
    ///
    /// Removes the integration files written by `rd install <harness>`,
    /// leaving all other user configuration untouched. Idempotent: running
    /// this on a harness that was never installed (or already uninstalled)
    /// is a no-op that exits 0.
    ///
    /// - `pi`: deletes `~/.pi/agent/extensions/radr.ts` if present.
    /// - `claude`: removes only the radr-managed hook entries from
    ///   `~/.claude/settings.json`, pruning empty event arrays and the
    ///   `hooks` key, but never deleting the settings file itself.
    Uninstall {
        /// Harness to uninstall radr from.
        #[arg(value_name = "HARNESS")]
        harness: Harness,
    },

    /// Manage agent state.
    Agent {
        #[command(subcommand)]
        subcommand: AgentSubcommand,
    },

    /// Print radr's runtime paths and metadata.
    ///
    /// Prints resolved paths (log directory, current log file) and metadata
    /// (version, effective log level) as `key: value` lines. Performs no
    /// filesystem writes.
    Info,
}

#[derive(Subcommand)]
enum AgentSubcommand {
    /// Dispatch a typed event; the pane's status is derived from the event
    /// stream via a finite state machine.
    ///
    /// Reads `$TMUX_PANE` from the environment to identify the current pane.
    /// Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not
    /// supplied — suitable for calling from hooks that may run outside tmux.
    ///
    /// `--harness` and `--pane` are declared once here and inherited by every
    /// event subcommand below (they may appear before or after the event
    /// name on the command line).
    Event {
        /// Harness name to write to the state file (e.g. `claude`).
        /// Defaults to `"unknown"` when omitted.
        #[arg(long, value_name = "HARNESS", global = true)]
        harness: Option<String>,

        /// Override the pane ID instead of reading `$TMUX_PANE`.
        #[arg(long, value_name = "PANE_ID", global = true)]
        pane: Option<String>,

        #[command(subcommand)]
        event: AgentEventSubcommand,
    },

    /// Whole-session cleanup for the current agent pane.
    ///
    /// Removes the status file and any other per-agent artifacts recorded
    /// for the pane. Reads `$TMUX_PANE` from the environment. Exits 0
    /// silently when `$TMUX_PANE` is not set and `--pane` is not supplied, or
    /// when no state file exists for the pane.
    Clear {
        /// Override the pane ID instead of reading `$TMUX_PANE`.
        #[arg(long, value_name = "PANE_ID")]
        pane: Option<String>,
    },
}

/// The five events accepted by `rd agent event <NAME>`.
///
/// `--id` is required on all five; `--type` is only accepted on the two
/// `TOOL_CALL_*` variants.
#[derive(Subcommand)]
enum AgentEventSubcommand {
    /// A harness session began (or resumed / was cleared). Resets the pane's
    /// status to `idle` from any prior status.
    #[command(name = "SESSION_STARTED")]
    SessionStarted {
        /// Unique id for this session (used to correlate events.jsonl entries).
        #[arg(long, value_name = "ID")]
        id: String,
    },

    /// A run (agent turn) started.
    #[command(name = "RUN_STARTED")]
    RunStarted {
        /// Unique id for this run.
        #[arg(long, value_name = "ID")]
        id: String,
    },

    /// A run (agent turn) finished.
    #[command(name = "RUN_FINISHED")]
    RunFinished {
        /// Unique id for this run (matching the corresponding RUN_STARTED).
        #[arg(long, value_name = "ID")]
        id: String,
    },

    /// A tool call started.
    #[command(name = "TOOL_CALL_START")]
    ToolCallStart {
        /// Unique id for this tool call.
        #[arg(long, value_name = "ID")]
        id: String,

        /// Payload type. `elicitation` drives the agent into `blocked`.
        #[arg(long, value_name = "TYPE")]
        r#type: Option<ToolCallTypeArg>,
    },

    /// A tool call finished.
    #[command(name = "TOOL_CALL_RESULT")]
    ToolCallResult {
        /// Unique id for this tool call (matching the corresponding
        /// TOOL_CALL_START).
        #[arg(long, value_name = "ID")]
        id: String,

        /// Payload type. `elicitation` clears a `blocked` status back to
        /// `working`.
        #[arg(long, value_name = "TYPE")]
        r#type: Option<ToolCallTypeArg>,
    },
}

/// Accepted `--type` values for `TOOL_CALL_START` / `TOOL_CALL_RESULT`.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
#[value(rename_all = "snake_case")]
enum ToolCallTypeArg {
    Custom,
    Elicitation,
}

impl From<ToolCallTypeArg> for status_fsm::ToolCallType {
    fn from(value: ToolCallTypeArg) -> Self {
        match value {
            ToolCallTypeArg::Custom => status_fsm::ToolCallType::Custom,
            ToolCallTypeArg::Elicitation => status_fsm::ToolCallType::Elicitation,
        }
    }
}

/// Harnesses supported by `rd install` and `rd uninstall`.
///
/// Shared by both subcommands so they can never drift out of sync on which
/// harnesses are supported: `match harness { ... }` in both `run_install`
/// and `run_uninstall` must remain exhaustive (no wildcard `_` arm), so
/// adding a new variant here forces both call sites to be updated.
#[derive(clap::ValueEnum, Clone, Debug)]
enum Harness {
    /// Pi coding agent — writes `~/.pi/agent/extensions/radr.ts`.
    Pi,
    /// Claude Code — patches `~/.claude/settings.json`.
    Claude,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging (best-effort; failures are non-fatal)
    let _guard = logging::init(cli.log_level);

    // Log the parsed CLI invocation
    tracing::debug!(
        "CLI invocation: rd {:?}",
        std::env::args().collect::<Vec<_>>()
    );

    match cli.command {
        None | Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }
        Some(Commands::List {
            server,
            sort_by,
            reverse,
            columns,
        }) => {
            commands::list::run(server, sort_by, reverse, columns)?;
        }
        Some(Commands::Install { harness }) => {
            commands::hooks::run_install(harness)?;
        }
        Some(Commands::Uninstall { harness }) => {
            commands::hooks::run_uninstall(harness)?;
        }
        Some(Commands::Agent {
            subcommand:
                AgentSubcommand::Event {
                    harness,
                    pane,
                    event,
                },
        }) => {
            let (kind, id, tool_call_type) = match event {
                AgentEventSubcommand::SessionStarted { id } => {
                    (commands::agent_event::EventKind::SessionStarted, id, None)
                }
                AgentEventSubcommand::RunStarted { id } => {
                    (commands::agent_event::EventKind::RunStarted, id, None)
                }
                AgentEventSubcommand::RunFinished { id } => {
                    (commands::agent_event::EventKind::RunFinished, id, None)
                }
                AgentEventSubcommand::ToolCallStart { id, r#type } => (
                    commands::agent_event::EventKind::ToolCallStart,
                    id,
                    r#type.map(Into::into),
                ),
                AgentEventSubcommand::ToolCallResult { id, r#type } => (
                    commands::agent_event::EventKind::ToolCallResult,
                    id,
                    r#type.map(Into::into),
                ),
            };
            commands::agent_event::run(kind, id, tool_call_type, harness, pane)?;
        }
        Some(Commands::Agent {
            subcommand: AgentSubcommand::Clear { pane },
        }) => {
            commands::agent_clear::run(pane)?;
        }
        Some(Commands::Info) => {
            commands::info::run(cli.log_level)?;
        }
    }

    Ok(())
}
