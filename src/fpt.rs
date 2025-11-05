//! Flash Partition Table (FPT)
//!
//! Knowledge herein is solely based on independent analysis efforts.
//! The expansion of the acronym FPT is confirmed by Intel in the appendix
//! on ME Firmware Status Registers in
//! <https://www.intel.com/content/dam/www/public/us/en/documents/technical-specifications/intel-power-node-manager-v3-spec.pdf>.
//! For references regarding data structures and logic,
//! see <https://github.com/peterbjornx/meimagetool> `...intelme/model/fpt/` (Java)
//! and <https://github.com/linuxboot/fiano/blob/main/pkg/intel/me/structures.go>
//! and <https://github.com/platomav/MEAnalyzer>
//! and <https://github.com/corna/me_cleaner>
//! and <https://github.com/mostav02/Remove_IntelME_FPT>.

use core::{
    convert::Infallible,
    fmt::{self, Display},
    mem::size_of,
    num::Wrapping,
};

use serde::{Deserialize, Serialize};
use zerocopy::{AlignmentError, ConvertError, FromBytes, IntoBytes, Ref, SizeError};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::{EMPTY, ver::Version};

pub const FTPR: &str = "FTPR";
pub const FTUP: &str = "FTUP";
pub const DLMP: &str = "DLMP";
pub const MDMV: &str = "MDMV";
pub const PSVN: &str = "PSVN";
pub const IVBP: &str = "IVBP";
pub const MFS: &str = "MFS";
pub const NFTP: &str = "NFTP";
pub const ROMB: &str = "ROMB";
pub const WCOD: &str = "WCOD";
pub const LOCL: &str = "LOCL";
pub const FLOG: &str = "FLOG";
pub const UTOK: &str = "UTOK";
pub const ISHC: &str = "ISHC";
pub const AFSP: &str = "AFSP";
pub const FTPM: &str = "FTPM";
pub const GLUT: &str = "GLUT";
pub const EFFS: &str = "EFFS";
pub const FOVD: &str = "FOVD";

pub const DIR_PARTS: &[&str] = &[
    FTPR, //
    FTUP, //
    DLMP, //
    MDMV, //
    NFTP, //
];

pub const FS_PARTS: &[&str] = &[
    MFS,  //
    AFSP, //
    EFFS, //
];

// Those have been found to be safe to remove.
pub const REMOVABLE_PARTS: &[&str] = &[
    FLOG, //
    FTUP, //
    IVBP, // IVBP is used in hibernation, should probably not be removed?!
    MFS,  //
    NFTP, //
    PSVN, //
    UTOK, //
];

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PartitionType {
    Code,
    Data,
    None,
}

// see https://troopers.de/downloads/troopers17/TR17_ME11_Static.pdf
pub fn get_part_info(n: &str) -> (PartitionType, &str) {
    match n {
        FTPR => (PartitionType::Code, "Main code partition"),
        FTUP => (PartitionType::Code, "[NFTP]+[WCOD]+[LOCL]"),
        DLMP => (PartitionType::Code, "IDLM partition"),
        MDMV => (PartitionType::Code, "Media protection (PAVP, JOM)"),
        PSVN => (PartitionType::Data, "Secure Version Number"),
        IVBP => (PartitionType::Data, "IV + Bring Up cache"),
        MFS => (PartitionType::Data, "ME Flash File System"),
        NFTP => (PartitionType::Code, "Additional code"),
        ROMB => (PartitionType::Code, "ROM Bypass"),
        WCOD => (PartitionType::Code, "WLAN uCode"),
        LOCL => (PartitionType::Code, "AMT Localization"),
        FLOG => (PartitionType::Data, "Flash Log"),
        UTOK => (PartitionType::Data, "Debug Unlock Token"),
        ISHC => (PartitionType::Code, "Integrated Sensors Hub"),
        AFSP => (PartitionType::None, "8778 55aa signature like MFS"),
        FTPM => (PartitionType::Code, "Firmware TPM (unconfirmed)"),
        GLUT => (PartitionType::Data, "Huffman Look-Up Table"),
        EFFS => (PartitionType::Data, "EFFS File System"),
        FOVD => (PartitionType::Data, "FOVD..."),
        _ => (PartitionType::None, "[> UNKNOWN <]"),
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FPTHeader {
    pub signature: [u8; 4],
    pub entries: u32,
    pub header_ver: u8,
    pub entry_ver: u8,
    pub header_len: u8,
    pub checksum: u8,
    pub ticks_to_add: u16,
    pub tokens_to_add: u16,
    pub uma_size_or_reserved: u32,
    pub flash_layout_or_flags: u32,
    // Not Present in ME version 7
    /// Version of Flash Image Tool used to create the image
    /// It is abbreviated FIT(C), though not clear what the C is for.
    /// Note: This is NOT related to the Firmware Interface Table.
    pub fitc_ver: Version,
}

const FPT_HEADER_SIZE: usize = size_of::<FPTHeader>();

impl Display for FPTHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hv = format!("  Header version: {}", self.header_ver);
        let ev = format!("  Entry version:  {}", self.entry_ver);
        let en = self.entries;
        let en = format!("  Entries:        {en}");
        let cs = format!("  Checksum:       {:02x}", self.checksum);
        let fv = self.fitc_ver;
        let fv = format!("  FITC version:   {fv}");
        write!(f, "{hv}\n{ev}\n{en}\n{cs}\n{fv}")
    }
}

