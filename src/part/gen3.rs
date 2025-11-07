use serde::{Deserialize, Serialize};

use crate::dir::{
    gen3::{ALWAYS_RETAIN, CPD_MAGIC_BYTES, CodePartitionDirectory},
    man::Manifest,
};
use crate::dump48;
use crate::part::{
    fpt::{DIR_PARTS, FPT, FPTEntry, FS_PARTS, FTPR, REMOVABLE_PARTS},
    part::{Partition, UnknownOrMalformedPartition, dir_clean, strs_to_strings},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CPDPartition {
    pub entry: FPTEntry,
    pub data: Vec<u8>,
    pub cpd: CodePartitionDirectory,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataPartition {
    pub entry: FPTEntry,
    pub data: Vec<u8>,
    pub manifest: Manifest,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Gen3Partition {
    Dir(CPDPartition),
    Data(DataPartition),
    MalformedOrUnknown(UnknownOrMalformedPartition),
}

impl Partition for Gen3Partition {
    fn data(&self) -> &Vec<u8> {
        match self {
            Self::Dir(d) => &d.data,
            Self::Data(d) => &d.data,
            Self::MalformedOrUnknown(d) => &d.data,
        }
    }
    fn entry(&self) -> &FPTEntry {
        match self {
            Self::Dir(d) => &d.entry,
            Self::Data(d) => &d.entry,
            Self::MalformedOrUnknown(d) => &d.entry,
        }
    }
    fn set_data(&mut self, data: Vec<u8>) {
        match self {
            Self::Dir(d) => d.data = data,
            Self::Data(d) => d.data = data,
            Self::MalformedOrUnknown(d) => d.data = data,
        }
    }
    fn set_entry(&mut self, entry: FPTEntry) {
        match self {
            Self::Dir(d) => d.entry = entry,
            Self::Data(d) => d.entry = entry,
            Self::MalformedOrUnknown(d) => d.entry = entry,
        }
    }
}

impl Gen3Partition {
    pub fn parse(data: &[u8], entry: FPTEntry, debug: bool) -> Self {
        let o = entry.offset();
        let n = entry.name();
        let n = n.as_str();
        let data = data.to_vec();
        match entry {
            _ if data.len() > 4 && &data[..4] == CPD_MAGIC_BYTES => {
                if !DIR_PARTS.contains(&n) && debug {
                    println!("Unknown CPD {n} @ 0x{o:08x}");
                }
                match CodePartitionDirectory::new(&data, o) {
                    Ok(cpd) => Gen3Partition::Dir(CPDPartition { entry, data, cpd }),
                    Err(e) => {
                        let note =
                            format!("Expected CPD {n} @ 0x{o:08x}, but could not parse it: {e}");
                        Gen3Partition::MalformedOrUnknown(UnknownOrMalformedPartition {
                            entry,
                            data,
                            note,
                        })
                    }
                }
            }
            _ if FS_PARTS.contains(&n) => {
                // TODO: parse MFS
                let note = "file system parsing not yet implemented".to_string();
                Gen3Partition::MalformedOrUnknown(UnknownOrMalformedPartition { entry, data, note })
            }
            _ => {
                if let Ok(manifest) = Manifest::new(&data) {
                    if debug {
                        println!("Manifest found in {n} @ 0x{o:08x}: {manifest}");
                    }
                    return Gen3Partition::Data(DataPartition {
                        entry,
                        data,
                        manifest,
                    });
                }
                let note = format!("Cannot (yet) parse {n} @ 0x{o:08x}, skipping...");
                if debug {
                    println!("{note}");
                    dump48(&data);
                }
                Gen3Partition::MalformedOrUnknown(UnknownOrMalformedPartition { entry, data, note })
            }
        }
    }
}

pub fn parse(fpt: &FPT, data: &[u8], debug: bool) -> Vec<Gen3Partition> {
    let parts = fpt
        .entries
        .iter()
        .map(|e| {
            let offset = e.offset();
            let size = e.size as usize;
            let end = offset + size;
            let l = data.len();
            if end > l {
                let note = format!("{offset:08x}..{end:08x} out of bounds ({l:08x})");
                Gen3Partition::MalformedOrUnknown(UnknownOrMalformedPartition {
                    entry: *e,
                    data: vec![],
                    note,
                })
            } else {
                // NOTE: We pass the exact data slice to be kept by
                // the partition besides its table entry metadata.
                Gen3Partition::parse(&data[offset..end], *e, debug)
            }
        })
        .collect();
    parts
}

pub fn clean(parts: &Vec<Gen3Partition>) -> Vec<Gen3Partition> {
    use log::info;
    // Step 1: Reduce down to the partitions to be kept, i.e., non-removable
    // ones.
    let mut reduced = parts
        .iter()
        .filter(|p| {
            let e = p.entry();
            if REMOVABLE_PARTS.contains(&e.name().as_str()) {
                info!("Remove {e}");
                false
            } else {
                info!("Retain {e}");
                true
            }
        })
        .map(|p| p.clone())
        .collect::<Vec<Gen3Partition>>();
    // Step 2: Clean the FTPR directory, retaining non-removable modules.
    if let Some(p) = reduced.iter_mut().find(|p| p.entry().name() == FTPR) {
        let offset = p.entry().offset();
        info!("FTPR @ {offset:08x}");
        // TODO: Extend with user-provided list
        let retention_list = strs_to_strings(ALWAYS_RETAIN);
        let mut cleaned = p.data().clone();
        match &p {
            Gen3Partition::Dir(dir) => {
                dir_clean(&dir.cpd, &retention_list, &mut cleaned);
            }
            _ => {}
        };
        p.set_data(cleaned);
    }
    // Step 3: Profit.
    reduced
}
