use core::mem;
use log::error;

pub mod dir;
pub mod fit;
pub mod fpt;
pub mod meta;
pub mod ver;

pub use fpt::ME_FPT;
use fpt::{AFSP, DLMP, EFFS, FTPR, FTUP, MDMV, MFS, NFTP};

fn dump48(data: &[u8]) {
    println!("Here are the first 48 bytes:");
    let b = &data[0..0x10];
    println!("{b:02x?}");
    let b = &data[0x10..0x20];
    println!("{b:02x?}");
    let b = &data[0x20..0x30];
    println!("{b:02x?}");
}

pub fn parse(data: &[u8], debug: bool) -> Result<ME_FPT, String> {
    let fit = fit::Fit::new(data);

    let cpd_bytes = dir::gen3::CPD_MAGIC.as_bytes();
    let mut gen2dirs = Vec::<dir::gen2::Directory>::new();
    let mut gen3dirs = Vec::<dir::gen3::CodePartitionDirectory>::new();

    // Scan for all CPDs (there may be some not listed in FPT)
    if debug {
        let mut o = 0;
        while o < data.len() {
            let buf = &data[o..o + 4];
            if buf.eq(cpd_bytes) {
                let Ok(cpd) = dir::gen3::CodePartitionDirectory::new(data[o..].to_vec(), o) else {
                    continue;
                };
                gen3dirs.push(cpd);
            }
            o += 16;
        }
        println!("Found {} CPDs doing a full scan", gen3dirs.len());
    }

    let mut base = 0;
    while base + 16 + mem::size_of::<fpt::FPT>() <= data.len() {
        // first 16 bytes are potentially other stuff
        let o = base + 16;
        if let Some(r) = fpt::FPT::parse(&data[o..]) {
            let fpt = match r {
                Ok(r) => r,
                Err(e) => {
                    error!("Cannot parse ME FPT entries @ {o:08x}: {e:?}");
                    continue;
                }
            };
            // realign base; what does this indicate?
            if base % 0x1000 != 0 {
                base = o;
                if debug {
                    println!("Realigned FPT base to {o:08x}");
                }
            }

            for e in &fpt.entries {
                let name = match std::str::from_utf8(&e.name) {
                    // some names are shorter than 4 bytes and padded with 0x0
                    Ok(n) => n.trim_end_matches('\0').to_string(),
                    Err(_) => format!("{:02x?}", &e.name),
                };
                let n = u32::from_be_bytes(e.name);
                let o = base + (e.offset & 0x003f_ffff) as usize;
                let s = e.size as usize;
                match n {
                    MDMV | DLMP | FTPR | NFTP => {
                        if o + 4 < data.len() {
                            let buf = &data[o..o + 4];
                            if buf.eq(cpd_bytes) {
                                if let Ok(cpd) = dir::gen3::CodePartitionDirectory::new(
                                    data[o..o + s].to_vec(),
                                    o,
                                ) {
                                    gen3dirs.push(cpd);
                                }
                            } else if let Ok(dir) = dir::gen2::Directory::new(&data[o..], o) {
                                gen2dirs.push(dir);
                            } else if debug {
                                println!("{name} @ {o:08x} has no CPD signature");
                                dump48(&data[o..]);
                            }
                        }
                    }
                    MFS | AFSP | EFFS => {
                        // TODO: parse MFS
                    }
                    _ => {
                        if !debug {
                            continue;
                        }
                        // We may encounter unknown CPDs.
                        if n != FTUP && o + 4 < data.len() {
                            let buf = &data[o..o + 4];
                            if let Ok(sig) = std::str::from_utf8(buf) {
                                if sig == dir::gen3::CPD_MAGIC {
                                    println!("Unknown $CPD in {name} @ 0x{o:08x} (0x{s:08x}).");
                                    continue;
                                }
                            }
                        }
                        if let Ok(m) = dir::man::Manifest::new(&data[o..]) {
                            println!("Manifest found in {name}: {m}");
                            continue;
                        }
                        println!("Cannot (yet) parse {name} @ 0x{o:08x} (0x{s:08x}), skipping...");
                        if debug {
                            dump48(&data[o..]);
                        }
                    }
                }
            }

            let me_fpt = ME_FPT {
                base,
                fpt,
                gen3dirs,
                gen2dirs,
                fit,
            };
            return Ok(me_fpt);
        }
        base += 16;
    }
    Err("No $FPT :(".to_string())
}
