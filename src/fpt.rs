//! For references regarding data structures and logic,
//! see https://github.com/peterbjornx/meimagetool ...intelme/model/fpt/ (Java)
//! and https://github.com/linuxboot/fiano/blob/main/pkg/intel/me/structures.go
//! and https://github.com/platomav/MEAnalyzer
//! and https://github.com/corna/me_cleaner

use core::fmt::{self, Display};
use core::mem::size_of;
use std::convert::Infallible;

use serde::{Deserialize, Serialize};
use zerocopy::{AlignmentError, ConvertError, FromBytes, Ref, SizeError};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::dir::gen2::Directory as Gen2Directory;
use crate::dir::gen3::CodePartitionDirectory;
use crate::fit::Fit;
use crate::ver::Version;

const FPT_MAGIC: &str = "$FPT";

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
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
    pub fitc_ver: Version,
}

const FPT_HEADER_SIZE: usize = size_of::<FPTHeader>();

impl Display for FPTHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hv = format!("  Header version: {}", self.header_ver);
        let ev = format!("  Entry version:  {}", self.entry_ver);
        let en = format!("  Entries:        {}", self.entries);
        let cs = format!("  Checksum:       {:02x}", self.checksum);
        let fv = format!("  FITC version:   {}", self.fitc_ver);
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
#[repr(C)]
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
    pub fn name(self) -> String {
        match std::str::from_utf8(&self.name) {
            Ok(n) => n.trim_end_matches('\0').to_string(),
            Err(_) => format!("{:02x?}", &self.name),
        }
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
    pub header: FPTHeader,
    pub entries: Vec<FPTEntry>,
}

impl<'a> FPT {
    pub fn parse(data: &'a [u8]) -> Option<Result<Self, FptError<'a>>> {
        let m = &data[..4];
        if m.eq(FPT_MAGIC.as_bytes()) {
            let header = match FPTHeader::read_from_prefix(data) {
                Ok((h, _)) => h,
                Err(e) => return Some(Err(FptError::HeaderParseError(e))),
            };
            // NOTE: Skip $FPT (header) itself
            let slice = &data[FPT_HEADER_SIZE..];
            let count = header.entries as usize;
            let entries = match Ref::<_, [FPTEntry]>::from_prefix_with_elems(slice, count) {
                Ok((r, _)) => r,
                Err(e) => return Some(Err(FptError::EntryParseError(e))),
            };

            Some(Ok(Self {
                header,
                entries: entries.to_vec(),
            }))
        } else {
            None
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ME_FPT {
    pub base: usize,
    pub fpt: FPT,
    pub gen3dirs: Vec<CodePartitionDirectory>,
    pub gen2dirs: Vec<Gen2Directory>,
    pub fit: Result<Fit, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PartitionType {
    Code,
    Data,
    None,
}

pub const FTUP: u32 = u32::from_be_bytes(*b"FTUP");
pub const DLMP: u32 = u32::from_be_bytes(*b"DLMP");
pub const FTPR: u32 = u32::from_be_bytes(*b"FTPR");
pub const NFTP: u32 = u32::from_be_bytes(*b"NFTP");
pub const MDMV: u32 = u32::from_be_bytes(*b"MDMV");

pub const MFS: u32 = u32::from_be_bytes(*b"MFS\0");
pub const AFSP: u32 = u32::from_be_bytes(*b"AFSP");
pub const EFFS: u32 = u32::from_be_bytes(*b"EFFS");

// see https://troopers.de/downloads/troopers17/TR17_ME11_Static.pdf
pub fn get_part_info(n: &str) -> (PartitionType, &str) {
    match n {
        "FTPR" => (PartitionType::Code, "Main code partition"),
        "FTUP" => (PartitionType::Code, "[NFTP]+[WCOD]+[LOCL]"),
        "DLMP" => (PartitionType::Code, "IDLM partition"),
        "PSVN" => (PartitionType::Data, "Secure Version Number"),
        // IVBP is used in hibernation, should probably not be removed?!
        "IVBP" => (PartitionType::Data, "IV + Bring Up cache"),
        "MFS" => (PartitionType::Data, "ME Flash File System"),
        "NFTP" => (PartitionType::Code, "Additional code"),
        "ROMB" => (PartitionType::Code, "ROM Bypass"),
        "WCOD" => (PartitionType::Code, "WLAN uCode"),
        "LOCL" => (PartitionType::Code, "AMT Localization"),
        "FLOG" => (PartitionType::Data, "Flash Log"),
        "UTOK" => (PartitionType::Data, "Debug Unlock Token"),
        "ISHC" => (PartitionType::Code, "Integrated Sensors Hub"),
        "AFSP" => (PartitionType::None, "8778 55aa signature like MFS"),
        "FTPM" => (PartitionType::Code, "Firmware TPM (unconfirmed)"),
        "GLUT" => (PartitionType::Data, "Huffman Look-Up Table"),
        "EFFS" => (PartitionType::Data, "EFFS File System"),
        "FOVD" => (PartitionType::Data, "FOVD..."),
        _ => (PartitionType::None, "[> UNKNOWN <]"),
    }
}
