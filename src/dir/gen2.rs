use core::fmt::{self, Display};
use core::str::from_utf8;

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Ref};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::dir::man::{self, Manifest};

const ENTRY_MAGIC: &str = "$MME";
const ENTRY_MAGIC_BYTES: &[u8] = ENTRY_MAGIC.as_bytes();
pub const SIG_LUT: &str = "LLUT";
pub const SIG_LUT_BYTES: &[u8] = SIG_LUT.as_bytes();
pub const SIG_LZMA_BYTES: &[u8] = &[0x36, 0x00, 0x40, 0x00];

// https://github.com/skochinsky/me-tools me_unpack.py MeModuleHeader2
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Entry {
    pub magic: [u8; 4],
    pub name: [u8; 0x10],
    pub hash: [u8; 0x20],
    pub mod_base: u32,     // e.g. 0x0200_9000
    pub offset: u32,       // e.g. 0x0001_5b4a
    pub code_size: u32,    // e.g. 0x0004_2000
    pub size: u32,         // e.g. 0x0001_d13b
    pub memory_size: u32,  // e.g. 0x0004_b425
    pub pre_uma_size: u32, // e.g. 0x0004_b425 (often same as memory_size)
    pub entry_point: u32,  // e.g. 0x2009_1000
    pub flags: Flags,      // e.g. 0x0010_d42a
    pub _54: u32,          // e.g. 0x0000_0008
    pub _58: u32,          // so far all 0
    pub _5c: u32,          // so far all 0
}

#[bitfield(u32)]
#[derive(Immutable, FromBytes, IntoBytes, Serialize, Deserialize)]
pub struct Flags {
    #[bits(4)]
    _r: u8,
    #[bits(3)]
    pub compression: Compression,
    _r: bool,

    _r: u8,

    _r: bool,
    #[bits(3)]
    rapi: usize,
    #[bits(2)]
    kapi: usize,
    #[bits(2)]
    _r: u8,

    _r: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    Uncompressed,
    Huffman,
    Lzma,
    Unknown,
}

impl Compression {
    const fn from_bits(val: u8) -> Self {
        match val {
            0 => Self::Uncompressed,
            1 => Self::Huffman,
            2 => Self::Lzma,
            _ => Self::Unknown,
        }
    }

    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BinaryMap {
    pub rapi: usize, // 3 bits, really
    pub kapi: usize, // 2 bits, really
    pub code_start: usize,
    pub code_end: usize,
    pub data_end: usize,
}

impl Entry {
    pub fn bin_map(&self) -> BinaryMap {
        let b = self.mod_base;
        let rapi = self.flags.rapi();
        let kapi = self.flags.kapi();
        let code_start = (b as usize + (rapi + kapi) * 0x1000) as usize;
        let code_end = (b + self.code_size) as usize;
        let data_end = (b + self.memory_size) as usize;
        BinaryMap {
            rapi,
            kapi,
            code_start,
            code_end,
            data_end,
        }
    }
}

impl Display for BinaryMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let r = self.rapi;
        let k = self.kapi;
        let s = self.code_start;
        let e = self.code_end;
        let de = self.data_end;
        write!(
            f,
            "RAPI {r:03b} KAPI {k:02b} code {s:08x}:{e:08x}, data end {de:08x}"
        )
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = match from_utf8(&self.name) {
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", self.name),
        };
        let o = self.offset;
        let s = self.size;
        let e = self.entry_point;
        let c = self.flags.compression();
        write!(f, "{n:16} {s:08x} @ {o:08x}, entry point {e:08x}, {c:10?}")
    }
}

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Header {
    name: [u8; 4],
    _pad: [u8; 8],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct Directory {
    pub manifest: Manifest,
    pub header: Header,
    pub entries: Vec<Entry>,
    pub offset: usize,
    pub name: String,
}

impl Display for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = &self.name;
        let o = self.offset;
        let m = self.manifest;
        write!(f, "{n} @ {o:08x}, {m}")
    }
}

const HEADER_SIZE: usize = core::mem::size_of::<Header>();

impl Directory {
    pub fn new(data: &[u8], offset: usize) -> Result<Self, String> {
        let Ok(manifest) = Manifest::new(data) else {
            return Err("cannot parse Gen 2 directory manifest".to_string());
        };
        let count = manifest.header.entries as usize;
        let d = &data[man::MANIFEST_SIZE..];
        let Ok((header, _)) = Header::read_from_prefix(d) else {
            return Err("cannot parse ME FW Gen 2 directory header".to_string());
        };
        let pos = man::MANIFEST_SIZE + HEADER_SIZE;
        let m = &data[pos..pos + 4];

        if !m.eq(ENTRY_MAGIC_BYTES) {
            return Err(format!(
                "entry magic not found, got {m:02x?}, wanted {ENTRY_MAGIC_BYTES:02x?} ({ENTRY_MAGIC})"
            ));
        }

        let slice = &data[pos..];
        let Ok((r, _)) = Ref::<_, [Entry]>::from_prefix_with_elems(slice, count) else {
            return Err(format!(
                "cannot parse ME FW Gen 2 directory entries @ {:08x}",
                pos
            ));
        };
        let entries = r.to_vec();
        let name = match from_utf8(&header.name) {
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", header.name),
        };
        Ok(Self {
            manifest,
            header,
            entries,
            offset,
            name,
        })
    }
}
