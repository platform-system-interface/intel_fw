use core::fmt::{self, Debug, Display};
use core::mem::size_of;
use std::path::PathBuf;

use ansi_term::Colour::{Cyan, Purple, Red};
use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use strum::Display;
use zerocopy::FromBytes;
use zerocopy_derive::FromBytes;

// write out files to a directory (`xdump/`)
const EXTRACT: bool = true;
// verbose output
const VERBOSE: bool = true;
// write out a new file with the pages sorted
const WRITE_SORTED: bool = false;

const PAGE_MAGIC_MASK: u32 = 0xfff0_7800;

const MAGIC: u32 = u32::from_le_bytes(*b"MFS\0");
const PAGE_SIZE: usize = 0x4000;

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy)]
#[repr(C)]
pub struct PageFlags(u8);

impl Debug for PageFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fl = format!("{:04b}", self.0 & 0xf);
        write!(f, "{fl}")
    }
}

impl Display for PageFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fl = self.0 & 0xf;
        // TODO: Those are values seen so far; what do they mean?
        // second bit is always 1 (?)
        // last bit may indicate "dirty"
        let fls = match fl {
            0b0100 => "okay?".to_string(),
            0b0111 => "dirty?".to_string(),
            0b1100 => "live?".to_string(),
            0b1110 => "active?".to_string(),
            _ => format!("{fl:04b}"),
        };
        write!(f, "{fls}")
    }
}

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct PageHeader {
    pub num: u8,
    pub _1: u8, // 78
    pub flags: PageFlags,
    pub _3: u8,         // ff
    pub all_0: u32,     // not always, can be 01. 2b, 29
    pub magic: [u8; 4], // first page only, ffff otherwise
    // freed_flags: [u8; 0x80],
    // block_itab: [u8; 0x40],
    pub smth: u32, // first page only, ffff otherwise
    pub all_f: u32,
}

impl Display for PageHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let num = self.num;
        // typically f7, fc, or fe
        let flags = self.flags;
        if num == 0xff && flags.0 == 0xff {
            return write!(f, "page unused");
        }
        write!(f, "page {num:02}, {flags} ({flags:?})")
    }
}

const PAGE_HEADER_SIZE: usize = size_of::<PageHeader>();

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct Page {
    pub header: PageHeader,
    pub indices: Indices,
    pub live_chunks: Vec<Chunk>,
    pub dead_chunks: Vec<Chunk>,
    /// offset in storage
    pub offset: usize,
}

impl Page {
    pub fn is_active(&self) -> bool {
        let n = self.header.num;
        n != 0x00 && n != 0xff
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Display)]
enum ChunkType {
    Rest,
    Data,
    Big,
    BigRest,
    Unknown,
}

impl From<u8> for ChunkType {
    fn from(value: u8) -> Self {
        match value {
            0b1000 => Self::Rest,
            0b1001 => Self::BigRest,
            0b1010 => Self::Data,
            0b1011 => Self::Big,
            _ => Self::Unknown,
        }
    }
}

impl ChunkType {
    // It looks like the first bits are _always_ `10`. Other bits?
    const fn from_bits(val: u8) -> Self {
        match val & 0xf {
            0b1000 => Self::Rest,
            0b1001 => Self::BigRest,
            0b1010 => Self::Data,
            0b1011 => Self::Big,
            _ => Self::Unknown,
        }
    }

    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u8)]
#[derive(FromBytes, Serialize, Deserialize)]
pub struct ChunkMeta {
    #[bits(4)]
    file_num: u8,
    #[bits(4)]
    chunk_type: ChunkType,
}

impl ChunkMeta {
    pub fn data_offset(&self) -> usize {
        match self.chunk_type() {
            ChunkType::Data | ChunkType::Big => 5,
            ChunkType::Rest | ChunkType::BigRest => 2,
            _ => 0,
        }
    }
}

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ChunkHeader {
    pub meta: ChunkMeta,
    pub size: u8,
}

