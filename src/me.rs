//! Intel Management Engine Firmware Parser
//!
//! Intel provides a closed-source tool called FIT(C) to their customers for
//! editing firmware images, as referenced in the following documents:
//! - [FSP guide](https://cdrdv2-public.intel.com/334348/5th-gen-core-i5-5350u-eval-kit-fsp-user-guide.pdf)
//! - [TXE guide](https://www.portwell.eu/index.php?eID=dumpFile&t=f&f=10304&token=9d79dec7d7313cf82d445b05ccd5013a6b97ee81&download=)

use serde::{Deserialize, Serialize};

use crate::dir::gen2::Module;
use crate::dir::{
    gen2::Directory as Gen2Directory,
    gen3::{CPD_MAGIC_BYTES, CodePartitionDirectory},
};
use crate::ifwi::{BPDT, PreIFWI};
use crate::part::{
    fpt::{FPT, FTPR, MIN_FPT_SIZE},
    gen2::{DirPartition, Gen2Partition},
    gen3::{CPDPartition, Gen3Partition},
    generic::{ClearOptions, Partition},
    partitions::Partitions,
};
use crate::ver::Version;
use crate::{EMPTY, dump48};

const PROBE_IFWI: bool = false;

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
    pub fn clean(&mut self, options: &ClearOptions) {
        let mut fpt = self.fpt.clone();
        fpt.clear(options);
        self.fpt = fpt;
        let mut parts = self.partitions.clone();
        parts.clear(options);
        self.partitions = parts;
        let debug = true;
        if debug {
            for p in &self.partitions {
                println!("Remaining: {}", p.entry());
            }
        }
    }

    pub fn files_for_dir(&self, part_name: &String) -> Vec<(String, Vec<u8>)> {
        match &self.partitions {
            Partitions::Gen2(parts) => {
                let dir = parts.iter().find(|p| p.entry().name() == *part_name);
                match dir {
                    Some(Gen2Partition::Dir(d)) => {
                        let mut res = vec![];
                        for m in &d.dir.modules {
                            match m {
                                Module::Uncompressed(e) => {
                                    let o = e.offset as usize;
                                    let s = e.size as usize;
                                    let data = d.data[o..o + s].to_vec();
                                    let name = e.name();
                                    res.push((name, data));
                                }
                                Module::Lzma(Ok(e)) => {
                                    let o = e.offset as usize;
                                    let s = e.size as usize;
                                    let data = d.data[o..o + s].to_vec();
                                    let name = e.name();
                                    res.push((name, data));
                                }
                                _ => {}
                            }
                        }
                        res
                    }
                    _ => vec![],
                }
            }
            Partitions::Gen3(parts) => {
                let dir = parts.iter().find(|p| p.entry().name() == *part_name);
                match dir {
                    Some(Gen3Partition::Dir(d)) => {
                        let mut res = vec![];
                        for e in &d.cpd.entries {
                            let f = e.flags_and_offset;
                            let o = f.offset() as usize;
                            let s = e.size as usize;
                            let data = d.data[o..o + s].to_vec();
                            let name = e.name();
                            res.push((name, data));
                        }
                        res
                    }
                    _ => vec![],
                }
            }
            _ => vec![],
        }
    }

    pub fn check_dir_sigs(&self) -> Vec<(String, Result<(), String>)> {
        match &self.partitions {
            Partitions::Gen2(parts) => {
                let dirs = parts
                    .iter()
                    .filter_map(|p| match p {
                        Gen2Partition::Dir(d) => Some(d.clone()),
                        _ => None,
                    })
                    .collect::<Vec<Box<DirPartition>>>();
                dirs.iter()
                    .map(|d| (d.entry.name(), d.check_signature()))
                    .collect()
            }
            Partitions::Gen3(parts) => {
                let dirs = parts
                    .iter()
                    .filter_map(|p| match p {
                        Gen3Partition::Dir(d) => Some(d.clone()),
                        _ => None,
                    })
                    .collect::<Vec<CPDPartition>>();
                dirs.iter()
                    .map(|d| (d.entry.name(), d.check_signature()))
                    .collect()
            }
            _ => vec![],
        }
    }

    pub fn check_ftpr_presence(&self) -> Result<(), String> {
        match &self.partitions {
            Partitions::Gen2(parts) => {
                if parts.iter().any(|p| p.entry().name() == FTPR) {
                    Ok(())
                } else {
                    Err("not found".into())
                }
            }
            Partitions::Gen3(parts) => {
                if parts.iter().any(|p| p.entry().name() == FTPR) {
                    Ok(())
                } else {
                    Err("not found".into())
                }
            }
            _ => Err("not recognized as ME generation 2 or 3".into()),
        }
    }

    pub fn relocate_partitions(&mut self) -> Result<(), String> {
        let sorted_parts = &self.partitions.get_sorted();
        let first = sorted_parts
            .into_iter()
            .filter(|p| p.entry().size() > 0)
            .nth(0);
        if let Some(p) = first {
            let e = p.entry();
            let n = e.name();
            let min_offset = MIN_FPT_SIZE;
            let new_offset = match &self.partitions {
                Partitions::Gen2(parts) => {
                    // Find a Directory partition to calculate the offset.
                    let dir_part = parts.iter().find(|p| matches!(p, Gen2Partition::Dir(_)));
                    match dir_part {
                        Some(Gen2Partition::Dir(d)) => d.dir.calc_new_offset(min_offset as u32)?,
                        _ => return Err("no directory partition found".into()),
                    }
                }
                Partitions::Gen3(_) => e.offset().min(min_offset) as u32,
                _ => todo!(),
            };

            let old_offset = e.offset();
            println!("old offset: {old_offset:08x}");
            println!("new offset: {new_offset:08x}");

            if let Err(e) = self.partitions.relocate(&n, new_offset) {
                return Err(format!("Cannot relocate partitions: {e}"));
            }

            let offset = new_offset as usize;
            // It may happen that some part of the FPT area had not been covered
            // by an FPT entry, but it contained data that can now be overwritten
            // when relocating partitions.
            let mut non_covered = vec![];
            let r = offset..offset + e.size();
            for nc in &self.non_covered {
                let o = nc.offset;
                let e = o + nc.data.len();
                if r.contains(&o) || r.contains(&e) {
                    println!("Drop data not covered by FPT and now overlapping: {o:08x}..{e:08x}");
                } else {
                    non_covered.push(nc.clone());
                }
            }
            self.non_covered = non_covered;

            // Update partition table entry
            for e in &mut self.fpt.entries {
                if e.name() == n {
                    e.set_offset(offset as u32);
                }
            }

            Ok(())
        } else {
            Err("no FPT partitions found".into())
        }
    }

    /// Clear out fully removable partitions and adjust FPT
    pub fn to_vec(&self) -> Result<Vec<u8>, String> {
        let debug = true;

        if debug {
            println!("Recreate ME region from components");
        }

        let mut res = self.partitions.to_vec()?;
        // Round to next 4k
        let min_size = res.len().next_multiple_of(4096);
        if debug {
            println!("  Minimum size: {:08x}", min_size);
        }
        res.resize(min_size, EMPTY);

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

        Ok(res)
    }
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ME {
    pub base: usize,
    pub generation: Generation,
    pub version: Option<Version>,
    pub fpt_area: FPTArea,
    // NOTE: There _may_ be directories outside the FPT area.
    // It is yet unclear how they are referenced.
    pub cpds: Vec<CodePartitionDirectory>,
}

