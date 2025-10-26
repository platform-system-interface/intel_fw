use log::{error, warn};

use intel_fw::{
    Firmware,
    dir::{
        gen2::{Directory as Gen2Dir, Module},
        gen3::CodePartitionDirectory,
    },
    fit::Fit,
    fpt::FPT,
    me::{Directories, ME},
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

pub fn show(fw: &Firmware, verbose: bool) {
    if verbose {
        println!("{fw:#02x?}");
    }
    println!();
    match &fw.ifd {
        Ok(ifd) => println!("{ifd}"),
        Err(e) => warn!("Could not parse IFD: {e:?}"),
    }
    if let Some(me_res) = &fw.me {
        match me_res {
            Ok(me) => print_me(&me),
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