impl Display for ChunkHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fnum = self.meta.file_num();
        let sz = self.chunk_size();
        let ct = self.meta.chunk_type();

        write!(f, "{fnum:2}: {ct:4} {sz:5}B")
    }
}

const ALIGNMENT: usize = 16;

impl ChunkHeader {
    pub fn chunk_size(&self) -> usize {
        if self.meta.chunk_type() == ChunkType::Big || self.meta.chunk_type() == ChunkType::BigRest
        {
            self.size as usize * 0x100
        } else {
            self.size as usize
        }
    }

    pub fn data_size(&self) -> usize {
        self.chunk_size() - self.meta.data_offset()
    }

    pub fn aligned(&self) -> usize {
        // chunks are 16-byte aligned, filled with 0xff to the end
        let s = self.chunk_size();
        if s.is_multiple_of(ALIGNMENT) {
            s
        } else {
            s.next_multiple_of(16)
        }
    }
}

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Chunk {
    pub header: ChunkHeader,
    pub offset: usize,
}

impl Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let h = self.header;
        let o = self.offset;
        write!(f, "{h} {o:04x}")
    }
}

impl Chunk {
    pub fn is_active(&self) -> bool {
        self.header.meta.file_num() == 0
    }
}

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FilePath {
    pub page: u8,
    pub offset_key: u8,
    pub file_num: u8,
}

impl Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let no = self.file_num;
        let ok = self.offset_key;
        let pg = self.page;

        write!(f, "p{pg:03}-k{ok:02x}-n{no:02x}")
    }
}

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ExtendedFileEntry {
    pub entry: BaseFileEntry,

    pub size: u16,
    pub path: FilePath,
}

#[derive(FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct BaseFileEntry {
    pub state: u8,
    pub flags: u8,

    pub sub: u8, // sub ID or revision
    pub id: u8,

    pub xx: u8,
    pub owner: u8, // not sure

    pub size: u16,
    pub path: FilePath,
}

fn colored_hex(color: ansi_term::Color, value: u8) -> String {
    let f = format!("{:02x}", value);
    if value > 0 {
        color.paint(f).to_string()
    } else {
        f
    }
}

impl BaseFileEntry {
    pub fn idox(&self) -> String {
        let i = self.id;
        let s = self.sub;
        let x = self.xx;
        let o = self.owner;

        format!("{i:02x}-{s:02x}_{x:02x}-{o:02x}")
    }

    pub fn idox_colored(&self) -> String {
        let i = self.id;
        let s = colored_hex(Red, self.sub);
        let x = colored_hex(Purple, self.xx);
        let o = colored_hex(Cyan, self.owner);

        format!("{i:02x}-{s}_{x}-{o}")
    }
}

impl Display for BaseFileEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let st = self.state;
        let fl = self.flags;

        let sz = self.size;
        let p = self.path;

        let i = self.idox_colored();

        // values seen so far; usually ending in C (0b1100) or 8 (0b1000)
        let sx = match st {
            0x1c => "1C",
            0x5c => "5C",
            0xcc => "CC",
            0xdc => "DC", // present?
            0xec => "EC",
            0xfc => "FC",

            0x48 => "48",
            0xc8 => "C8", // overwritten?
            _ => "..",
        };
        let tt = format!("{st:02x} ({sx})");

        // only high 4 bits are set, usually >=4
        let flh = match fl >> 4 {
            ..4 => " ",
            _ => "!",
        };
        let fll = match fl & 0xf {
            0 => " ",
            _ => "?",
        };
        let ll = format!("{fl:02x}{flh}{fll}");

        write!(f, "{i}  {sz:5}  {p}  {tt}  {ll}")
    }
}

#[derive(Clone, Copy, Debug)]
pub enum FileEntry {
    Simple(BaseFileEntry),
    Extended(ExtendedFileEntry),
}

impl FileEntry {
    pub fn id(&self) -> u8 {
        match self {
            FileEntry::Simple(e) => e.id,
            FileEntry::Extended(e) => e.entry.id,
        }
    }

    pub fn flags(&self) -> u8 {
        match self {
            FileEntry::Simple(e) => e.flags,
            FileEntry::Extended(e) => e.entry.flags,
        }
    }

