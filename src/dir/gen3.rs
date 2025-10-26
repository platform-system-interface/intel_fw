use core::fmt::{self, Display};

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Ref};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::dir::man::Manifest;
use crate::meta::get_meta_for_key;

pub const CPD_MAGIC: &str = "$CPD";
pub const CPD_MAGIC_BYTES: &[u8] = CPD_MAGIC.as_bytes();

// see <https://troopers.de/downloads/troopers17/TR17_ME11_Static.pdf>
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

// See <https://github.com/corna/me_cleaner> `check_and_remove_modules_gen3()`
#[bitfield(u32)]
#[derive(Immutable, FromBytes, IntoBytes, Serialize, Deserialize)]
pub struct FlagsAndOffset {
    #[bits(25)]
    pub offset: u32,
    pub compressed: bool,
    #[bits(6)]
    pub _unknown: u8,
}

// See <https://github.com/skochinsky/me-tools> class `CPDEntry`
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct CPDEntry {
    pub name: [u8; 12],
    pub flags_and_offset: FlagsAndOffset,
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
        let o = self.flags_and_offset.offset();
        let s = self.size;
        let end = o + s;
        // See https://github.com/corna/me_cleaner check_and_remove_modules_gen3
        let meta = {
            if n.ends_with(".met") {
                "metadata"
            } else if n.ends_with(".man") {
                "manifest"
            } else if self.flags_and_offset.compressed() {
                "Huffman"
            } else {
                "uncompressed or LZMA"
            }
        };
        write!(f, "{n:13} @ 0x{o:06x}:0x{end:06x} (0x{s:06x}) {meta:20}")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct CodePartitionDirectory {
    pub header: CPDHeader,
    pub manifest: Result<Manifest, String>,
    pub entries: Vec<CPDEntry>,
    pub offset: usize,
    pub size: usize,
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
        let l3 = format!("  file name        offset    end       size      kind");
        write!(f, "{l1}\n{l2}\n{l3}\n").unwrap();
        let sorted_entries = self.sorted_entries();
        for e in sorted_entries {
            write!(f, "  {e}\n").unwrap();
        }
        write!(f, "")
    }
}

impl CodePartitionDirectory {
    pub fn new(data: Vec<u8>, offset: usize, size: usize) -> Result<Self, String> {
        let Ok((header, _)) = CPDHeader::read_from_prefix(&data) else {
            return Err("could not parse CPD header".to_string());
        };
        let n = header.part_name;
        let name = match std::str::from_utf8(&n) {
            // some names are shorter than 4 bytes and padded with 0x0
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", n),
        };
        let header_size = if header.version_or_checksum == 0x00140102 {
            HEADER_SIZE + 4
        } else {
            HEADER_SIZE
        };
        let pos = header_size;
        let count = header.entries as usize;
        let slice = &data[pos..];
        let Ok((r, _)) = Ref::<_, [CPDEntry]>::from_prefix_with_elems(slice, count) else {
            return Err(format!(
                "cannot parse ME FW Gen 3 directory entries @ {:08x}",
                pos
            ));
        };
        let entries = r.to_vec();

        let manifest = {
            let name = format!("{}.man", name);
            if let Some(e) = entries.iter().find(|e| e.name() == name) {
                let b = &data[e.flags_and_offset.offset() as usize..];
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
            size,
            name: name.to_string(),
        };

        Ok(cpd)
    }

    fn sorted_entries(&self) -> Vec<CPDEntry> {
        let mut entries = self.entries.clone();
        entries.sort_by_key(|e| e.flags_and_offset.offset());
        entries
    }
}
