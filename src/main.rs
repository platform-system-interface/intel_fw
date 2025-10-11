use std::{
    fs::{self, File},
    io::Write,
};

use clap::{Parser, Subcommand, ValueEnum};
use log::{debug, error, info, trace, warn};

mod clean;
mod show;

use intel_fw::{dir, parse};

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
                if let Some(out_file) = &output {
                    info!("Output will be written to: {out_file}");
                }
                info!("Reading {file_name}...");
                let mut data = fs::read(file_name).unwrap();
                match parse(&data, debug) {
                    Ok(fpt) => {
                        show::show(&fpt, verbose);
                        println!();
                        let blocklist = Vec::from(["rbe", "kernel", "syslib", "bup"])
                            .iter()
                            .map(|s| String::from(*s))
                            .collect();

                        for d in &fpt.gen3dirs {
                            if d.name == "FTPR" {
                                let dir_offset = d.offset;
                                let removables = d.clone().removable_entries(&blocklist);
                                println!("- {dir_offset:08x}");
                                for (mod_offset, size) in removables {
                                    println!("-- {mod_offset:08x} {size:08x}");
                                    for o in 0..size {
                                        data[dir_offset + mod_offset + o] = 0xff;
                                    }
                                }

                                let r = d.remainder();
                                let e = fpt.entries.iter().find(|e| e.name() == "FTPR").unwrap();
                                let end = dir_offset + e.size as usize;
                                info!("Remaining: {r:08x}..{end:08x}");
                                for o in r..end {
                                    data[o] = 0xff;
                                }
                            }
                        }

                        let fpt_offset = fpt.base;
                        for e in &fpt.entries {
                            match e.name().as_str() {
                                "FLOG" | "FTUP" | "IVBP" | "MFS" | "NFTP" | "PSVN" | "UTOK" => {
                                    let offset = fpt_offset + e.offset as usize;
                                    let size = e.size as usize;
                                    for o in offset..offset + size {
                                        data[o] = 0xff;
                                    }
                                }
                                _ => {} //
                            }
                        }

                        if let Some(out_file) = output {
                            let mut file = File::create(out_file).unwrap();
                            file.write_all(&data).unwrap();
                        }
                        todo!("clean");
                    }
                    Err(e) => {
                        error!("Could not parse ME FPT: {e}");
                    }
                }
            }
            MeCommand::Show { file_name } => {
                let data = fs::read(file_name).unwrap();
                match parse(&data, debug) {
                    Ok(fpt) => {
                        show::show(&fpt, verbose);
                    }
                    Err(e) => {
                        error!("Could not parse ME FPT: {e}");
                    }
                }
            }
        },
    }
}
