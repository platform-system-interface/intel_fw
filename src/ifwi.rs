/// Integrated Firmware Image
///
/// For reference, see coreboot `util/cbfstool/ifwitool.c` and
/// <https://github.com/tianocore/edk2-platforms/blob/devel-IntelAtomProcessorE3900/Silicon/BroxtonSoC/BroxtonSiPkg/Include/Library/BpdtLib.h>
/// as referenced in <https://cdrdv2-public.intel.com/671281/uefi-firmware-enabling-guide-for-the-intel-atom-processor-e3900-series.pdf>
use core::fmt::{self, Display};

use serde::{Deserialize, Serialize};
use strum::Display as StrDisplay;
use zerocopy::{FromBytes, IntoBytes, Ref};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::ver::Version;

const BPDT_MAGIC: u32 = 0x0000_55aa;
const BPDT_RECOVERY_MAGIC: u32 = 0x00aa_55aa;

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PreIFWIHeader {
    pub size: u32,
    pub checksum: u32,
}

const PRE_IFWI_HEADER_SIZE: usize = size_of::<PreIFWIHeader>();

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PreIFWIEntry {
    pub offset: u32,
    pub size: u32, // could also be end, not sure yet
}

/// Another struct at the start / before IFWI with BPDT v2
///
/// Contains references to FPT and BPDTs
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PreIFWI {
    pub header: PreIFWIHeader,
    pub entries: Vec<PreIFWIEntry>,
}

// Safety check
const PRE_IFWI_MAX_SIZE: u32 = 0x200;

const PRE_IFWI_OFFSET: usize = 16;

const CRC32: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC);

impl PreIFWI {
    pub fn parse(data: &[u8]) -> Result<Self, String> {
        let (header, rest) = match PreIFWIHeader::read_from_prefix(&data[PRE_IFWI_OFFSET..]) {
            Ok(r) => r,
            Err(e) => {
                return Err(format!("cannot parse Pre-IFWI header: {e:?}"));
            }
        };

        let s = header.size;
        if s > PRE_IFWI_MAX_SIZE {
            return Err(format!(
                "Pre-IFWI should not be this large, expecing < {PRE_IFWI_MAX_SIZE}, got {s}"
            ));
        }

        // Parse the entries themselves.
        let remaining = (header.size as usize) - PRE_IFWI_HEADER_SIZE;
        let entries = match Ref::<_, [PreIFWIEntry]>::from_bytes(&rest[..remaining]) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(format!("cannot parse Pre-IFWI entries: {e}"));
            }
        };
        let entries = entries.to_vec();

        let res = Self { header, entries };

        let cs = res.header.checksum;
        // See if we get the same value.
        let cs_calculated = res.checksum();
        if cs_calculated != cs {
            return Err(format!(
                "Pre-IFWI checksum mismatch: got {cs:08x}, expected {cs_calculated:08x}"
            ));
        }

        Ok(res)
    }

    pub fn checksum(&self) -> u32 {
        let mut c = self.header;
        // Initial checksum field itself must be 0.
        c.checksum = 0;
        let d = [c.as_bytes(), self.entries.as_bytes()].concat();
        CRC32.checksum(&d)
    }
}

/// BPDT header
///
/// Taken from coreboot's ifwitool
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Header {
    magic: u32,
    /// Number of entries
    pub descriptor_count: u16,
    pub bpdt_version: u16,
    /// Unused, should be 0.
    xor_redundant_block: u32,
    pub ifwi_version: u32,
    pub fit_tool_version: Version,
}

impl Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.descriptor_count;
        let bv = self.bpdt_version;
        let iv = self.ifwi_version;
        let fv = self.fit_tool_version;
        write!(f, "{c:2} entries, BPDT v{bv}, IFWI v{iv}, FIT {fv}")
    }
}

