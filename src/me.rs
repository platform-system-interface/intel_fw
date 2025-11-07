//! Intel Management Engine Firmware Parser
//!
//! Intel provides a closed-source tool called FIT(C) to their customers for
//! editing firmware images, as referenced in the following documents:
//! - [FSP guide](https://cdrdv2-public.intel.com/334348/5th-gen-core-i5-5350u-eval-kit-fsp-user-guide.pdf)
//! - [TXE guide](https://www.portwell.eu/index.php?eID=dumpFile&t=f&f=10304&token=9d79dec7d7313cf82d445b05ccd5013a6b97ee81&download=)

use serde::{Deserialize, Serialize};

use crate::EMPTY;
use crate::dir::{
    gen2::Directory as Gen2Directory,
    gen3::{CPD_MAGIC_BYTES, CodePartitionDirectory},
};
use crate::part::{
    fpt::{FPT, MIN_FPT_SIZE},
    partitions::Partitions,
};

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub enum Generation {
    Gen1,
    Gen2,
    Gen3,
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Directories {
    Gen2(Vec<Gen2Directory>),
    Gen3(Vec<CodePartitionDirectory>),
    Unknown,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Data {
    pub offset: usize,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FPTArea {
    pub fpt: FPT,
    pub partitions: Partitions,
    pub non_covered: Vec<Data>,
    pub original_size: usize,
}

impl FPTArea {
    /// Clear out fully removable partitions and adjust FPT
    pub fn clean(&mut self) {
        let mut fpt = self.fpt.clone();
        fpt.clear();
        self.fpt = fpt;
        let mut parts = self.partitions.clone();
        parts.clear();
        self.partitions = parts;
        let debug = true;
        if debug {
            for p in &self.partitions {
                println!("Remaining: {}", p.entry());
            }
        }
    }

    /// Clear out fully removable partitions and adjust FPT
    pub fn to_vec(&self) -> Vec<u8> {
        let debug = true;

        if debug {
            println!("Recreate ME region from components");
        }

        let mut res = self.partitions.to_vec();
        if debug {
            println!("  Minimum size: {:08x}", res.len());
        }
        // Restore the original size, so that the resulting slice fully covers
        // the FPT area. The resulting bytes can be used to overwrite the region
        // in a given image, e.g., after cleaning or other partition changes.
        res.resize(self.original_size, EMPTY);
        if debug {
            println!(" Restored size: {:08x}", res.len());
        }

        // Any range within the FPT area may be non-covered.
        for u in &self.non_covered {
            let o = u.offset;
            let s = u.data.len();
            let e = o + s;
            if debug {
                println!("Restore data not covered by FPT @ {o:08x} ({s} bytes)");
            }
            if e < res.len() {
                res[o..e].copy_from_slice(&u.data);
            }
        }

        // NOTE: This *has* to go last; the FPT is not covered by itself.
        let raw_fpt = self.fpt.clone().to_vec();
        let s = self.fpt.original_size;
        if debug {
            println!("Write back FPT ({s} bytes)");
        }
        res[..s].copy_from_slice(&raw_fpt);

        res
    }
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ME {
    pub base: usize,
    pub generation: Generation,
    pub fpt_area: FPTArea,
    // NOTE: There _may_ be directories outside the FPT area.
    // It is yet unclear how they are referenced.
    pub cpds: Vec<CodePartitionDirectory>,
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
            let original_size = data.len();
            let partitions = Partitions::parse(&fpt, data, debug);

            // TODO: use this when stabilized
            /*
            let gen2 = core::intrinsics::discriminant_value(Partitions::Gen2);
            let gen3 = core::intrinsics::discriminant_value(Partitions::Gen3);
            let generation = match partitions {
                gen2 => Generation::Gen2,
                gen3 => Generation::Gen3,
                _ => Generation::Unknown,
            };
            */
            let generation = if matches!(partitions, Partitions::Gen2(_)) {
                Generation::Gen2
            } else {
                Generation::Gen3
            };

            let non_covered = partitions
                .non_covered_ranges()
                .iter()
                .map(|u| {
                    let offset = u.start;
                    let end = u.end;
                    let data = data[offset..end].to_vec();
                    Data { data, offset }
                })
                .collect();

            let fpt_area = FPTArea {
                fpt,
                partitions,
                non_covered,
                original_size,
            };

            // TODO: filter out CPDs that are covered by the FPT
            let cpds = Self::cpd_scan(data);

            Some(Ok(Self {
                base,
                fpt_area,
                generation,
                cpds,
            }))
        } else {
            None
        }
    }

    // Find an FPT in a given slice, and if detected, get the parse result,
    // which includes the offset where it was found as its base address.
    pub fn scan(data: &[u8], debug: bool) -> Option<Result<Self, String>> {
        fn find_me(data: &[u8], debug: bool) -> Option<Result<ME, String>> {
            for o in (0..data.len() - MIN_FPT_SIZE - 0x10).step_by(0x40) {
                if let Some(r) = ME::parse(&data[o..], o, debug) {
                    return Some(r);
                }
            }
            None
        }
        let mut r = find_me(data, debug);

        if let Some(Ok(me)) = &mut r {
            let all_cpds = Self::cpd_scan(data);
            me.cpds = all_cpds;
        }

        r
    }

    // Scan for all CPDs (there may be some not listed in FPT)
    pub fn cpd_scan(data: &[u8]) -> Vec<CodePartitionDirectory> {
        let mut gen3dirs = Vec::<CodePartitionDirectory>::new();
        let mut o = 0;
        while o < data.len() {
            if &data[o..o + 4] == CPD_MAGIC_BYTES {
                let Ok(cpd) = CodePartitionDirectory::new(&data[o..], o) else {
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
