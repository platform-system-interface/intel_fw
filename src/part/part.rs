use serde::{Deserialize, Serialize};

use crate::{EMPTY, Removables, part::fpt::FPTEntry};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DataPartition {
    pub entry: FPTEntry,
    pub data: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct UnknownOrMalformedPartition {
    pub entry: FPTEntry,
    pub data: Vec<u8>,
    pub note: String,
}

pub trait Partition {
    fn entry(&self) -> &FPTEntry;
    fn data(&self) -> &Vec<u8>;
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
}

pub fn strs_to_strings(strs: &[&str]) -> Vec<String> {
    Vec::from(strs).iter().map(|s| String::from(*s)).collect()
}

// Clear out removable ranges in the FTPR directory
pub fn dir_clean(dir: &dyn Removables, retention_list: &Vec<String>, data: &mut Vec<u8>) {
    use log::info;
    for r in dir.removables(&retention_list) {
        let offset = r.start;
        let size = r.end - r.start;
        info!("Freeing {size:8} bytes @ {offset:08x}");
        for o in r {
            data[o] = EMPTY;
        }
    }
}