    pub fn idox(&self) -> String {
        match self {
            FileEntry::Simple(e) => e.idox(),
            FileEntry::Extended(e) => e.entry.idox(),
        }
    }

    pub fn path(&self) -> FilePath {
        match self {
            FileEntry::Simple(e) => e.path,
            FileEntry::Extended(e) => e.entry.path,
        }
    }

    pub fn size(&self) -> usize {
        match self {
            FileEntry::Simple(e) => e.size as usize,
            FileEntry::Extended(e) => e.entry.size as usize,
        }
    }
}

impl Display for FileEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileEntry::Simple(e) => write!(f, "{e}"),
            FileEntry::Extended(e) => write!(f, "{}", e.entry),
        }
    }
}

const INDICES_SIZE: usize = 0x40;

#[derive(FromBytes, Clone, Copy, Debug, Serialize, Deserialize)]
#[repr(C, packed)]
pub struct Indices(#[serde(with = "serde_bytes")] [u8; INDICES_SIZE]);

const BASE_FILE_ENTRY_SIZE: usize = size_of::<BaseFileEntry>();

// TODO: evaluate header length, separate from page 0
const PAGE_HEADER_LENGTH: usize = 0x90;
const CHUNK_OFFSET: usize = PAGE_HEADER_LENGTH + INDICES_SIZE;

const BLOCK_SIZE: usize = 0x100;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Display)]
pub enum DataReadError {
    PageNotFound(usize),
    NotInPage,
    UnexpectedChunkSize,
    UnexpectedChunkOffset,
    UnknownChunk,
    ChunkParseError,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Display)]
pub enum DataReadResult {
    Done,
    Need(FilePath),
}

pub fn read_data(
    data: &[u8],
    page: &Page,
    file_path: &FilePath,
    file_size: usize,
    res: &mut Vec<u8>,
) -> Result<DataReadResult, DataReadError> {
    let n = page.header.num;
    let po = page.offset;
    // file offset key -> index translation
    let k = file_path.offset_key as usize;
    let i = page.indices.0[k];
    // base offset
    let bo = i as usize * BLOCK_SIZE;
    // chunk offset
    let mut co = if bo == 0 { CHUNK_OFFSET } else { bo };

    println!("Read data for path {file_path}, start seeking");
    println!("  page {n} @{po:08x}; {k} -> {i} / initial offset: {co:04x}");

    if co >= PAGE_SIZE {
        return Err(DataReadError::UnexpectedChunkOffset);
    }
    let Ok((mut h, _)) = ChunkHeader::read_from_prefix(&data[co..]) else {
        return Err(DataReadError::ChunkParseError);
    };
    if h.meta.chunk_type() == ChunkType::Unknown {
        return Err(DataReadError::UnknownChunk);
    }

    // seek to first chunk belonging to file
    while h.meta.file_num() != file_path.file_num {
        if VERBOSE {
            println!("Skipping             {h} @ {:08x}", po + co);
        } // next offset
        co += h.aligned();
        if co > PAGE_SIZE - ALIGNMENT {
            return Err(DataReadError::NotInPage);
        }
        let Ok((nh, _)) = ChunkHeader::read_from_prefix(&data[co..]) else {
            return Err(DataReadError::ChunkParseError);
        };
        h = nh;
        if h.meta.chunk_type() == ChunkType::Unknown {
            return Err(DataReadError::UnknownChunk);
        }
    }

    loop {
        // A chunk must not cross the page boundary.
        if co + h.chunk_size() > PAGE_SIZE {
            return Err(DataReadError::UnexpectedChunkSize);
        }

        let remaining = file_size - res.len();
        let read_size = h.data_size().min(remaining);
        if VERBOSE {
            println!("Reading {read_size:4} bytes / {h} @ {:08x}", po + co);
        }
        let cdo = h.meta.data_offset();
        let o = co + cdo;
        res.extend_from_slice(&data[o..o + read_size]);

        // We are done if we reached the desired length of the file.
        let remaining = file_size - res.len();
        if remaining == 0 {
            return Ok(DataReadResult::Done);
        }

        // TODO: handle the Rust way
        if cdo == 5 {
            let Ok((p, _)) = FilePath::read_from_prefix(&data[co + 2..]) else {
                return Err(DataReadError::ChunkParseError);
            };
            // continue to read from respective page
            return Ok(DataReadResult::Need(p));
        }

        // Look at the next offset / chunk.
        co += h.aligned();
        if co > PAGE_SIZE - ALIGNMENT {
            return Err(DataReadError::UnexpectedChunkSize);
        }
        if h.meta.chunk_type() == ChunkType::Unknown {
            return Err(DataReadError::UnknownChunk);
        }
        let Ok((nh, _)) = ChunkHeader::read_from_prefix(&data[co..]) else {
            return Err(DataReadError::ChunkParseError);
        };
        h = nh;
    }
}

