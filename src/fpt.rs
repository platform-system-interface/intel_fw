use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy_derive::{FromBytes, IntoBytes};

use crate::dir::gen2::Directory as Gen2Directory;
use crate::dir::gen3::CodePartitionDirectory;
use crate::fit::Fit;
use crate::ver::Version;

// see https://github.com/peterbjornx/meimagetool ...intelme/model/fpt/ (Java)
// and https://github.com/linuxboot/fiano/blob/main/pkg/intel/me/structures.go
// and https://github.com/platomav/MEAnalyzer
#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
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

// ...
pub const FPT_MAGIC: &str = "$FPT";

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct FPT {
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

impl Display for FPT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hv = format!("  Header version: {}", self.header_ver);
        let ev = format!("  Entry version:  {}", self.entry_ver);
        let en = format!("  Entries:        {}", self.entries);
        let cs = format!("  Checksum:       {:02x}", self.checksum);
        let v = format!("  FITC version:   {}", self.fitc_ver);
        write!(f, "{hv}\n{ev}\n{cs}\n{v}")
    }
}

#[allow(non_camel_case_types)]
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ME_FPT {
    pub base: usize,
    pub header: FPT,
    pub entries: Vec<FPTEntry>,
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
        // IVBP used in hibernation
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
