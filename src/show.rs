use intel_fw::dir::{gen2::Directory as Gen2Dir, gen3::CodePartitionDirectory};
use intel_fw::fit::Fit;
use intel_fw::fpt::{FPT, ME_FW};

fn print_gen2_dirs(dirs: &Vec<Gen2Dir>) {
    println!("Gen 2 directories:");
    for dir in dirs {
        println!("{dir}");
        for e in &dir.entries {
            let pos = dir.offset + e.offset as usize;
            /*
            let sig =
                u32::read_from_prefix(&data[pos..pos + 4]).unwrap();
            let kind = match sig {
                SIG_LUT => "LLUT",
                SIG_LZMA => "LZMA",
                _ => {
                    dump48(&data[pos..]);
                    "unknown"
                }
            };
            */
            let kind = "...";
            let t = e.compression_type();
            let b = e.bin_map();
            println!(" - {e}    {pos:08x} {t:?} ({kind})\n     {b}");
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

fn print_fpt(fpt: &FPT) {
    let FPT { header, entries } = fpt;
    println!("{header}");
    println!("Entries:");
    println!("  name     offset     end         size       type  notes");
    let mut entries = entries.clone();
    entries.sort_by_key(|e| e.offset);
    for e in entries {
        println!("- {e}");
    }
}

fn print_fit(fit: &Result<Fit, String>) {
    match fit {
        Ok(fit) => {
            println!("FIT @ {:08x}, {}", fit.offset, fit.header);
            for e in &fit.entries {
                println!("  {e}");
            }
        }
        Err(e) => {
            log::error!("Could not parse FIT: {e}");
        }
    }
}

pub fn show(me_fw: &ME_FW, verbose: bool) {
    if verbose {
        println!("{me_fw:#02x?}");
    }
    println!();
    let ME_FW {
        base,
        fpt,
        gen3dirs,
        gen2dirs,
        fit,
    } = me_fw;
    println!("FPT at 0x{base:08x}:");
    print_fpt(&fpt);
    println!();
    print_fit(&fit);
    println!();
    if !gen2dirs.is_empty() {
        print_gen2_dirs(&gen2dirs);
    }
    if !gen3dirs.is_empty() {
        print_gen3_dirs(&gen3dirs);
    }
}