fn read_file(
    file_path: &FilePath,
    name: &str,
    size: usize,
    pages: &[Page],
    data: &[u8],
) -> Result<Vec<u8>, DataReadError> {
    let mut file_path = file_path.clone();
    let mut d = vec![];
    loop {
        let pnum = file_path.page;
        let Some(p) = pages.iter().find(|p| p.header.num == pnum) else {
            return Err(DataReadError::PageNotFound(pnum as usize));
        };
        let po = p.offset;

        let page_data = &data[po..po + PAGE_SIZE];
        let progress = format!("{}/{size}", d.len());
        match read_data(page_data, p, &file_path, size, &mut d) {
            Ok(DataReadResult::Done) => {
                let all_read = size == d.len();
                let a = if all_read { "OK" } else { "NO" };
                println!("File {name}: read {progress}: {a}");
                return Ok(d);
            }
            Ok(DataReadResult::Need(p)) => {
                println!("File {name}: read {progress}: need {p}");
                file_path = p;
            }
            Err(e) => {
                println!("File {name}: read {progress}: {e}");
                return Err(e);
            }
        }
    }
}

fn save_file(path: &Option<PathBuf>, name: &str, data: &[u8]) {
    if let Some(p) = path {
        let file_name = format!("{name}.bin");
        let out = p.join(file_name);
        println!("saving {out:?}");

        if EXTRACT {
            use std::fs::File;
            use std::io::Write;
            let Ok(mut f) = File::create(out) else {
                return;
            };
            let Ok(_) = f.write_all(data) else {
                return;
            };
        }
    }
}

fn process_file(
    i: usize,
    file: &FileEntry,
    pages: &[Page],
    data: &[u8],
    extract_path: &Option<PathBuf>,
) {
    let file_path = file.path();
    let size = file.size();

    let idox = file.idox();
    let name = format!("{idox}_{file_path}_{i:03}");

    match read_file(&file_path, name.as_str(), size, pages, data) {
        Ok(d) => save_file(extract_path, name.as_str(), &d),
        Err(e) => println!("{e}"),
    }

    if let FileEntry::Extended(e) = file {
        let file_path = e.path;
        let name = format!("{idox}_{file_path}_{i:03}");
        println!("extra file found: {name}");

        match read_file(&file_path, name.as_str(), size, pages, data) {
            Ok(d) => save_file(extract_path, name.as_str(), &d),
            Err(e) => println!("{e}"),
        }
    }
}

