use core::fmt::{self, Display};
use core::ops::Range;

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Ref};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::Removables;
use crate::dir::man::Manifest;
use crate::meta::get_meta_for_key;

// These must never be removed. They are essential for platform initialization.
pub const ALWAYS_RETAIN: &[&str] = &[
    "bup",    // bringup
    "rbe",    //
    "kernel", //
    "syslib", //
];

pub const CPD_MAGIC: &str = "$CPD";
pub const CPD_MAGIC_BYTES: &[u8] = CPD_MAGIC.as_bytes();

const FOUR_K: usize = 0x1000;

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

impl CPDHeader {
    pub fn name(&self) -> String {
        let n = self.part_name;
        match std::str::from_utf8(&n) {
            // some names are shorter than 4 bytes and padded with 0x0
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", n),
        }
    }
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
            Err(e) => e.to_string(),
        };
        let l3 = "  file name        offset    end       size      kind".to_string();
        write!(f, "{l1}\n{l2}\n{l3}\n")?;
        let sorted_entries = self.sorted_entries();
        for e in sorted_entries {
            writeln!(f, "  {e}")?;
        }
        write!(f, "")
    }
}

impl CodePartitionDirectory {
    pub fn new(data: &[u8], offset: usize) -> Result<Self, String> {
        let Ok((header, _)) = CPDHeader::read_from_prefix(data) else {
            return Err("could not parse CPD header".to_string());
        };
        let name = header.name();
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
                offset + pos
            ));
        };
        let entries = r.to_vec();

        let manifest_name = format!("{name}.man");
        let manifest = {
            if let Some(e) = entries.iter().find(|e| e.name() == manifest_name) {
                let o = e.flags_and_offset.offset() as usize;
                let end = o + e.size as usize;
                Manifest::new(&data[o..end])
            } else {
                Err("no manifest found".to_string())
            }
        };

        let size = data.len();
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

    // Entries sorted by offset
    fn sorted_entries(&self) -> Vec<CPDEntry> {
        let mut entries = self.entries.clone();
        entries.sort_by_key(|e| e.flags_and_offset.offset());
        entries
    }
}

impl Removables for CodePartitionDirectory {
    /// Removable ranges relative to the start of the directory
    fn removables(&self, retention_list: &[String]) -> Vec<Range<usize>> {
        use log::info;
        let mut removables = vec![];

        for entry in &self.entries {
            let o = entry.flags_and_offset.offset() as usize;
            let e = o + entry.size as usize;
            let n = entry.name();
            let r = format!("{o:08x}..{e:08x}: {n:14}");
            match &n {
                n if n.ends_with(".man") => info!("Retain manifest  {r}"),
                n if n.ends_with(".met") => info!("Retain metadata  {r}"),
                n if retention_list.contains(n) => info!("Retain necessary {r}"),
                _ => {
                    info!("Remove           {r}");
                    removables.push(o..e);
                }
            }
        }
        // Remaining space to free after last entry
        let sorted = self.sorted_entries();
        if let Some(last) = sorted.last() {
            let end = (last.flags_and_offset.offset() + last.size) as usize;
            let o = end.next_multiple_of(FOUR_K);
            let e = self.size;
            removables.push(o..e);
            info!("Remaining space: {o:08x}..{e:08x}");
        }
        removables
    }
}