#[derive(Debug)]
pub enum FptError<'a> {
    HeaderParseError(SizeError<&'a [u8], FPTHeader>),
    EntryParseError(
        ConvertError<
            AlignmentError<&'a [u8], [FPTEntry]>,
            SizeError<&'a [u8], [FPTEntry]>,
            Infallible,
        >,
    ),
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FPTEntry {
    pub name: [u8; 4],
    pub owner: [u8; 4],
    pub offset: u32,
    pub size: u32,
    pub start_tokens: u32,
    pub max_tokens: u32,
    pub scratch_sectors: u32,
    pub flags: u32,
}

impl FPTEntry {
    pub fn name(&self) -> String {
        match std::str::from_utf8(&self.name) {
            // some names are shorter than 4 bytes and padded with 0x0
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", self.name),
        }
    }

    pub fn offset(&self) -> usize {
        self.offset as usize & 0x003f_ffff
    }

    pub fn size(&self) -> usize {
        self.size as usize
    }
}

const FPT_ENTRY_SIZE: usize = size_of::<FPTEntry>();

impl Display for FPTEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = self.offset as usize;
        let s = self.size as usize;
        let end = o + s;
        let name = self.name();
        let (part_type, full_name) = get_part_info(&name);
        let part_info = format!("{part_type:?}: {full_name}");
        let name_offset_end_size = format!("{name:>4} @ 0x{o:08x}:0x{end:08x} (0x{s:08x})");

        write!(f, "{name_offset_end_size}  {part_info}")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FPT {
    pub pre_header: Vec<u8>,
    pub header: FPTHeader,
    pub entries: Vec<FPTEntry>,
    pub original_size: usize,
}

pub const FPT_SIZE: usize = size_of::<FPT>();

const FPT_MAGIC: &str = "$FPT";
const FPT_MAGIC_BYTES: &[u8] = FPT_MAGIC.as_bytes();

const POSSIBLE_OFFSET: usize = 16;

// The FPT magic is either at the start or at a 16 bytes offset.
fn determine_offset(data: &[u8]) -> Option<usize> {
    let m = &data[..FPT_MAGIC_BYTES.len()];
    if m.eq(FPT_MAGIC_BYTES) {
        return Some(0);
    } else {
        let m = &data[POSSIBLE_OFFSET..POSSIBLE_OFFSET + FPT_MAGIC_BYTES.len()];
        if m.eq(FPT_MAGIC_BYTES) {
            return Some(POSSIBLE_OFFSET);
        } else {
            return None;
        }
    }
}

impl<'a> FPT {
    pub fn parse(data: &'a [u8]) -> Option<Result<Self, FptError<'a>>> {
        let Some(offset) = determine_offset(data) else {
            return None;
        };
        // Save for checksum recalculation
        let pre_header = &data[..offset];
        let d = &data[offset..];
        let header = match FPTHeader::read_from_prefix(d) {
            Ok((h, _)) => h,
            Err(e) => return Some(Err(FptError::HeaderParseError(e))),
        };
        // NOTE: Skip $FPT (header) itself
        let slice = &d[FPT_HEADER_SIZE..];
        let count = header.entries as usize;
        let entries = match Ref::<_, [FPTEntry]>::from_prefix_with_elems(slice, count) {
            Ok((r, _)) => r,
            Err(e) => return Some(Err(FptError::EntryParseError(e))),
        };

        let original_size = pre_header.len() + FPT_HEADER_SIZE + FPT_ENTRY_SIZE * entries.len();

        Some(Ok(Self {
            pre_header: pre_header.to_vec(),
            header,
            entries: entries.to_vec(),
            original_size,
        }))
    }

    // Find an FPT in a given slice, and if the magic is detected, get the
    // parse result and the offset.
    pub fn scan(data: &'a [u8]) -> Option<(Result<Self, FptError<'a>>, usize)> {
        for o in (0..data.len() - FPT_SIZE - POSSIBLE_OFFSET).step_by(0x40) {
            if let Some(fpt) = Self::parse(&data[o..]) {
                return Some((fpt, o));
            }
        }
        None
    }

    /// Two's complement of the sum of the bytes
    pub fn header_checksum(&self) -> u8 {
        let mut c = self.header.clone();
        // Initial checksum field itself must be 0.
        c.checksum = 0;
        let d = [self.pre_header.as_bytes(), c.as_bytes()].concat();
        let sum = d.iter().map(|e| Wrapping(*e as i8)).sum::<Wrapping<i8>>();
        -sum.0 as u8
    }

    /// Remove all entries but FTPR, adjusting header and checksum
    pub fn clear(&mut self) {
        let new_entries = self
            .entries
            .iter()
            .filter(|e| e.name() == FTPR)
            .map(|e| *e)
            .collect::<Vec<FPTEntry>>();
        self.entries = new_entries;
        self.header.entries = self.entries.len() as u32;
        // clear EFFS presence flag
        // TODO: define bitfield, parameterize via API
        self.header.flash_layout_or_flags &= 0xffff_fffe;
        self.header.checksum = self.header_checksum();
    }

    pub fn to_vec(self) -> Vec<u8> {
        let all = [
            self.pre_header.as_bytes(),
            self.header.as_bytes(),
            self.entries.as_bytes(),
        ]
        .concat();
        let mut res = all.to_vec();
        res.resize(self.original_size, EMPTY);
        res
    }
}

#[cfg(test)]
static DATA: &[u8] = include_bytes!("../tests/me11.fpt");

#[cfg(test)]
static FPT_CLEANED: &[u8] = include_bytes!("../tests/me11_cleaned.fpt");

#[test]
fn parse_size_error() {
    let parsed = FPT::parse(&DATA[..70]);
    assert!(parsed.is_some());
    let fpt_res = parsed.unwrap();
    assert!(matches!(fpt_res, Err(FptError::EntryParseError(_))));
}

#[test]
fn parse_okay_fpt() {
    let parsed = FPT::parse(&DATA[16..]);
    assert!(parsed.is_some());
    let fpt_res = parsed.unwrap();
    assert!(fpt_res.is_ok());
    let fpt = fpt_res.unwrap();
    assert_eq!(fpt.header.entries as usize, fpt.entries.len());
}

#[test]
fn parse_okay_fpt_with_offset() {
    let parsed = FPT::parse(&DATA);
    assert!(parsed.is_some());
    let fpt_res = parsed.unwrap();
    assert!(fpt_res.is_ok());
    let fpt = fpt_res.unwrap();
    assert_eq!(fpt.header.entries as usize, fpt.entries.len());
}

#[test]
fn checksum() {
    let parsed = FPT::parse(&DATA);
    let fpt = parsed.unwrap().unwrap();
    assert_eq!(fpt.header_checksum(), fpt.header.checksum);
}

#[test]
fn clear() {
    let mut fpt = FPT::parse(&DATA).unwrap().unwrap();
    fpt.clear();
    let s = fpt.original_size;
    let cleaned = &fpt.to_vec();
    assert_eq!(cleaned, &FPT_CLEANED[..s]);
}
