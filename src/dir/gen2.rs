use core::fmt::{self, Display};
use core::ops::Range;
use core::str::from_utf8;

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Ref};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::Removables;
use crate::dir::man::{self, Manifest};

// These must never be removed. They are essential for platform initialization.
pub const ALWAYS_RETAIN: &[&str] = &[
    "BUP",  // bringup
    "ROMP", //
];

const MODULE_MAGIC: &str = "$MME";
const MODULE_MAGIC_BYTES: &[u8] = MODULE_MAGIC.as_bytes();
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

    pub fn name(&self) -> String {
        match std::str::from_utf8(&self.name) {
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", &self.name),
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
pub struct HuffmanHeader {
    magic: [u8; 4],
    chunk_count: u32,
    chunk_base: u32,
    _unk0: u32,
    hs0: u32,
    hs1: u32,
    _unk1: u32,
    _unk2: u32,
    _r: [u8; 16], // all 0 in sample
    chunk_size: u32,
    _unk3: u32,
    name: [u8; 8],
}

const HUFFMAN_HEADER_SIZE: usize = size_of::<HuffmanHeader>();

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct Huffman {
    header: HuffmanHeader,
    chunks: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Module {
    Uncompressed(Entry),
    Huffman(Result<(Entry, Huffman), String>),
    Lzma(Result<Entry, String>),
    Unknown(Entry),
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
    pub modules: Vec<Module>,
    pub offset: usize,
    pub size: usize,
    pub name: String,
}

impl Display for Directory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = &self.name;
        let o = self.offset;
        let s = self.size;
        let m = self.manifest;
        write!(f, "{n} @ {o:08x}, {s} bytes, {m}")
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

        if !m.eq(MODULE_MAGIC_BYTES) {
            return Err(format!(
                "entry magic not found, got {m:02x?}, wanted {MODULE_MAGIC_BYTES:02x?} ({MODULE_MAGIC})"
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

        // Check for consistency and wrap entries with additional information.
        let modules = entries
            .iter()
            .map(|e| {
                let c = e.flags.compression();
                let o = e.offset as usize;
                let sig = &data[o..o + 4];
                match c {
                    Compression::Huffman => {
                        if sig != SIG_LUT_BYTES {
                            return Module::Huffman(Err(format!(
                                "Expected {SIG_LUT_BYTES:02x?} @ {o:08x}, got {sig:02x?}"
                            )));
                        }
                        let (header, _) = HuffmanHeader::read_from_prefix(&data[o..]).unwrap();
                        let count = header.chunk_count as usize;
                        let co = o + HUFFMAN_HEADER_SIZE;
                        let (chunks, _) =
                            Ref::<_, [u32]>::from_prefix_with_elems(&data[co..], count).unwrap();
                        let huff = Huffman {
                            header,
                            chunks: chunks.to_vec(),
                        };
                        Module::Huffman(Ok((*e, huff)))
                    }
                    Compression::Lzma => {
                        if sig != SIG_LZMA_BYTES {
                            return Module::Lzma(Err(format!(
                                "Expected {SIG_LZMA_BYTES:02x?} @ {o:08x}, got {sig:02x?}"
                            )));
                        }
                        Module::Lzma(Ok(*e))
                    }
                    Compression::Uncompressed => Module::Uncompressed(*e),
                    Compression::Unknown => Module::Unknown(*e),
                }
            })
            .collect();

        let size = data.len();
        Ok(Self {
            manifest,
            header,
            modules,
            offset,
            size,
            name,
        })
    }

    fn dump_ranges(ranges: &Vec<Range<usize>>) {
        let group_size = 4;
        for (i, r) in ranges.iter().enumerate() {
            if i % group_size == group_size - 1 {
                println!("{r:08x?}");
            } else {
                print!("{r:08x?} ");
            }
        }
        println!();
    }

    // Get the offset ranges of the chunks.
    fn chunks_as_ranges(self: &Self, chunks: &Vec<u32>, stream_end: usize) -> Vec<Range<usize>> {
        // NOTE: This is the end of the directory.
        // me_cleaner uses the end of the ME region.
        let dir_end = self.offset + self.size;
        let mut nonzero_offsets = vec![stream_end];
        let offsets = chunks
            .iter()
            .map(|c| {
                let o = *c as usize;
                // Highest byte contains flags. 0x80 means inactive.
                const CHUNK_INACTIVE: usize = 0x80;
                if o >> 24 == CHUNK_INACTIVE {
                    0
                } else {
                    let xo = o & 0x00ff_ffff;
                    if xo != 0 {
                        nonzero_offsets.push(xo);
                    }
                    xo
                }
            })
            .collect::<Vec<usize>>();
        nonzero_offsets.sort();
        // Turn offsets into ranges by finding the offset of the next chunk.
        offsets
            .iter()
            .map(|offset| {
                let o = *offset;
                let e = if o != 0 {
                    // NOTE: nonzero_offsets are a subset of offsets, so this should never fail.
                    let p = nonzero_offsets.iter().position(|e| *e == o).unwrap();
                    let next = p + 1;
                    // The last entry has no successor.
                    if next < nonzero_offsets.len() {
                        nonzero_offsets[next]
                    } else {
                        stream_end.min(dir_end)
                    }
                } else {
                    0
                };
                o..e
            })
            .collect::<Vec<Range<usize>>>()
    }
}

impl Removables for Directory {
    /// Removable ranges relative to the start of the directory
    fn removables(self: &Self, retention_list: &Vec<String>) -> Vec<Range<usize>> {
        use log::{debug, info, warn};
        let debug = false;
        let mut removables = vec![];

        let mut unremovable_chunks = vec![];
        let mut all_chunks = vec![];
        let dir_offset = self.offset;

        for m in &self.modules {
            // Get the full directory entry.
            let e = match m {
                Module::Uncompressed(m) => m,
                Module::Lzma(Ok(m)) => m,
                Module::Huffman(Ok((m, h))) => {
                    let n = m.name();
                    let o = m.offset as usize;
                    let s = m.size as usize;

                    // NOTE: The header is always the same, since multiple
                    // Huffman-encoded modules point to the same offset.
                    let cs = h.header.chunk_size;
                    if all_chunks.len() == 0 {
                        info!("Huffman chunk size: {cs}");
                        let stream_end = (h.header.hs0 + h.header.hs1) as usize;
                        all_chunks = self.chunks_as_ranges(&h.chunks, stream_end);
                    }

                    const CHUNK_OFFSET: u32 = 0x10000000;
                    // Each module occupies its own range of chunks.
                    let b = m.mod_base - (h.header.chunk_base + CHUNK_OFFSET);
                    let c = (m.code_size / cs) as usize;
                    let first_chunk = (b / cs) as usize;
                    let last_chunk = first_chunk + c;
                    info!("Huffman compressed {n} @ {o:08x} ({s} bytes)");
                    let a = if retention_list.contains(&n) {
                        for o in &all_chunks[first_chunk..last_chunk + 1] {
                            if o.start != 0 {
                                unremovable_chunks.push(o.clone());
                            }
                        }
                        "retained"
                    } else {
                        "removed"
                    };
                    info!("  Chunks {first_chunk}..{last_chunk} will be {a}");
                    continue;
                }
                Module::Lzma(Err(e)) | Module::Huffman(Err(e)) => {
                    warn!("Compressed module could not be parsed: {e}, skipping");
                    continue;
                }
                Module::Unknown(m) => {
                    let n = m.name();
                    let o = m.offset;
                    let s = m.size;
                    info!("Unknown module {n} @ {o:08x} ({s} bytes)");
                    continue;
                }
            };
            let n = e.name();
            let o = e.offset as usize;
            let s = e.size as usize;

            match &n {
                n if retention_list.contains(n) => {
                    info!("Retain {n} @ {o:08x} ({s} bytes)");
                }
                n => {
                    info!("Remove {n} @ {o:08x} ({s} bytes)");
                    removables.push(o..o + s);
                }
            }
        }

        let mut r = 0;
        for c in &all_chunks {
            let mut remove = true;
            // Filter out chunks that must be kept: those in range of some unremovable chunk
            // TODO: Simplify when Range.is_overlapping is stabiliized (currently nightly),
            // https://doc.rust-lang.org/core/slice/trait.GetDisjointMutIndex.html#tymethod.is_overlapping
            for u in &unremovable_chunks {
                if (u.contains(&c.start)) || (u.contains(&(c.end - 1))) {
                    debug!("OVERLAP: {u:06x?} (partially) contains {c:06x?}");
                    remove = false;
                    break;
                }
            }
            if remove && c.start < c.end {
                // NOTE: Chunks are relative to the start of the ME region (or FPT?).
                // We provide the offset relative to the directory.
                let o = c.start - dir_offset;
                let e = c.end - dir_offset;
                removables.push(o..e);
                r += 1;
            } else if debug && c.start != 0 {
                debug!("KEEP {c:06x?}");
            }
        }

        let a = &all_chunks.len();
        let u = unremovable_chunks.len();
        info!("Total chunks:   {a}");
        info!("   unremovable: {u}");
        info!("   will remove: {r}");

        if debug {
            debug!("== All chunks");
            Self::dump_ranges(&all_chunks);
            debug!("== Unremovable chunks");
            Self::dump_ranges(&unremovable_chunks);
            debug!("== All removables");
            Self::dump_ranges(&removables);
        }

        removables
    }
}