const DUMP: bool = true;

/// Print a BPDT
fn print_bpdt(bpdt: &BPDT, data: &[u8], offset: usize) {
    let bpdt_offset = offset + bpdt.offset as usize;
    let h = bpdt.header;
    println!("{h}  @ {bpdt_offset:08x}");
    for e in &bpdt.entries {
        println!("{e}");
        if e.offset > 0 && DUMP {
            let o = bpdt_offset + e.offset as usize;
            if o < data.len() {
                dump48(&data[o..]);
            }
        }
    }
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

            let version = partitions.get_me_version();

            let non_covered = partitions
                .non_covered_ranges()
                .iter()
                .map(|u| {
                    let offset = u.start;
                    let end = u.end;
                    if offset > data.len() || end > data.len() {
                        return Data {
                            data: vec![],
                            offset,
                        };
                    }
                    let data = data[offset..end].to_vec();
                    Data { offset, data }
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
                generation,
                version,
                fpt_area,
                cpds,
            }))
        } else {
            if PROBE_IFWI {
                match PreIFWI::parse(data) {
                    Ok(pre_ifwi) => {
                        let h = pre_ifwi.header;
                        println!("{h:02x?}");
                        for e in pre_ifwi.entries {
                            println!("- {e:02x?}");
                            if e.offset > 0 {
                                let o = e.offset as usize;
                                let slice = &data[o..];
                                // entries are either FPT or BPDTs
                                match FPT::parse(slice) {
                                    Some(Ok(fpt)) => {
                                        println!("FPT: {}", fpt.header);
                                        continue;
                                    }
                                    Some(Err(e)) => println!("FPT: {e:?}"),
                                    _ => {}
                                }
                                match BPDT::parse(slice, o) {
                                    Ok(bpdt) => {
                                        print_bpdt(&bpdt, data, o);
                                        continue;
                                    }
                                    Err(e) => println!("BPDT: {e:?}"),
                                }
                                if o + 48 < data.len() {
                                    dump48(slice);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        println!("Pre-IFWI: {e:?}");
                        match Self::bpdt_scan(data) {
                            Ok(bpdt) => {
                                let bpdt_offset = bpdt.offset;
                                print_bpdt(&bpdt, data, 0);
                                match bpdt.next(&data[bpdt_offset..]) {
                                    Some(Ok(bpdt)) => print_bpdt(&bpdt, data, bpdt_offset),
                                    Some(Err(e)) => println!("{e:?}"),
                                    _ => println!("nope"),
                                }
                            }
                            Err(e) => println!("{e}"),
                        }
                    }
                }
            }
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

    // TODO: we shouldn't have to scan...
    pub fn bpdt_scan(data: &[u8]) -> Result<BPDT, String> {
        for o in (0..data.len()).step_by(0x40) {
            if let Ok(r) = BPDT::parse(&data[o..], o) {
                return Ok(r);
            }
        }
        Err("not found".into())
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
