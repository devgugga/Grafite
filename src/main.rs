use grafite::error;

use clap::{Parser, Subcommand};
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "grafite",
    version,
    about = "Provenance records from verifiable Git facts"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Extract provenance records from Git history.
    Sync,
    /// Report provider preconditions.
    Doctor,
    /// Show provenance records that touched a path.
    Why { path: String },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let outcome: error::Result<String> = match cli.command {
        Command::Sync => match std::env::current_dir() {
            Ok(dir) => grafite::commands::sync(&dir),
            Err(e) => Err(grafite::error::Error::new(
                "determine the current directory",
                "<cwd>",
                e.to_string(),
                "run grafite from inside a git repository",
            )),
        },
        Command::Doctor => Ok(String::new()),
        Command::Why { path: _ } => Ok(String::new()),
    };
    match outcome {
        Ok(payload) => {
            if !payload.is_empty() {
                println!("{payload}");
            }
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
