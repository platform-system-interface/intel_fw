//! Modern tool to work on Intel firmware images
//!
//! Detecting what's given is unfortunately hard, because Intel firmware images
//! offer no simple indicator of the platform underneath or software features.
//! E.g. Lenovo ThinkPad X270:
//! It can be based on Skylake or Kaby Lake (SKL/KBL), 100/200 series chipsets.
//! Two different X270 laptops may or may not contain Intel AMT and its drivers.
//! So we can only provide meaningful information by looking at a full firmware
//! image in its entirety. This tool brings together all publicly known details.

use std::fs;

use clap::{Parser, Subcommand, ValueEnum};
use log::{debug, info};

mod clean;
mod show;

use intel_fw::Firmware;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Partition {
    MFS,
    FTPR,
    CODE, // only Gen 1
}

#[derive(Subcommand, Debug)]
enum MeCommand {
    /// Clean up (CS)ME partitions and related platform features
    Clean {
        /// File to write output to (cleaned image)
        #[clap(long, short = 'O')]
        output: Option<String>,
        /// Set MeAltDisable or HAP bit in addition (requires a full image)
        #[clap(long, short = 'S')]
        soft_disable: bool,
        /// Set MeAltDisable or HAP bit, nothing else (requires a full image)
        #[clap(long, short)]
        soft_disable_only: bool,
        /// Relocate FTPR partition to top of ME region
        #[clap(long, short)]
        relocate: bool,
        /// Truncuate empty part of the fimrware image
        #[clap(long, short)]
        truncate: bool,
        /// Retain FTPR modules even if they could be removed
        #[clap(long, short)]
        keep_modules: bool,
        /// Comma separated list of partitions to keep unconditionally
        #[clap(short, long, value_delimiter = ',')]
        whitelist: Option<Vec<Partition>>,
        /// Comma separated list of partitions to remove unconditionally
        #[clap(short, long, value_delimiter = ',')]
        blacklist: Option<Vec<Partition>>,
        /// Remove ME/TXE write permissions on other flash regions (requires a full image)
        #[clap(long, short)]
        descriptor: bool,
        /// Extract flash descriptor to a file, adjusting regions when used with truncate (requires a full image)
        #[clap(long, short = 'D')]
        extract_descriptor: Option<String>,
        /// Extract ME region to a file if given a full image
        #[clap(long, short = 'M')]
        extract_me: Option<String>,
        /// File to read
        file_name: String,
    },
    /// Display the (CS)ME high-level structures (full image or ME region)
    #[clap(verbatim_doc_comment)]
    Show {
        /// File to read
        file_name: String,
    },
    /// Scan for (CS)ME data structures (useful for update images)
    #[clap(verbatim_doc_comment)]
    Scan {
        /// File to read
        file_name: String,
    },
    /// Check for consistency (full image or ME region)
    #[clap(verbatim_doc_comment)]
    Check {
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

/// Analyze and modify Intel firmware images
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Command to run
    #[command(subcommand)]
    cmd: Command,
    #[clap(long, short, action)]
    debug: bool,
    #[clap(long, short, action)]
    verbose: bool,
}

fn main() {
    println!("Intel Firmware Tool 🔧");
    // Default to log level "info". Otherwise, you get no "regular" logs.
    let env = env_logger::Env::default().default_filter_or("info");
    env_logger::Builder::from_env(env).init();

    let Cli {
        cmd,
        debug,
        verbose,
    } = Cli::parse();
    match cmd {
        Command::Bg(_) => todo!(),
        Command::Me(cmd) => match cmd {
            MeCommand::Clean {
                descriptor,
                keep_modules,
                relocate,
                soft_disable,
                soft_disable_only,
                truncate,
                whitelist,
                blacklist,
                file_name,
                output,
                extract_descriptor,
                extract_me,
            } => {
                debug!("Configuration:");
                debug!("  Adjust flash descriptor: {descriptor}");
                debug!("  Retain FTPR modules:     {keep_modules}");
                debug!("  Relocate FTPR partition: {relocate}");
                debug!("  Truncate empty parts:    {truncate}");
                let disable_me = soft_disable || soft_disable_only;
                debug!("  Soft disable ME:         {disable_me}");
                debug!("");
                if let Some(allowlist) = whitelist {
                    debug!("Allowlist: {allowlist:?}");
                }
                if let Some(blocklist) = blacklist {
                    debug!("Blocklist: {blocklist:?}");
                }
                debug!("");
                if let Some(descriptor_file) = extract_descriptor {
                    info!("Dump flash descriptor to {descriptor_file}");
                }
                if let Some(me_file) = extract_me {
                    info!("Dump ME region to {me_file}");
                }
                if let Some(out_file) = output {
                    info!("Output will be written to: {out_file}");
                }
                info!("Reading {file_name}...");
                let data = fs::read(file_name).unwrap();
                let fw = Firmware::parse(&data, debug);
                show::show(&fw, verbose);
                println!();
                todo!("clean");
            }
            MeCommand::Scan { file_name } => {
                let data = fs::read(file_name).unwrap();
                let fw = Firmware::scan(&data, debug);
                show::show(&fw, verbose);
            }
            MeCommand::Check { file_name } => {
                todo!("check {file_name}")
            }
            MeCommand::Show { file_name } => {
                let data = fs::read(file_name).unwrap();
                let fw = Firmware::parse(&data, debug);
                show::show(&fw, verbose);
            }
        },
    }
}
