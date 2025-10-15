//! Intel Flash Descriptor (IFD)
//!
//! The IFD was extended over time; for reference,
//! see <https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/io-controller-hub-8-datasheet.pdf>
//! and <https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-mobile-p/intel-600-series-chipset-family-on-package-platform-controller-hub-pch-datash/002/>
//! and <https://www.intel.com/content/www/us/en/content-details/710279/intel-600-series-and-intel-700-series-chipset-family-on-package-platform-controller-hub-pch-datasheet-volume-2-of-2.html>
//! and <https://opensecuritytraining.info/IntroBIOS_files/Day2_02_Advanced%20x86%20-%20BIOS%20and%20SMM%20Internals%20-%20Flash%20Descriptor.pdf>
//! and coreboot `util/ifdtool/`.
//!
//! The IFD consists of multiple sections, which got more over generations
//! of processors. The following table is based on the Chip 600 PCH docs.
//! Offsets of specific sections are described via the Descriptor Map,
//! called base addresses, commonly abbreviation as xxBA.
//! NOTE: The base addresses are compact values and really mean bits 4..11
//! of 25-bit values, so we nead to expand them to get the real addresses.
//! See the implementations for the calculations.
//!
//! | Section                      |
//! | ---------------------------- |
//! | Signature + Descriptor Map   |
//! | Components                   |
//! | Regions                      |
//! | Masters                      |
//! | PCH Soft Straps              |
//! | Reserved                     |
//! | Management Engine VSCC Table |
//! | Descriptor Upper Map         |
//! | OEM Section                  |

use std::fmt::Display;

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::FromBytes;
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

// NOTE: This is the LE representation.
const MAGIC: u32 = 0x0ff0_a55a;

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FLMAP0 {
    FCBA: u8,
    #[bits(2)]
    NC: u8,
    #[bits(6)]
    _0: u8,
    FRBA: u8,
    #[bits(3)]
    NR: u8,
    #[bits(5)]
    _1: u8,
}

impl FLMAP0 {
    fn fcba(self) -> usize {
        (self.FCBA() as usize) << 4
    }
    fn nc(self) -> usize {
        self.NC() as usize + 1
    }

    fn frba(self) -> usize {
        (self.FRBA() as usize) << 4
    }
    fn nr(self) -> usize {
        self.NR() as usize + 1
    }
}

impl Display for FLMAP0 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fcba = self.fcba();
        let nc = self.nc();
        let frba = self.frba();
        let nr = self.nr();
        let c = format!("          components:  {nc}, base: 0x{fcba:08x}");
        let r = format!("             regions:  {nr}, base: 0x{frba:08x}");
        write!(f, "{c}\n{r}")
    }
}

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FLMAP1 {
    FMBA: u8,
    #[bits(3)]
    NM: u8,
    #[bits(5)]
    _0: u8,
    FISBA: u8,
    ISL: u8,
}

impl FLMAP1 {
    fn fmba(self) -> usize {
        (self.FMBA() as usize) << 4
    }
    fn nm(self) -> usize {
        self.NM() as usize + 1
    }

    fn fisba(self) -> usize {
        (self.FISBA() as usize) << 4
    }
    fn isl(self) -> usize {
        self.ISL() as usize
    }
}

impl Display for FLMAP1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmba = self.fmba();
        let isl = self.isl();
        let nm = self.nm();
        let fisba = self.FISBA();
        let m = format!("             masters:  {nm}, base: 0x{fmba:08x}");
        let i = format!("   ICH8 strap length: {isl}, base: 0x{fisba:08x}");
        write!(f, "{m}\n{i}")
    }
}

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FLMAP2 {
    FMSBA: u8,
    MSL: u8,
    _0: u16,
}

impl FLMAP2 {
    fn fmsba(self) -> usize {
        (self.FMSBA() as usize) << 4
    }
    fn msl(self) -> usize {
        self.MSL() as usize
    }
}

impl Display for FLMAP2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmsba = self.fmsba();
        let msl = self.msl();
        write!(f, "    MCH strap length:  {msl}, base: 0x{fmsba:08x}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Header {
    magic: u32,
    flmap0: FLMAP0,
    flmap1: FLMAP1,
    flmap2: FLMAP2,
    flmap3: u32, // TODO
}

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FlashRegion {
    #[bits(13)]
    base: u32,
    #[bits(3)]
    _0: u8,
    #[bits(13)]
    limit: u32,
    #[bits(3)]
    _1: u8,
}

impl FlashRegion {
    fn ba(self) -> usize {
        self.base() as usize * 4096
    }
    fn la(self) -> usize {
        self.limit() as usize * 4096 + 4095
    }
}

impl Display for FlashRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.ba();
        let l = self.la();
        write!(f, "{b:08x} - {l:08x}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Regions {
    flreg0: FlashRegion,
    flreg1: FlashRegion,
    flreg2: FlashRegion,
    flreg3: FlashRegion,
    flreg4: FlashRegion,
}

impl Display for Regions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r0 = format!("   Flash descriptor:          {}", self.flreg0);
        let r1 = format!("   BIOS (host) firmware:      {}", self.flreg1);
        let r2 = format!("   ME (coprocessor) firmware: {}", self.flreg2);
        let r3 = format!("   Gigabit ethernet data:     {}", self.flreg3);
        let r4 = format!("   Platform data:             {}", self.flreg4);
        write!(f, "{r0}\n{r1}\n{r2}\n{r3}\n{r4}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct IFD {
    header: Header,
    regions: Regions,
}

impl Display for IFD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Flash ===")?;
        writeln!(f, "== Configuration ==")?;
        writeln!(f, "{}", self.header.flmap0)?;
        writeln!(f, "{}", self.header.flmap1)?;
        writeln!(f, "{}", self.header.flmap2)?;
        writeln!(f, "== Regions ==")?;
        write!(f, "{}", self.regions)
    }
}

const OFFSET: usize = 16;

#[derive(Debug)]
pub enum IfdError {
    NoIfd(String),
}

impl IFD {
    pub fn parse(data: &[u8]) -> Result<Self, IfdError> {
        let (header, _) = Header::read_from_prefix(&data[OFFSET..]).unwrap();

        if header.magic != MAGIC {
            return Err(IfdError::NoIfd(format!(
                "IFD magic not as expected, got: {:08x}, wanted: {MAGIC:08x}",
                header.magic
            )));
        }

        if false {
            println!("{header:#010x?}");
        }
        let (regions, _) = Regions::read_from_prefix(&data[header.flmap0.frba()..]).unwrap();

        Ok(Self { header, regions })
    }
}
