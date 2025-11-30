#![doc = include_str!("../README.md")]

use log::{info, warn};
use serde::{Deserialize, Serialize};

pub mod dir;
pub mod fit;
pub mod ifd;
pub mod ifwi;
pub mod me;
pub mod meta;
pub mod part;
pub mod ver;

use fit::{Fit, FitError};
use ifd::{IFD, IfdError};
use me::ME;

// An empty byte in a NOR flash is all-1's.
pub const EMPTY: u8 = 0xff;

/// Common method for anything that has removable parts, such as directories.
pub trait Removables {
    /// Get removable ranges relative to the start of a section or directory.
    /// The respective section/directory needs to know its own offset.
    fn removables(&self, retention_list: &[String]) -> Vec<core::ops::Range<usize>>;
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Firmware {
    pub ifd: Result<IFD, IfdError>,
    pub me: Option<Result<ME, String>>,
    pub fit: Result<Fit, FitError>,
}

const DUMP: bool = false;

impl Firmware {
    pub fn parse(data: &[u8], debug: bool) -> Self {
        let ifd = IFD::parse(data);
        let me = match &ifd {
            Ok(ifd) => {
                let me_region = ifd.regions.me_range();
                let b = me_region.start;
                info!("ME region start @ {b:08x}");
                let l = data.len();
                if b > l {
                    warn!("ME region out of bounds, only got {l:08x}");
                    // TODO: work out smth regarding offsets
                    // v2 has another struct first, which has not magic...
                    if let Ok(bpdt) = ME::bpdt_scan(&data[4096..]) {
                        let o = bpdt.offset;
                        let h = bpdt.header;
                        let bpdt_offset = o + 4096;
                        println!("{h}  @ {o:08x}");
                        for e in &bpdt.entries {
                            println!("{e}");
                            if e.offset > 0 && DUMP {
                                let o = bpdt_offset + e.offset as usize;
                                if o < data.len() {
                                    dump48(&data[o..]);
                                }
                            }
                        }

                        match bpdt.next(&data[bpdt_offset..]) {
                            Some(Ok(bpdt)) => {
                                let o = bpdt.offset;
                                let h = bpdt.header;
                                println!();
                                println!("{h}  @ {o:08x}");
                                for e in bpdt.entries {
                                    println!("{e}");
                                    if e.offset > 0 && DUMP {
                                        let o = bpdt_offset + e.offset as usize;
                                        if o < data.len() {
                                            dump48(&data[o..]);
                                        }
                                    }
                                }
                            }
                            Some(Err(e)) => println!("{e:?}"),
                            _ => println!("no sub-partition"),
                        }
                    }
                    None
                } else {
                    ME::parse(&data[me_region], b, debug)
                }
            }
            Err(e) => {
                warn!("Not a full image: {e:?}");
                ME::parse(data, 0, debug)
            }
        };
        let fit = Fit::new(data);
        Self { ifd, me, fit }
    }

    pub fn scan(data: &[u8], debug: bool) -> Self {
        let ifd = IFD::parse(data);
        let me = ME::scan(data, debug);
        let fit = Fit::new(data);
        Self { ifd, me, fit }
    }
}

/// Dump first 48 bytes of a slice. Meant solely for debugging.
/// Tip: If you need context to investigate data, pass a reference to
/// `data[offset-16..]` instead of `data[offset..]`.
pub(crate) fn dump48(data: &[u8]) {
    println!("Here are 48 bytes:");
    println!(" {:02x?}", &data[0x00..0x10]);
    println!(" {:02x?}", &data[0x10..0x20]);
    println!(" {:02x?}", &data[0x20..0x30]);
}
