/**
 * Root help texts, matching the Rust (clap) output byte for byte.
 * `help`, bare `agentctl` and `--help` print the long form; `-h` the short form.
 */
export const LONG_HELP =
  "📡 agentctl — tmux based agent multiplexer\n\nSpot and manage AI agent processes running across your tmux sessions.\n\nUsage: agentctl [OPTIONS] [COMMAND]\n\nCommands:\n  help       Show usage information\n  list       List tmux panes running agent harness processes\n  install    Install agentctl on a harness\n  uninstall  Uninstall agentctl from a harness\n  agent      Manage agent state\n  info       Print agentctl's runtime paths and metadata\n\nOptions:\n      --log-level <LEVEL>\n          Log level for the log file: trace, debug, info, warn, or error (default: info). Does not affect stdout/stderr output\n          \n          [default: info]\n          [possible values: trace, debug, info, warn, error]\n\n  -h, --help\n          Print help (see a summary with '-h')\n\n  -V, --version\n          Print version\n";

export const SHORT_HELP =
  "📡 agentctl — tmux based agent multiplexer\n\nUsage: agentctl [OPTIONS] [COMMAND]\n\nCommands:\n  help       Show usage information\n  list       List tmux panes running agent harness processes\n  install    Install agentctl on a harness\n  uninstall  Uninstall agentctl from a harness\n  agent      Manage agent state\n  info       Print agentctl's runtime paths and metadata\n\nOptions:\n      --log-level <LEVEL>  Log level for the log file: trace, debug, info, warn, or error (default: info). Does not affect stdout/stderr output [default: info] [possible values: trace, debug, info, warn, error]\n  -h, --help               Print help (see more with '--help')\n  -V, --version            Print version\n";
