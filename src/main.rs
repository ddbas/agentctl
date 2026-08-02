mod commands;
#[allow(dead_code)]
mod state;

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
    /// harness (`claude` or `pi`).  Output is four left-aligned columns:
    /// `<agent>  <session>  <window>.<pane>  <state>`.
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
        /// Valid values: state, agent, session, pane, updated-at.
        #[arg(
            long = "sort-by",
            value_name = "KEY",
            default_value_t = commands::list::SortKey::UpdatedAt
        )]
        sort_by: commands::list::SortKey,

        /// Reverse the sort order.
        /// For `updated-at` (the default key) this means oldest-active first.
        #[arg(short = 'r', long)]
        reverse: bool,
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

    /// Manage agent state.
    State {
        #[command(subcommand)]
        subcommand: StateSubcommand,
    },
}

#[derive(Subcommand)]
enum StateSubcommand {
    /// Write the current agent state to the state file.
    ///
    /// Reads `$TMUX_PANE` from the environment to identify the current pane.
    /// Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not
    /// supplied — suitable for calling from hooks that may run outside tmux.
    Set {
        /// New state. Accepted values: idle, working, done, blocked.
        #[arg(value_name = "STATE")]
        state: String,

        /// Harness name to write to the state file (e.g. `claude`).
        /// Defaults to `"unknown"` when omitted.
        #[arg(long, value_name = "HARNESS")]
        harness: Option<String>,

        /// Override the pane ID instead of reading `$TMUX_PANE`.
        #[arg(long, value_name = "PANE_ID")]
        pane: Option<String>,
    },

    /// Remove the state file for the current agent pane.
    ///
    /// Reads `$TMUX_PANE` from the environment. Exits 0 silently when
    /// `$TMUX_PANE` is not set and `--pane` is not supplied, or when no
    /// state file exists for the pane.
    Clear {
        /// Override the pane ID instead of reading `$TMUX_PANE`.
        #[arg(long, value_name = "PANE_ID")]
        pane: Option<String>,
    },
}

/// Harnesses supported by `rd install`.
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

    match cli.command {
        None | Some(Commands::Help) => {
            Cli::command().print_long_help()?;
        }
        Some(Commands::List {
            server,
            sort_by,
            reverse,
        }) => {
            commands::list::run(server, sort_by, reverse)?;
        }
        Some(Commands::Install { harness }) => {
            commands::hooks::run_install(harness)?;
        }
        Some(Commands::State {
            subcommand:
                StateSubcommand::Set {
                    state,
                    harness,
                    pane,
                },
        }) => {
            commands::set_state::run(state, harness, pane)?;
        }
        Some(Commands::State {
            subcommand: StateSubcommand::Clear { pane },
        }) => {
            commands::clear_state::run(pane)?;
        }
    }

    Ok(())
}
