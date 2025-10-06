use clap::{Parser, Subcommand};

use log::info;

#[derive(Subcommand, Debug)]
enum MeCommand {
    /// Clean up (CS)ME partitions and related platform features
    Clean {
        /// File to read
        file_name: String,
    },
    /// Display the (CS)ME high-level structures
    #[clap(verbatim_doc_comment)]
    Show {
        /// File to read
        file_name: String,
    },
}

#[derive(Subcommand)]
enum BootGuardCommand {
    #[clap(verbatim_doc_comment)]
    Manifests,
}

#[derive(Parser)]
enum Command {
    /// Analyze and edit (CS)ME firmware and features
    #[command(subcommand)]
    Me(MeCommand),
    /// Anything related to BootGuard, such as manifests
    #[command(subcommand)]
    Bg(BootGuardCommand),
}

/// Intel firmware tool
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Command to run
    #[command(subcommand)]
    cmd: Command,
    #[clap(long, short, action)]
    verbose: bool,
}

fn main() {
    // Default to log level "info". Otherwise, you get no "regular" logs.
    let env = env_logger::Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env).init();
    info!("Intel Firmware Tool");

    let Cli { cmd, verbose: _ } = Cli::parse();
    match cmd {
        Command::Bg(_) => todo!(),
        Command::Me(cmd) => match cmd {
            MeCommand::Clean { file_name } => todo!("clean {file_name}"),
            MeCommand::Show { file_name } => todo!("show {file_name}"),
        },
    }
}
