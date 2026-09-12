use crate::commands::hooks::tilde_collapse;
use crate::logging;

/// Run the `agentctl info` subcommand: print version, effective log level, log
/// directory, and current log file as `key: value` lines. Performs no
/// filesystem writes.
pub fn run(log_level: logging::LogLevel) -> anyhow::Result<()> {
    let version = env!("CARGO_PKG_VERSION");

    let log_level_str = match log_level {
        logging::LogLevel::Trace => "trace",
        logging::LogLevel::Debug => "debug",
        logging::LogLevel::Info => "info",
        logging::LogLevel::Warn => "warn",
        logging::LogLevel::Error => "error",
    };

    let log_dir =
        logging::log_dir().ok_or_else(|| anyhow::anyhow!("failed to resolve log directory"))?;
    let current_log_file = logging::current_log_file(&log_dir);

    let log_dir_tilde = tilde_collapse(&log_dir);
    let current_log_file_tilde = tilde_collapse(&current_log_file);

    println!("version: {version}");
    println!("log level: {log_level_str}");
    println!("log directory: {log_dir_tilde}");
    println!("current log file: {current_log_file_tilde}");

    Ok(())
}
