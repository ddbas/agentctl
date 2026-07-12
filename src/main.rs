mod commands;

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
    /// harness (`claude` or `pi`).  Output is three left-aligned columns:
    /// `<agent>  <session>  <window>.<pane>`.
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
    }

    Ok(())
}