/// BPDT entry type
///
/// Many if not most of those are CPDs.
/// It is not clear which version of BPDT has which entry types. The whitepaper
/// does not define them and refers to code instead, which only covers types up
/// to 15, though coreboot's ifwitool also has 16, 17 and 18, stating that it
/// supported BPDT v1 only at the time.
/// The Google "Coral" Chromebook has BPDT v1 and can thus be taken as a sample.
/// The System76 Lemur Pro 10 (Tigerlake) has BDPT v2.
#[derive(Immutable, StrDisplay, Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
#[allow(non_camel_case_types)]
pub enum Etype {
    // Intel's public code covers the first type up to 15
    SMIP = 0,
    RBEP = 1,
    FTPR = 2,
    #[strum(serialize = "Microcode")]
    UCOD = 3,
    #[strum(serialize = "Boot block")]
    IBBP = 4,
    #[strum(serialize = "Secondary BPDT")]
    SBPDT = 5,
    #[strum(serialize = "OEM boot block")]
    OBBP = 6,
    NFTP = 7,
    ISHP = 8,
    DLMP = 9,
    IFPOverride = 10,
    #[strum(serialize = "Debug tokens")]
    DTOK = 11,
    UFSPhy = 12,
    UFS_GPP = 13,
    #[strum(serialize = "PMC firmware")]
    PMCP = 14,
    #[strum(serialize = "IUnit")]
    IUNP = 15,
    // Additional types taken from coreboot
    #[strum(serialize = "NVM Config")]
    NVMC = 16,
    UEP = 17,
    #[strum(serialize = "UFS Rate B Config")]
    UFSB = 18,
    // Additional discoveries from System76 Lemur Pro 10 image, BPDT v2
    #[strum(serialize = "OEM Partition")]
    OEMP = 20,
    IOMP = 23,
    NPHY = 24,
    TBTP = 25,
    // We may not be able to rely on IFWI being immediately followed by the FPT.
    #[strum(serialize = "Partition Table")]
    FPT = 32,
    Unknown(u16),
}

impl From<u16> for Etype {
    fn from(t: u16) -> Self {
        match t {
            0 => Etype::SMIP,
            1 => Etype::RBEP,
            2 => Etype::FTPR,
            3 => Etype::UCOD,
            4 => Etype::IBBP,
            5 => Etype::SBPDT,
            6 => Etype::OBBP,
            7 => Etype::NFTP,
            8 => Etype::ISHP,
            9 => Etype::DLMP,
            10 => Etype::IFPOverride,
            11 => Etype::DTOK,
            12 => Etype::UFSPhy,
            13 => Etype::UFS_GPP,
            14 => Etype::PMCP,
            15 => Etype::IUNP,
            16 => Etype::NVMC,
            17 => Etype::UEP,
            18 => Etype::UFSB,
            20 => Etype::OEMP,
            23 => Etype::IOMP,
            24 => Etype::NPHY,
            25 => Etype::TBTP,
            32 => Etype::FPT,
            u => Etype::Unknown(u),
        }
    }
}

impl Into<u16> for Etype {
    fn into(self) -> u16 {
        match self {
            Etype::Unknown(t) => t,
            t => t.into(),
        }
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Entry {
    pub etype: u16,
    pub flags: u16,
    pub offset: u32,
    pub size: u32,
}

impl Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: figure out how to directly instantiate through zerocopy traits
        let t = match Etype::from(self.etype) {
            Etype::Unknown(n) => format!("Unknown ({n:2})"),
            t => format!("{t}"),
        };
        let o = self.offset;
        let s = self.size;
        let e = o + s;
        let fl = self.flags;
        write!(f, "{t:15} @ {o:08x}..{e:08x}; {fl:08x}")
    }
}

/// Boot Partition Descriptor Table
#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct BPDT {
    pub offset: usize,
    pub header: Header,
    pub entries: Vec<Entry>,
}

