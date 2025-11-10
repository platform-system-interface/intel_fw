use std::ops::Range;

use serde::{Deserialize, Serialize};
use zerocopy::IntoBytes;

use crate::EMPTY;
use crate::dir::gen3::CPD_MAGIC_BYTES;
use crate::part::{
    fpt::{FPT, FPTEntry},
    gen2::{self, Gen2Partition},
    gen3::{self, Gen3Partition},
    part::{GenUnknownPartition, Partition},
};
use crate::ver::Version;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Partitions {
    Gen2(Vec<Gen2Partition>),
    Gen3(Vec<Gen3Partition>),
    Unknown(Vec<GenUnknownPartition>),
}

// https://users.rust-lang.org/t/solved-unified-iteration-over-enum-of-vectors/11830/3
impl<'a> IntoIterator for &'a Partitions {
    type Item = &'a dyn Partition;
    type IntoIter = Box<dyn Iterator<Item = &'a dyn Partition> + 'a>;

    fn into_iter(self) -> Self::IntoIter {
        match *self {
            Partitions::Gen2(ref s) => Box::new(s.into_iter().map(|x| x as &dyn Partition)),
            Partitions::Gen3(ref s) => Box::new(s.into_iter().map(|x| x as &dyn Partition)),
            Partitions::Unknown(ref s) => Box::new(s.into_iter().map(|x| x as &dyn Partition)),
        }
    }
}

impl Partitions {
    pub fn get_entries(self: &Self) -> Vec<FPTEntry> {
        self.into_iter().map(|p| *p.entry()).collect()
    }

    pub fn get_sorted_entries(&self) -> Vec<FPTEntry> {
        let mut entries = self.get_entries().clone();
        entries.sort_by_key(|e| e.offset());
        entries
    }

    pub fn parse(fpt: &FPT, data: &[u8], debug: bool) -> Self {
        let entries = &fpt.entries;
        // NOTE: We can only implicitly decide whether the given image is for
        // the 2nd or 3rd ME generation by looking at the directories themselves.
        // The heuristic may fail though, so we expose the ME generation
        // at a higher level instead where we add more detection features.
        let is_gen3 = entries
            .iter()
            .find(|e| {
                let o = e.offset();
                let l = o + CPD_MAGIC_BYTES.len();
                l < data.len() && data[o..l].eq(CPD_MAGIC_BYTES)
            })
            .is_some();

        let partitions = if is_gen3 {
            let parts = gen3::parse(fpt, data, debug);
            Partitions::Gen3(parts)
        } else {
            let parts = gen2::parse(fpt, data, debug);
            Partitions::Gen2(parts)
        };

        partitions
    }

    pub fn get_me_version(&self) -> Option<Version> {
        match self {
            Partitions::Gen2(parts) => {
                if let Some(Gen2Partition::Dir(d)) =
                    parts.iter().find(|p| matches!(p, Gen2Partition::Dir(_)))
                {
                    Some(d.dir.manifest.header.version)
                } else {
                    None
                }
            }
            Partitions::Gen3(parts) => {
                if let Some(Gen3Partition::Dir(d)) =
                    parts.iter().find(|p| matches!(p, Gen3Partition::Dir(_)))
                {
                    if let Ok(m) = d.cpd.manifest {
                        Some(m.header.version)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Get ranges not covered by FPT entries.
    pub fn non_covered_ranges(&self) -> Vec<Range<usize>> {
        let mut res = vec![];
        for w in self.get_sorted_entries().windows(2) {
            let curr = w[0];
            let next = w[1];
            let o = curr.offset() + curr.size();
            if o < next.offset() {
                let u = o..next.offset();
                res.push(u);
            }
        }
        res
    }

    // TODO: retention list
    /// Clear out fully removable partitions, adjusting header and checksum
    pub fn clear(&mut self) {
        let parts = match &self {
            Partitions::Gen2(parts) => {
                let res = gen2::clean(&parts);
                Partitions::Gen2(res)
            }
            Partitions::Gen3(parts) => {
                let res = gen3::clean(&parts);
                Partitions::Gen3(res)
            }
            Partitions::Unknown(p) => {
                let res = p.iter().map(|p| p.clone()).collect();
                Partitions::Unknown(res)
            }
        };
        *self = parts;
    }

    pub fn get_sorted(&self) -> Self {
        match self {
            Partitions::Gen2(parts) => {
                let mut parts = parts.to_vec();
                parts.sort_by_key(|p| p.entry().offset());
                Partitions::Gen2(parts)
            }
            Partitions::Gen3(parts) => {
                let mut parts = parts.to_vec();
                parts.sort_by_key(|p| p.entry().offset());
                Partitions::Gen3(parts)
            }
            Partitions::Unknown(parts) => {
                let mut parts = parts.to_vec();
                parts.sort_by_key(|p| p.entry().offset());
                Partitions::Unknown(parts)
            }
        }
    }

    pub fn relocate(&mut self, part_name: &str, offset: u32) -> Result<(), String> {
        *self = match self {
            Partitions::Gen2(parts) => {
                let p = parts.iter_mut().find(|p| p.entry().name() == part_name);
                if let Some(p) = p {
                    if let Err(e) = p.relocate(offset) {
                        return Err(format!("Cannot relocate partition: {e}"));
                    }
                }
                Partitions::Gen2(parts.to_vec())
            }
            Partitions::Gen3(parts) => {
                let p = parts.iter_mut().find(|p| p.entry().name() == part_name);
                if let Some(p) = p {
                    if let Err(e) = p.relocate(offset) {
                        return Err(format!("Cannot relocate partition: {e}"));
                    }
                }
                Partitions::Gen3(parts.to_vec())
            }
            Partitions::Unknown(parts) => {
                let parts = parts.to_vec();
                Partitions::Unknown(parts)
            }
        };
        Ok(())
    }

    pub fn to_vec(&self) -> Vec<u8> {
        fn copy_parts(parts: &Vec<&dyn Partition>, data: &mut Vec<u8>) {
            for p in parts {
                let offset = p.entry().offset();
                if offset == 0xffff_ffff {
                    continue;
                }
                let raw_part = p.data().as_bytes();
                let size = raw_part.len();
                let end = offset + size;
                if end <= data.len() {
                    data[offset..end].copy_from_slice(raw_part);
                }
            }
        }

        let sorted_parts = &self.get_sorted();

        // This gets us the smallest possible slice to copy into.
        let last = &sorted_parts.into_iter().last().unwrap().entry();
        let size = last.offset() + last.size();
        let mut data = vec![EMPTY; size];

        match sorted_parts {
            Partitions::Gen2(parts) => copy_parts(
                &parts.iter().map(|p| p as &dyn Partition).collect(),
                &mut data,
            ),
            Partitions::Gen3(parts) => copy_parts(
                &parts.iter().map(|p| p as &dyn Partition).collect(),
                &mut data,
            ),
            Partitions::Unknown(parts) => copy_parts(
                &parts.iter().map(|p| p as &dyn Partition).collect(),
                &mut data,
            ),
        };

        data
    }
}