pub fn parse(data: &[u8], extract_dir: Option<String>, verbose: bool) -> Result<bool, String> {
    let size = data.len();
    println!("Trying to parse MFS for Gen 2, size: {size:08x}");

    if !size.is_multiple_of(PAGE_SIZE) {
        return Err(format!("Size is not a multiple of page size ({PAGE_SIZE})"));
    }

    let Ok((m, _)) = u32::read_from_prefix(data) else {
        return Err("data read error".into());
    };
    if m & PAGE_MAGIC_MASK != PAGE_MAGIC_MASK {
        return Err(format!(
            "{m:08x} does not match page magic mask {PAGE_MAGIC_MASK:08x}"
        ));
    }

    let extract_dir = extract_dir.map(PathBuf::from);

    if let Some(d) = &extract_dir {
        let Ok(_) = std::fs::create_dir_all(d) else {
            return Err(format!("cannot create directory {d:?}"));
        };
    }

    let mut pages = Vec::<Page>::new();
    let mut files = Vec::<FileEntry>::new();

    // TODO: separate first page already
    for offset in (0..size).step_by(PAGE_SIZE) {
        let slice = &data[offset..offset + PAGE_SIZE];
        let Ok((header, _)) = PageHeader::read_from_prefix(slice) else {
            return Err(format!("Could not read header of page @ {offset:08x}"));
        };

        let mut live_chunks = Vec::<Chunk>::new();
        let mut dead_chunks = Vec::<Chunk>::new();

        let mut pos = CHUNK_OFFSET;

        let n = header.num;
        if n > 0 && n != 0xff {
            if verbose {
                println!("page {n}: read chunks...");
            }
            let mut dead = false;
            loop {
                if pos >= PAGE_SIZE {
                    if verbose {
                        println!("  read all chunks, reached {pos:04x}");
                    }
                    break;
                }
                let o = offset + pos;
                let Ok((ch, _)) = ChunkHeader::read_from_prefix(&data[o..]) else {
                    break;
                };
                let m: u8 = ch.meta.into();
                if m == 0xff || ch.size == 0 {
                    if verbose {
                        println!("  no chunk @ {pos:04x}; size {:04}, meta {m:02x?}", ch.size,);
                    }
                    // break;
                    // NOTE: those may be "dead" chunks
                    pos += 16;
                    dead = true;
                    continue;
                }
                let co = pos - CHUNK_OFFSET;
                let c = Chunk {
                    header: ch,
                    offset: co,
                };
                if verbose {
                    println!("  chunk @ {pos:04x}: {c}");
                }
                if dead {
                    dead_chunks.push(c);
                } else {
                    live_chunks.push(c);
                }
                if verbose && ch.meta.chunk_type() == ChunkType::Big {
                    let x8 = &data[o + 2..o + 10];
                    // NOTE: 3rd byte is always 0x00
                    // Examples:
                    // b0: [0b, 05, 00, 04, 00, 00, 00, 00]
                    // b0: [0b, 12, 00, 00, 00, 00, 00, 00]
                    // b0: [0c, 00, 00, e0, 7a, 33, 95, 52]
                    // b0: [0d, 00, 00, 87, 54, 7d, d8, ec]
                    // b0: [14, 05, 00, 00, ff, ff, ff, ff]
                    // b0: [14, 10, 00, 00, 0c, 00, 00, 04]
                    // b0: [14, 1d, 00, 00, e7, 03, 00, 00]
                    println!("  b0: {x8:02x?}");
                }
                pos += ch.aligned();
            }
        } else if verbose {
            println!("  no chunks to read");
        }
        let Ok((indices, _)) = Indices::read_from_prefix(&slice[PAGE_HEADER_LENGTH..]) else {
            break;
        };

        let p = Page {
            header,
            live_chunks,
            dead_chunks,
            indices,
            offset,
        };

        pages.push(p);
    }

    pages.sort_by_key(|p| p.header.num);

    if WRITE_SORTED {
        use std::io::Write;
        let Ok(mut file) = std::fs::File::create("sorted.bin") else {
            return Err("could not writed file".into());
        };
        for p in &pages {
            let o = p.offset;
            let p = &data[o..o + PAGE_SIZE];
            let Ok(_) = file.write_all(p) else {
                break;
            };
        }
    }

    // first page has MFS magic and the list of files
    if let Some(p0) = pages.first() {
        let m = u32::from_le_bytes(p0.header.magic);
        if m != MAGIC {
            return Err("Gen2 MFS: page 0 does not have expected magic".to_string());
        } else {
            let mut pos = p0.offset + PAGE_HEADER_SIZE;
            loop {
                if pos > p0.offset + PAGE_SIZE {
                    break;
                }
                let Ok((entry, _)) = BaseFileEntry::read_from_prefix(&data[pos..]) else {
                    // TODO
                    break;
                };
                if entry.flags == 0xff && entry.state == 0xff {
                    // no length field found yet, so just break here
                    break;
                }
                let s = entry.size;
                println!("- file @{pos:04x}: {} / {} bytes", entry.path, s);
                let o = pos + BASE_FILE_ENTRY_SIZE;
                // flags == 0xb0, or rather, bit 6 is NOT set
                if entry.flags & (1 << 6) == 0 {
                    let Ok(path) = FilePath::read_from_bytes(&data[o + 2..o + 5]) else {
                        break;
                    };
                    let Ok((size, _)) = u16::read_from_prefix(&data[o..]) else {
                        break;
                    };
                    println!("  extra file found @{o:04x}: {path} / {} bytes", size);
                    let e = ExtendedFileEntry { entry, path, size };
                    files.push(FileEntry::Extended(e));
                    pos += 5;
                } else {
                    files.push(FileEntry::Simple(entry));
                }
                pos += BASE_FILE_ENTRY_SIZE;
            }
        }
    }

    println!();
    println!("== List of files (page 0)");
    if false && !EXTRACT {
        files.sort_by_key(|e| e.id());
    }

    println!(" i    ID S  X  O   size   page/key/fno         ...");
    for (i, s) in files.iter().enumerate() {
        println!("{i:04}: {s}");
        if let FileEntry::Extended(e) = s {
            let sz = e.size;
            println!("              extra: {}  {}", sz, e.path);
        }
    }
    println!();

    let unique = files.iter().map(|i| i.id()).collect::<HashSet<_>>();

    println!("{} entries, {} unique", files.len(), unique.len());
    println!();

    let mut total_live_chunks = 0;
    let mut total_dead_chunks = 0;
    let mut total_active_chunks = 0;
    for p in &pages {
        let h = p.header;
        let lcs = p.live_chunks.len();
        let dcs = p.dead_chunks.len();
        let po = p.offset;
        println!("{h} @ {po:08x}");
        total_live_chunks += lcs;
        total_dead_chunks += dcs;

        if p.is_active() {
            for b in (0..0x40).step_by(0x10) {
                println!("    {:02x?}", &p.indices.0[b..b + 0x10]);
            }
            let fc: Vec<Chunk> = p
                .live_chunks
                .clone()
                .into_iter()
                .filter(|c| c.is_active())
                .collect();
            let acs = fc.len();
            total_active_chunks += acs;

            println!("{lcs} live chunks, {acs} active");
            if lcs > 0 {
                for (i, c) in p.live_chunks.iter().enumerate() {
                    if i > 0 && i % 4 == 0 {
                        println!(" |");
                    }
                    let b = if c.is_active() {
                        // first actual data byte
                        let b = data[po + CHUNK_OFFSET + c.offset + 2];
                        format!("{b:02x}")
                    } else {
                        "  ".to_string()
                    };
                    print!(" | {c} {b}");
                }
                println!(" |");
            }
            println!("{dcs} dead chunks");
            if dcs > 0 {
                for (i, c) in p.dead_chunks.iter().enumerate() {
                    if i > 0 && i % 4 == 0 {
                        println!(" |");
                    }
                    print!(" | {c}");
                }
                println!(" |");
            }
        }
        println!();
    }
    let ps = pages.len();
    println!("{ps} pages");
    println!("{total_live_chunks} live chunks total, {total_active_chunks} active");
    println!("{total_dead_chunks} dead chunks total");

    if true {
        for i in 0..files.len() {
            let Some(file) = files.get(i) else {
                println!("WARNING: file {i} could not be retrieved");
                continue;
            };
            // not clear yet what the flags mean
            let flags = file.flags();
            if (flags >> 4) < 4 {
                println!("WARNING: flags not as expected! {flags:08b}");
                continue;
            }
            process_file(i, file, &pages, data, &extract_dir);
            println!();
        }
    }

    // let i = 92;
    // let file = files.get(i).unwrap();
    // process_file(i, file, &pages, data);

    Ok(true)
}