impl Display for BPDT {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = self.header;
        let o = self.offset;
        writeln!(f, "{h}  @ {o:08x}")?;
        for e in &self.entries {
            writeln!(f, "- {e}")?;
        }
        write!(f, "")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BpdtError {
    ParseError(String),
    InvalidData(String),
}

impl BPDT {
    pub fn parse(data: &[u8], offset: usize) -> Result<Self, BpdtError> {
        let (header, rest) = match Header::read_from_prefix(data) {
            Ok(r) => r,
            Err(e) => {
                return Err(BpdtError::ParseError(format!(
                    "cannot parse BPDT header: {e:?}"
                )));
            }
        };

        let m = header.magic;
        if m != BPDT_MAGIC {
            return Err(BpdtError::InvalidData(format!(
                "BPDT: wrong magic, got {m:08x}, wanted {BPDT_MAGIC:08x}"
            )));
        }

        // Parse the entries themselves.
        let count = header.descriptor_count as usize;
        let entries = match Ref::<_, [Entry]>::from_prefix_with_elems(rest, count) {
            Ok((entries, _)) => entries,
            Err(e) => {
                return Err(BpdtError::ParseError(format!(
                    "cannot parse BPDT entries: {e}"
                )));
            }
        };
        let entries = entries.to_vec();

        Ok(Self {
            offset,
            header,
            entries,
        })
    }

    pub fn next(&self, data: &[u8]) -> Option<Result<Self, BpdtError>> {
        if let Some(e) = self
            .entries
            .iter()
            .find(|e| Etype::from(e.etype) == Etype::SBPDT)
        {
            let o = e.offset as usize;
            if o == 0 {
                return None;
            }
            let l = data.len();
            if o > l {
                let msg = format!("offset {o:08x}, out of bounds, only got {l:08x}");
                return Some(Err(BpdtError::InvalidData(msg)));
            }
            return Some(Self::parse(&data[o..], o));
        }
        None
    }
}

/*
00003000: aa55 0000 0d00 0200 e79f cc3a 0000 0000  .U.........:....
00003010: 0f00 0000 2900 6e08 0900 0000 0000 0000  ....).n.........
00003020: 0000 0000 0a00 0000 0000 0000 0000 0000  ................
00003030: 0500 0000 0000 0000 0000 0000 0100 0000  ................
00003040: 0010 0000 0080 0100 0c00 0000 0000 0000  ................
00003050: 0000 0000 0d00 0000 0000 0000 0000 0000  ................
00003060: 1400 0000 00e0 1700 0010 0000 0200 0000  ................
00003070: 0090 0500 0050 1200 0e00 0000 0090 0100  .....P..........
00003080: 0000 0400 1700 0000 00f0 1700 0020 0100  ............. ..
00003090: 1800 0000 0010 1900 0000 0100 1900 0000  ................
000030a0: 0010 1a00 00c0 0400 2000 0000 00d0 1e00  ........ .......
000030b0: 0010 0000 ffff ffff ffff ffff ffff ffff  ................
000030c0: ffff ffff ffff ffff ffff ffff ffff ffff  ................
--
00278000: aa55 0000 0800 0200 a0e9 1932 0000 0000  .U.........2....
00278010: 0f00 0000 2900 6e08 0900 0000 0000 0000  ....).n.........
00278020: 0000 0000 0a00 0000 0000 0000 0000 0000  ................
00278030: 0500 0000 0000 0000 0000 0000 0100 0000  ................
00278040: 0000 0000 0000 0000 0c00 0000 0000 0000  ................
00278050: 0000 0000 0d00 0000 0000 0000 0000 0000  ................
00278060: 1400 0000 0000 0000 0000 0000 0700 0000  ................
00278070: 0010 0000 00d0 1e00 ffff ffff ffff ffff  ................
00278080: ffff ffff ffff ffff ffff ffff ffff ffff  ................
00278090: ffff ffff ffff ffff ffff ffff ffff ffff  ................
002780a0: ffff ffff ffff ffff ffff ffff ffff ffff  ................
002780b0: ffff ffff ffff ffff ffff ffff ffff ffff  ................
002780c0: ffff ffff ffff ffff ffff ffff ffff ffff  ................
*/
