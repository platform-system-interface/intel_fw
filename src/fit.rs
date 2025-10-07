use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Ref};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

// firmware-interface-table-bios-specification-r1p2p1.pdf
const FIT_MAGIC: &str = "_FIT_   ";

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FitHeader {
    pub magic: [u8; 8],
    pub entries: u32,
    pub version: u16,
    pub checksum_valid_and_type: u8,
    pub checksum: u8,
}

impl Display for FitHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Header counts as entry, but we want the actual number of entries
        let e = self.entries - 1;
        write!(f, "{e} entries")
    }
}

#[derive(IntoBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u8)]
pub enum EntryType {
    Header = 0x00,
    MicrocodeUpdate = 0x01,
    StartupACM = 0x02,
    DiagnosticACM = 0x03,
    BIOSStartupModule = 0x07,
    TPMPolicyRecord = 0x08,
    BIOSPolicyRecord = 0x09,
    TXTPolicyRecord = 0x0A,
    KeyManifestRecord = 0x0B,
    BootPolicyManifest = 0x0C,
    CSESecureBoot = 0x10,
    FeaturePolicyDeliveryRecord = 0x2D,
    IntelReserved = 0x2E,
    JMPDebugPolicy = 0x2F,
    UnusedEntry = 0x7F,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FitEntry {
    pub addr: u64,
    pub size: [u8; 3],
    pub _11: u8,
    pub version: u16,
    pub checksum_valid_and_type: u8,
    pub checksum: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct Fit {
    pub header: FitHeader,
    pub entries: Vec<FitEntry>,
    pub mapping: usize,
    pub offset: usize,
}

const FIT_HEADER_SIZE: usize = core::mem::size_of::<FitHeader>();

// FIXME: This duplication is very tedious and prone to error.
// It is too easy to forget to add something here that was added to the enum.
impl TryFrom<u8> for EntryType {
    type Error = &'static str;

    fn try_from(v: u8) -> Result<EntryType, Self::Error> {
        match v {
            0x00 => Ok(EntryType::Header),
            0x01 => Ok(EntryType::MicrocodeUpdate),
            0x02 => Ok(EntryType::StartupACM),
            0x03 => Ok(EntryType::DiagnosticACM),
            0x04..=0x06 => Err("Intel Reserved"),
            0x07 => Ok(EntryType::BIOSStartupModule),
            0x08 => Ok(EntryType::TPMPolicyRecord),
            0x09 => Ok(EntryType::BIOSPolicyRecord),
            0x0A => Ok(EntryType::TXTPolicyRecord),
            0x0B => Ok(EntryType::KeyManifestRecord),
            0x0C => Ok(EntryType::BootPolicyManifest),
            0x0D..=0x0F => Err("Intel Reserved"),
            0x10 => Ok(EntryType::CSESecureBoot),
            0x11..=0x2C => Err("Intel Reserved"),
            0x2D => Ok(EntryType::FeaturePolicyDeliveryRecord),
            0x2E => Err("Intel Reserved"),
            0x2F => Ok(EntryType::JMPDebugPolicy),
            0x30..=0x70 => Err("Reserved for Platform Manufacturer Use"),
            0x71..=0x7E => Err("Intel Reserved"),
            0x7F => Err("Unused Entry"),
            _ => Err("unknown FIT entry type"),
        }
    }
}

const MAP_8M: usize = 0x007f_ffff;
const MAP_16M: usize = 0x00ff_ffff;

const SIZE_8M: usize = 8 * 1024 * 1024;
const SIZE_16M: usize = 16 * 1024 * 1024;

// The flash is mapped so that it ends at 0xffff_ffff, so we need to
// map it to resolve pointers.
fn get_mapping(size: usize) -> usize {
    match size {
        SIZE_8M => MAP_8M,
        SIZE_16M => MAP_16M,
        _ => MAP_16M,
    }
}

impl Fit {
    pub fn new(data: &[u8]) -> Result<Self, String> {
        let fitp_pos = data.len() - 0x40;
        let fitp = &data[fitp_pos..fitp_pos + 4];
        let mapping = get_mapping(data.len());
        let Ok((fp, _)) = u32::read_from_prefix(fitp) else {
            return Err(format!("Cannot read FIT pointer @ {fitp_pos:08x}"));
        };
        if fp == 0xffff_ffff {
            let err = format!("Not a FIT: {fp:08x}");
            return Err(err);
        }
        let offset = mapping & fp as usize;
        // NOTE: FIT is usually aligned. The spec does not mandate it though.
        if offset % 0x10 != 0 {
            let err = format!("Not a FIT pointer: {offset:08x}");
            return Err(err);
        }

        let Ok((header, _)) = FitHeader::read_from_prefix(&data[offset..]) else {
            return Err(format!("No FIT header @ {offset:08x}"));
        };
        // NOTE: The header counts as a first entry.
        let count = (header.entries - 1) as usize;
        let pos = offset + FIT_HEADER_SIZE;
        let slice = &data[pos..];
        let Ok((r, _)) = Ref::<_, [FitEntry]>::from_prefix_with_elems(slice, count) else {
            return Err(format!("cannot parse FIT entries @ {:08x}", pos));
        };
        let entries = r.to_vec();
        let fit = Fit {
            header,
            entries,
            mapping,
            offset,
        };
        Ok(fit)
    }
}

impl FitEntry {
    pub fn get_type(&self) -> Result<EntryType, &str> {
        let t = self.checksum_valid_and_type & 0xef;
        EntryType::try_from(t)
    }

    pub fn get_type_name(&self) -> &str {
        let t = self.get_type();

        match t {
            Ok(EntryType::Header) => "FIT Header",
            Ok(EntryType::MicrocodeUpdate) => "Microcode Update",
            Ok(EntryType::StartupACM) => "Startup AC Module",
            Ok(EntryType::DiagnosticACM) => "Diagnostic AC Module",
            Ok(EntryType::BIOSStartupModule) => "BIOS Startup Module",
            Ok(EntryType::TPMPolicyRecord) => "TPM Policy Record",
            Ok(EntryType::BIOSPolicyRecord) => "BIOS Policy Record",
            Ok(EntryType::TXTPolicyRecord) => "TXT Policy Record",
            Ok(EntryType::KeyManifestRecord) => "Key Manifest Record",
            Ok(EntryType::BootPolicyManifest) => "Boot Policy Manifest",
            Ok(EntryType::CSESecureBoot) => "CSE Secure Boot",
            Ok(EntryType::FeaturePolicyDeliveryRecord) => "Feature Policy Delivery Record",
            Ok(EntryType::IntelReserved) => "Intel Reserved",
            Ok(EntryType::JMPDebugPolicy) => "JMP $ Debug Policy",
            Ok(EntryType::UnusedEntry) => "Unused Entry (skip)",
            Err(e) => e,
        }
    }

    pub fn is_checksum_valid(&self) -> bool {
        self.checksum_valid_and_type & 0x80 > 0
    }
}

impl Display for FitEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = self.get_type_name();
        let addr = self.addr;
        let s = self.size;
        let size = u32::from_le_bytes([s[2], s[1], s[0], 0]);
        let ver = self.version;
        let cs = if self.is_checksum_valid() {
            format!("checksum {:02x}", self.checksum)
        } else {
            "no checksum".to_string()
        };
        write!(f, "{t:40} {size:08x} @ {addr:08x} version {ver:04x} {cs}")
    }
}
