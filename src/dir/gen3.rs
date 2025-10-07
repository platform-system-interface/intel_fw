use crate::dir::man::Manifest;
use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy::FromBytes;
use zerocopy_derive::{FromBytes, IntoBytes};

use crate::meta::get_meta_for_key;

pub const CPD_MAGIC: &str = "$CPD";

// see https://troopers.de/downloads/troopers17/TR17_ME11_Static.pdf
#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct CPDHeader {
    pub magic: [u8; 4],
    pub entries: u32,
    pub version_or_checksum: u32,
    pub part_name: [u8; 4],
    // Some ME variants have an extra 4 bytes here...
    // _10: u32,
}

const HEADER_SIZE: usize = core::mem::size_of::<CPDHeader>();

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct CPDEntry {
    pub name: [u8; 12],
    pub offset: u32,
    pub size: u32,
    pub compression_flag: u32,
}

impl CPDEntry {
    pub fn name(&self) -> String {
        match std::str::from_utf8(&self.name) {
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", &self.name),
        }
    }
}

impl Display for CPDEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.name();
        let o = self.offset;
        let s = self.size;
        let end = o + s;
        let flag = self.compression_flag;

        write!(f, "{n:13} @ 0x{o:06x}:0x{end:06x} (0x{s:06x}) {flag:032b}")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct CodePartitionDirectory {
    pub header: CPDHeader,
    pub manifest: Result<Manifest, String>,
    pub entries: Vec<CPDEntry>,
    pub offset: usize,
    pub name: String,
}

fn stringify_vec(v: Vec<u8>) -> String {
    v.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<String>>()
        .join("")
}

impl Display for CodePartitionDirectory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let checksum = self.header.version_or_checksum;
        let o = self.offset;
        let n = &self.name;
        let l1 = format!("{n} @ {o:08x}, checksum or version: {checksum:08x}");
        let l2 = match &self.manifest {
            Ok(m) => {
                let h = stringify_vec(m.hash_key());
                let m = format!("{m}");
                let kh = format!("Key hash: {h}");
                let me = match get_meta_for_key(h.as_str()) {
                    Some(meta) => format!(", {meta}"),
                    None => String::new(),
                };
                format!("{m}\n{kh}{me}")
            }
            Err(e) => format!("{e}"),
        };
        let l3 = format!("  file name        offset    end       size           compression flags");
        write!(f, "{l1}\n{l2}\n{l3}\n").unwrap();
        let mut entries = self.entries.clone();
        entries.sort_by_key(|e| e.offset);
        for e in entries {
            write!(f, "  {e}\n").unwrap();
        }
        write!(f, "")
    }
}

// TODO: See https://github.com/skochinsky/me-tools class CPDEntry
// What is the other u8?!
const OFFSET_MASK: u32 = 0xffffff;

impl CodePartitionDirectory {
    pub fn new(data: Vec<u8>, offset: usize) -> Result<Self, String> {
        let Ok((header, _)) = CPDHeader::read_from_prefix(&data) else {
            return Err("could not parse CPD header".to_string());
        };
        let n = header.part_name;
        let name = match std::str::from_utf8(&n) {
            // some names are shorter than 4 bytes and padded with 0x0
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", n),
        };
        let mut entries = Vec::<CPDEntry>::new();
        let header_size = if header.version_or_checksum == 0x00140102 {
            HEADER_SIZE + 4
        } else {
            HEADER_SIZE
        };
        for e in 0..header.entries as usize {
            let pos = header_size + e * 24;
            let (mut entry, _) = CPDEntry::read_from_prefix(&data[pos..]).unwrap();
            entry.offset &= OFFSET_MASK;
            entries.push(entry);
        }

        let manifest = {
            let name = format!("{}.man", name);
            if let Some(e) = entries.iter().find(|e| e.name() == name) {
                let b = &data[e.offset as usize..];
                Manifest::new(b)
            } else {
                Err("no manifest found".to_string())
            }
        };

        let cpd = CodePartitionDirectory {
            header,
            manifest,
            entries,
            offset,
            name: name.to_string(),
        };

        Ok(cpd)
    }
}
