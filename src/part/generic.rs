/// Generic data structures and helpers for partitions
use serde::{Deserialize, Serialize};

use crate::{
    EMPTY, Removables,
    part::fpt::{FPTEntry, FTPR},
};

/// Data partitions are for now treated uniformly, but may carry semantics.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataPartition {
    pub entry: FPTEntry,
    pub data: Vec<u8>,
}

/// Last resort if a partition cannot be classified
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnknownOrMalformedPartition {
    pub entry: FPTEntry,
    pub data: Vec<u8>,
    pub note: String,
}

/// Common trait for partitions and their relationship with the FPT
pub trait Partition {
    fn entry(&self) -> &FPTEntry;
    fn data(&self) -> &Vec<u8>;
    fn set_entry(&mut self, entry: FPTEntry);
    fn set_data(&mut self, data: Vec<u8>);
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum GenUnknownPartition {
    Data(DataPartition),
    Unknown(UnknownOrMalformedPartition),
}

impl Partition for GenUnknownPartition {
    fn data(&self) -> &Vec<u8> {
        match self {
            Self::Data(d) => &d.data,
            Self::Unknown(d) => &d.data,
        }
    }
    fn entry(&self) -> &FPTEntry {
        match self {
            Self::Data(d) => &d.entry,
            Self::Unknown(d) => &d.entry,
        }
    }
    fn set_data(&mut self, data: Vec<u8>) {
        match self {
            Self::Data(d) => d.data = data,
            Self::Unknown(d) => d.data = data,
        }
    }
    fn set_entry(&mut self, entry: FPTEntry) {
        match self {
            Self::Data(d) => d.entry = entry,
            Self::Unknown(d) => d.entry = entry,
        }
    }
}

pub fn strs_to_strings(strs: &[&str]) -> Vec<String> {
    Vec::from(strs).iter().map(|s| String::from(*s)).collect()
}

/// Options for clearing partitions and directories
pub struct ClearOptions {
    pub keep_modules: bool,
    pub parts_force_retention: Vec<String>,
    pub parts_force_deletion: Vec<String>,
}

/// Shared logic for deciding whether a partition should be retained.
pub fn retain(part_name: String, options: &ClearOptions) -> bool {
    part_name == FTPR
        || options.parts_force_retention.contains(&part_name)
        || (!options.parts_force_deletion.is_empty()
            && !options.parts_force_deletion.contains(&part_name))
}

/// Clear out removable ranges in the FTPR directory
pub fn dir_clean(dir: &dyn Removables, retention_list: &[String], data: &mut [u8]) {
    use log::info;
    for r in dir.removables(retention_list) {
        let offset = r.start;
        let size = r.end - r.start;
        info!("Freeing {size:8} bytes @ {offset:08x}");
        for o in r {
            data[o] = EMPTY;
        }
    }
}
