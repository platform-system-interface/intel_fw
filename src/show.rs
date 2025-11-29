use log::{error, warn};

use intel_fw::{
    Firmware,
    dir::gen2::{Directory as Gen2Dir, Module},
    fit::Fit,
    ifd::{FlashMasterV1, FlashMasterV2, IFD},
    me::{Generation, ME},
    part::{fpt::FTUP, gen2::Gen2Partition, gen3::Gen3Partition, partitions::Partitions},
};

fn print_gen2_dir(dir: &Gen2Dir) {
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
        println!("- {e} @ {pos:08x}\n     {b}");
    }
    println!();
}

// Format a bool, used to tell if a bit is set.
fn bool_as_str(b: bool) -> &'static str {
    if b { "set" } else { "not set" }
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
    println!("=== Intel (CS)ME ===");
    println!("{:?} detected", me.generation);
    println!();
    println!("FPT at 0x{:08x}:", me.base);
    let pre_header = &me.fpt_area.fpt.pre_header;
    let header = &me.fpt_area.fpt.header;
    println!("Pre-header: {pre_header:02x?}");
    println!("{header}");
    println!("Entries:");
    println!("  name     offset     end         size       type  notes");
    let entries = me.fpt_area.fpt.get_sorted_entries();
    for e in entries {
        println!("- {e}");
    }
    println!();
    match &me.fpt_area.partitions {
        Partitions::Gen2(parts) => {
            println!("Partitions and directories:");
            println!();
            for p in parts {
                if let Gen2Partition::Dir(dir) = p {
                    print_gen2_dir(&dir.dir);
                }
            }
        }
        Partitions::Gen3(parts) => {
            println!("Partitions and directories:");
            println!();
            for p in parts {
                if let Gen3Partition::Dir(dir) = p {
                    let d = &dir.cpd;
                    if d.name == FTUP {
                        // FTUP contains NFTP and potentially WCOD and LOCL.
                        // Skip it to avoid redundant printing.
                        continue;
                    }
                    println!("{d}");
                }
            }
        }
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

fn print_ifd(ifd: &IFD, ver: Option<IfdVersion>) {
    println!("=== Flash descriptor ===");
    println!("{ifd}");
    println!("== Masters ==");
    for (i, e) in ifd.masters.iter().enumerate() {
        match ver {
            Some(IfdVersion::V1) => {
                let m = FlashMasterV1::from_bits(*e);
                println!(" {i:2}:\n{m}");
            }
            Some(IfdVersion::V2) => {
                let m = FlashMasterV2::from_bits(*e);
                println!(" {i:2}:\n{m}");
            }
            _ => println!(" {i:2}: {e:08x}"),
        }
    }
}

pub fn show(fw: &Firmware, verbose: bool) {
    if verbose {
        println!("{fw:#02x?}");
    }
    println!();
    match &fw.ifd {
        Ok(ifd) => {
            let ver = get_ifd_ver(&fw.me);
            print_ifd(ifd, ver);
        }
        Err(e) => warn!("Could not parse IFD: {e:?}"),
    }
    if let Some(me_res) = &fw.me {
        match me_res {
            Ok(me) => {
                if let Ok(ifd) = &fw.ifd {
                    print_me_soft_config(me, ifd);
                    println!();
                }
                print_me(me);
            }
            Err(e) => error!("ME firmware could not be parsed: {e:?}"),
        }
    } else {
        error!("No ME firmware found");
    }
    println!();
    match &fw.fit {
        Ok(fit) => print_fit(fit),
        Err(e) => warn!("Could not parse FIT: {e:?}"),
    }
    println!();
}
