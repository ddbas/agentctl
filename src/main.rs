mod commands;
#[allow(dead_code)]
mod state;

use clap::{CommandFactory, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dv",
    about = "🐂 dv — tmux based agent multiplexer",
    long_about = "🐂 dv — tmux based agent multiplexer\n\nSpot and manage AI agent processes running across your tmux sessions.",
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
        /// `DV_TMUX_SOCKET` environment variable when not set.
        #[arg(short, long, value_name = "PATH")]
        server: Option<String>,
    },

    /// Write the current agent state to the state file.
    ///
    /// Reads `$TMUX_PANE` from the environment to identify the current pane.
    /// Exits 0 silently when `$TMUX_PANE` is not set and `--pane` is not
    /// supplied — suitable for calling from hooks that may run outside tmux.
    #[command(name = "set-state")]
    SetState {
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
    #[command(name = "clear-state")]
    ClearState {
        /// Override the pane ID instead of reading `$TMUX_PANE`.
        #[arg(long, value_name = "PANE_ID")]
        pane: Option<String>,
    },

    /// Manage agent harness hooks.
    Hooks {
        #[command(subcommand)]
        subcommand: HooksSubcommand,
    },
}

#[derive(Subcommand)]
enum HooksSubcommand {
    /// Install hooks for a harness.
    ///
    /// Writes the necessary hook files for the chosen harness so that
    /// `dv list` shows real-time agent state without polling.
    Install {
        /// Harness to install hooks for.
        #[arg(value_name = "HARNESS")]
        harness: Harness,
    },
}

/// Harnesses supported by `dv hooks install`.
#[derive(clap::ValueEnum, Clone, Debug)]
enum Harness {
    /// Pi coding agent — writes `~/.pi/agent/extensions/drovr.ts`.
    Pi,
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
        Some(Commands::List { server }) => {
            commands::list::run(server)?;
        }
        Some(Commands::SetState {
            state,
            harness,
            pane,
        }) => {
            commands::set_state::run(state, harness, pane)?;
        }
        Some(Commands::ClearState { pane }) => {
            commands::clear_state::run(pane)?;
        }
        Some(Commands::Hooks {
            subcommand: HooksSubcommand::Install { harness },
        }) => {
            commands::hooks::run_install(harness)?;
        }
    }

    Ok(())
}
