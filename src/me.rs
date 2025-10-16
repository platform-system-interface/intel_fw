use serde::{Deserialize, Serialize};

use crate::{
    dir::{
        gen2::Directory as Gen2Directory,
        gen3::{CPD_MAGIC_BYTES, CodePartitionDirectory},
        man::Manifest,
    },
    fpt::{AFSP, DLMP, EFFS, FPT, FTPR, FTUP, MDMV, MFS, NFTP},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Generation {
    Gen1,
    Gen2,
    Gen3,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Directories {
    Gen2(Vec<Gen2Directory>),
    Gen3(Vec<CodePartitionDirectory>),
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ME {
    pub base: usize,
    pub generation: Generation,
    pub fpt: FPT,
    pub dirs: Directories,
}

fn dump48(data: &[u8]) {
    println!("Here are the first 48 bytes:");
    let b = &data[0..0x10];
    println!("{b:02x?}");
    let b = &data[0x10..0x20];
    println!("{b:02x?}");
    let b = &data[0x20..0x30];
    println!("{b:02x?}");
}

impl ME {
    pub fn parse(data: &[u8], base: usize, debug: bool) -> Option<Result<Self, String>> {
        if let Some(r) = FPT::parse(data) {
            let fpt = match r {
                Ok(r) => r,
                Err(e) => {
                    return Some(Err(format!("Cannot parse ME FPT @ {base:08x}: {e:?}")));
                }
            };

            let mut gen2dirs = Vec::<Gen2Directory>::new();
            let mut gen3dirs = Vec::<CodePartitionDirectory>::new();
            // NOTE: We can only implicitly decide whether the given image is for
            // the 2nd or 3rd ME generation by looking at the directories themselves.
            for e in &fpt.entries {
                let name = match std::str::from_utf8(&e.name) {
                    // some names are shorter than 4 bytes and padded with 0x0
                    Ok(n) => n.trim_end_matches('\0').to_string(),
                    Err(_) => format!("{:02x?}", &e.name),
                };
                let n = u32::from_be_bytes(e.name);
                let o = (e.offset & 0x003f_ffff) as usize;
                let s = e.size as usize;
                match n {
                    MDMV | DLMP | FTPR | NFTP => {
                        if o + 4 < data.len() {
                            let buf = &data[o..o + 4];
                            if buf.eq(CPD_MAGIC_BYTES) {
                                if let Ok(cpd) =
                                    CodePartitionDirectory::new(data[o..o + s].to_vec(), o)
                                {
                                    gen3dirs.push(cpd);
                                }
                            } else if let Ok(dir) = Gen2Directory::new(&data[o..], o) {
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
                            if buf == CPD_MAGIC_BYTES {
                                println!("Unknown $CPD in {name} @ 0x{o:08x} (0x{s:08x}).");
                                continue;
                            }
                        }
                        if let Ok(m) = Manifest::new(&data[o..]) {
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

            let (generation, dirs) = {
                if gen3dirs.len() > 0 {
                    (Generation::Gen3, Directories::Gen3(gen3dirs))
                } else if gen2dirs.len() > 0 {
                    (Generation::Gen2, Directories::Gen2(gen2dirs))
                } else {
                    return None;
                }
            };

            Some(Ok(Self {
                base,
                generation,
                fpt,
                dirs,
            }))
        } else {
            None
        }
    }

    // Scan for all CPDs (there may be some not listed in FPT)
    pub fn cpd_scan(data: &[u8]) -> Vec<CodePartitionDirectory> {
        let mut gen3dirs = Vec::<CodePartitionDirectory>::new();
        let mut o = 0;
        while o < data.len() {
            let buf = &data[o..o + 4];
            if buf.eq(CPD_MAGIC_BYTES) {
                let Ok(cpd) = CodePartitionDirectory::new(data[o..].to_vec(), o) else {
                    continue;
                };
                gen3dirs.push(cpd);
            }
            o += 16;
        }
        if false {
            println!("Found {} CPDs doing a full scan", gen3dirs.len());
        }
        gen3dirs
    }
}
