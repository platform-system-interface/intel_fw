use log::{error, warn};

use intel_fw::{
    dir::{
        gen2::{Directory as Gen2Dir, Module},
        gen3::CodePartitionDirectory,
    },
    fit::Fit,
    ifd::{FlashMasterV1, FlashMasterV2, IFD},
    me::{Directories, Generation, ME},
    Firmware,
};

fn print_gen2_dirs(dirs: &Vec<Gen2Dir>) {
    println!("Gen 2 directories:");
    for dir in dirs {
        println!("{dir}");
        for m in &dir.modules {
            let e = match m {
                Module::Uncompressed(e) => e,
                Module::Huffman(Ok((e, _))) => e,
                Module::Lzma(Ok(e)) => e,
                Module::Unknown(e) => e,
                _ => continue,
            };
            let pos = dir.offset + e.offset as usize;
            let b = e.bin_map();
            println!(" - {e} @ {pos:08x}\n     {b}");
        }
        println!();
    }
}

fn print_gen3_dirs(dirs: &Vec<CodePartitionDirectory>) {
    println!("Gen 3 directories:");
    for d in dirs {
        println!("{d}");
    }
}

// Format a bool, used to tell if a bit is set.
fn bool_as_str(b: bool) -> &'static str {
    if b {
        "set"
    } else {
        "not set"
    }
}

fn print_me_soft_config(me: &ME, ifd: &IFD) {
    println!("== ME soft configuration ==");
    match &me.generation {
        Generation::Gen1 => {
            let imd = ifd.ich_me_disabled();
            println!("   ICH MeDisable bit: {}", bool_as_str(imd));
        }
        Generation::Gen2 => {
            let amd = ifd.alt_me_disabled();
            println!("    AltMeDisable bit: {}", bool_as_str(amd));
        }
        Generation::Gen3 => {
            let hap = ifd.hap();
            println!("             HAP bit: {}", bool_as_str(hap));
        }
        Generation::Unknown => {
            println!("   Cannot tell, ME generation not known.");
        }
    }
}

fn print_me(me: &ME) {
    println!("FPT at 0x{:08x}:", me.base);
    let pre_header = &me.fpt.pre_header;
    let header = &me.fpt.header;
    println!("Pre-header: {pre_header:02x?}");
    println!("{header}");
    println!("Entries:");
    println!("  name     offset     end         size       type  notes");
    let mut entries = me.fpt.entries.clone();
    entries.sort_by_key(|e| e.offset);
    for e in entries {
        println!("- {e}");
    }
    match &me.dirs {
        Directories::Gen2(dirs) => print_gen2_dirs(dirs),
        Directories::Gen3(dirs) => print_gen3_dirs(dirs),
        // TODO
        _ => {}
    }
}

fn print_fit(fit: &Fit) {
    println!("FIT @ {:08x}, {}", fit.offset, fit.header);
    for e in &fit.entries {
        println!("  {e}");
    }
}

enum IfdVersion {
    V1,
    V2,
}

/// Get the IFD version based on ME generation.
fn get_ifd_ver(me: &Option<Result<ME, String>>) -> Option<IfdVersion> {
    let Some(Ok(me)) = me else {
        return None;
    };
    match me.generation {
        Generation::Gen3 => Some(IfdVersion::V2),
        _ => Some(IfdVersion::V1),
    }
}

pub fn show(fw: &Firmware, verbose: bool) {
    if verbose {
        println!("{fw:#02x?}");
    }
    println!();
    match &fw.ifd {
        Ok(ifd) => {
            if verbose {
                println!("{ifd:?}");
            } else {
                println!("{ifd}");
                println!("== Masters ==");
                let ifd_ver = get_ifd_ver(&fw.me);
                for (i, e) in ifd.masters.iter().enumerate() {
                    match ifd_ver {
                        Some(IfdVersion::V1) => {
                            let m = FlashMasterV1::from_bits(*e);
                            println!(" {i:2}:\n{m}");
                        }
                        Some(IfdVersion::V2) => {
                            let m = FlashMasterV2::from_bits(*e);
                            println!(" {i:2}:\n{m}");
                        }
                        _ => println!(" {i:2}: {e:08x}"),
                    };
                }
            }
        }
        Err(e) => warn!("Could not parse IFD: {e:?}"),
    }
    if let Some(me_res) = &fw.me {
        match me_res {
            Ok(me) => {
                if let Ok(ifd) = &fw.ifd {
                    print_me_soft_config(&me, &ifd);
                    println!();
                }
                print_me(&me);
            }
            Err(e) => error!("ME firmware could not be parsed: {e:?}"),
        }
    } else {
        error!("No ME firmware found");
    }
    println!();
    match &fw.fit {
        Ok(fit) => print_fit(&fit),
        Err(e) => warn!("Could not parse FIT: {e:?}"),
    }
    println!();
}
