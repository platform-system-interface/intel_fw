use serde::{Deserialize, Serialize};
use zerocopy::IntoBytes;

use crate::dir::gen2::{ALWAYS_RETAIN, Directory, LUT_HEADER_SIZE};
use crate::dump48;
use crate::part::{
    fpt::{FPT, FPTEntry, FTPR},
    part::{
        ClearOptions, DataPartition, Partition, UnknownOrMalformedPartition, dir_clean, retain,
        strs_to_strings,
    },
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DirPartition {
    pub dir: Directory,
    pub entry: FPTEntry,
    pub data: Vec<u8>,
}

impl DirPartition {
    pub fn check_signature(&self) -> Result<(), String> {
        if self.dir.manifest.verify() {
            Ok(())
        } else {
            Err("hash mismatch".into())
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Gen2Partition {
    Dir(Box<DirPartition>),
    Data(DataPartition),
    MalformedOrUnknown(UnknownOrMalformedPartition),
}

impl Partition for Gen2Partition {
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

impl Gen2Partition {
    pub fn parse(data: &[u8], entry: FPTEntry, debug: bool) -> Self {
        let o = entry.offset();
        let data = data.to_vec();
        if let Ok(dir) = Directory::new(&data, o) {
            let p = DirPartition { dir, entry, data };
            Gen2Partition::Dir(Box::new(p))
        } else {
            if debug {
                let n = entry.name();
                println!("Data: {n} @ 0x{o:08x}");
                dump48(&data);
            }
            Gen2Partition::Data(DataPartition { entry, data })
        }
    }

    pub fn relocate(&mut self, offset: u32) -> Result<(), String> {
        match self {
            Self::Dir(p) => {
                let old_offset = p.entry.offset() as u32;
                p.entry.set_offset(offset);
                // rebase Huffman chunks
                let offset_diff = old_offset - offset;
                println!("Adjust Huffman LUT, diff: {offset_diff:08x}");
                p.dir.rebase_huffman_chunks(offset_diff)?;
                if let Some((mod_offset, huffman_mod)) = p.dir.get_huffman_mod() {
                    let o = mod_offset;
                    let l = LUT_HEADER_SIZE;
                    p.data[o..o + l].copy_from_slice(huffman_mod.header.as_bytes());
                    let o = mod_offset + LUT_HEADER_SIZE;
                    let l = huffman_mod.chunks.len() * 4;
                    p.data[o..o + l].copy_from_slice(huffman_mod.chunks.as_bytes());
                }
            }
            Self::Data(p) => p.entry.set_offset(offset),
            Self::MalformedOrUnknown(p) => p.entry.set_offset(offset),
        }
        Ok(())
    }
}

pub fn parse(fpt: &FPT, data: &[u8], debug: bool) -> Vec<Gen2Partition> {
    fpt.entries
        .iter()
        .map(|e| {
            let offset = e.offset();
            let size = e.size as usize;
            let end = offset + size;
            let l = data.len();
            if end > l {
                let note = format!("{offset:08x}..{end:08x} out of bounds ({l:08x})");
                Gen2Partition::MalformedOrUnknown(UnknownOrMalformedPartition {
                    entry: *e,
                    data: vec![],
                    note,
                })
            } else {
                // NOTE: We pass the exact data slice to be kept by
                // the partition besides its table entry metadata.
                Gen2Partition::parse(&data[offset..end], *e, debug)
            }
        })
        .collect()
}

pub fn clean(parts: &[Gen2Partition], options: &ClearOptions) -> Vec<Gen2Partition> {
    use log::info;
    parts
        .iter()
        .filter(|p| {
            let e = p.entry();
            let n = e.name();
            if retain(n, options) {
                info!("Retain {e}");
                true
            } else {
                info!("Remove {e}");
                false
            }
        })
        .map(|p| {
            let mut p = p.clone();
            if p.entry().name() == FTPR && !options.keep_modules {
                let offset = p.entry().offset();
                info!("FTPR @ {offset:08x}");
                // TODO: Extend with user-provided list
                let retention_list = strs_to_strings(ALWAYS_RETAIN);
                let mut cleaned = p.data().clone();
                if let Gen2Partition::Dir(dir) = &p {
                    dir_clean(&dir.dir, &retention_list, &mut cleaned);
                }
                p.set_data(cleaned);
            }
            p
        })
        .collect()
}
