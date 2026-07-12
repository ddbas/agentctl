mod commands;

use clap::{CommandFactory, Parser, Subcommand};

/// dv — the drovr developer CLI
#[derive(Parser)]
#[command(
    name = "dv",
    about = "dv — the drovr developer CLI",
    long_about = None,
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

    /// List tmux panes running agent harness processes (claude, pi).
    List {
        /// Target a specific tmux server socket file (passed as `tmux -S <path>`).
        #[arg(short, long)]
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
