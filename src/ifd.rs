//! Intel Flash Descriptor (IFD)
//!
//! The IFD was extended over time; for reference,
//! see <https://www.intel.com/content/dam/www/public/us/en/documents/datasheets/io-controller-hub-8-datasheet.pdf>
//! and <https://www.intel.com/content/dam/doc/datasheet/7300-chipset-memory-controller-hub-datasheet.pdf>
//! and <https://www.intel.com/content/www/us/en/content-details/332690/intel-100-series-chipset-family-platform-controller-hub-pch-datasheet-volume-1.html>
//! and <https://www.intel.com/content/www/us/en/content-details/332691/intel-100-series-chipset-family-platform-controller-hub-pch-datasheet-volume-2.html>
//! and <https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-mobile-p/intel-600-series-chipset-family-on-package-platform-controller-hub-pch-datash/002/>
//! and <https://www.intel.com/content/www/us/en/content-details/710279/intel-600-series-and-intel-700-series-chipset-family-on-package-platform-controller-hub-pch-datasheet-volume-2-of-2.html>
//! and <https://opensecuritytraining.info/IntroBIOS_files/Day2_02_Advanced%20x86%20-%20BIOS%20and%20SMM%20Internals%20-%20Flash%20Descriptor.pdf>
//! and coreboot `util/ifdtool/`.
//!
//! The IFD consists of multiple sections and fields, which got more over generations
//! of processors. While the ICH8 datasheet still detailed the sections and fields,
//! some semantics changed over time, without public documentation from Intel.
//! Unfortunately, there is no other single source of truth documenting which
//! processors would require which exact fields, either. One major change came with
//! Skylake, as per coreboot commit `1f7fd720c81755144423f2d4062c39cc651adc0a`.
//! The following table is based on the 600 series chipset PCH datasheet.
//! The rough sections have generally stayed the same, but not the fields.
//! Offsets of specific sections are described via the Descriptor Map,
//! called base addresses, commonly abbreviation as xxBA.
//! NOTE: The base addresses are compact values and really mean bits 4..11
//! of 25-bit values, so we nead to expand them to get the real addresses.
//! See the implementations for the calculations.
//!
//! | Section                      | Meaning                                |
//! | ---------------------------- | -------------------------------------- |
//! | Signature + Descriptor Map   | Offsets of other sections              |
//! | Components                   | Flash parts and their parameters       |
//! | Regions                      | Flash partitions as offsets            |
//! | Masters                      | Access control for regions             |
//! | PCH Soft Straps              | Platform specific control bits         |
//! | Reserved                     |                                        |
//! | Management Engine VSCC Table | Vendor-specific component capabilities |
//! | Descriptor Upper Map         |                                        |
//! | OEM Section                  |                                        |
//!
//! For a list of acronyms, see the Serial Peripheral Interface (SPI) section
//! in the 400 or 600 series chipset PCH datasheet volume 1.
//! <https://edc.intel.com/content/www/us/en/design/ipla/software-development-platforms/client/platforms/alder-lake-mobile-p/intel-600-series-chipset-family-on-package-platform-controller-hub-pch-datash/serial-peripheral-interface-spi/>

// We retain the all-uppercase acronyms in the struct definitions.
// Lowercase helpers are provided through implementations.
#![allow(non_snake_case)]

use std::fmt::{Debug, Display};

use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, IntoBytes};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::{EMPTY, me::Generation, ver::Version};

// NOTE: This is the LE representation.
const MAGIC: u32 = 0x0ff0_a55a;
// This is based on examples, excluding the VSCC table, upper descriptor map and
// OEM section.
const SIZE: usize = 0x800;

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
        let c = format!("        components:  {nc}, base: 0x{fcba:08x}");
        let r = format!("           regions:  {nr}, base: 0x{frba:08x}");
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
        let nm = self.nm();
        // NOTE: On later platforms, FISBA was changed into FPSBA (PCH Strap).
        let fisba = self.fisba();
        let isl = self.isl();
        let m = format!("           masters:  {nm}, base: 0x{fmba:08x}");
        let s = format!("   ICH8/PCH straps: {isl:2}, base: 0x{fisba:08x}");
        write!(f, "{m}\n{s}")
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

// Only for 100 up to 900 series chipset PCHs, per coreboot util/ifdtool
impl Display for FLMAP2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fmsba = self.fmsba();
        // coreboot calls this PSL
        let msl = self.msl();
        write!(f, "        MCH straps: {msl:2}, base: 0x{fmsba:08x}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Header {
    magic: u32,
    flmap0: FLMAP0,
    flmap1: FLMAP1,
    flmap2: FLMAP2, // 100x series
    flmap3: u32,    // TODO: 500, 600, 800 and 900 series
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
enum Density {
    K512,
    M1,
    M2,
    M4,
    M8,
    M16,
    _Undefined,
    _Reserved,
}

impl Density {
    const fn from_bits(val: u8) -> Self {
        match val {
            0b000 => Self::K512,
            0b001 => Self::M1,
            0b010 => Self::M2,
            0b011 => Self::M4,
            0b100 => Self::M8,
            0b101 => Self::M16,
            0b111 => Self::_Reserved,
            _ => Self::_Undefined,
        }
    }

    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
enum Frequency {
    M20,
    M33,
    M48,
    M50_30,
    M17,
    _Undefined,
    _Reserved,
}

impl Frequency {
    const fn from_bits(val: u8) -> Self {
        match val {
            0b000 => Self::M20,
            0b001 => Self::M33,
            0b010 => Self::M48,
            0b011 => Self::_Undefined,
            0b100 => Self::M50_30,
            0b101 => Self::_Undefined,
            0b110 => Self::M17,
            _ => Self::_Reserved,
        }
    }

    const fn into_bits(self) -> u8 {
        self as u8
    }
}

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FlashComponentConfig {
    #[bits(3)]
    comp1_density: Density,
    #[bits(3)]
    comp2_density: Density,
    #[bits(2)]
    _r: u8,

    #[bits(8)]
    _r: u8,

    #[bits(1)]
    _r: u8,
    #[bits(3)]
    read_clock_frequency: Frequency,
    fast_read_support: bool,
    #[bits(3)]
    fast_read_clock_frequency: Frequency,

    #[bits(3)]
    write_erase_clock_frequency: Frequency,
    #[bits(3)]
    read_id_status_clock_frequency: Frequency,
    #[bits(2)]
    _r: u8,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct FlashInvalidInstructions {
    inst1: u8,
    inst2: u8,
    inst3: u8,
    inst4: u8,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Components {
    FLCOMP: FlashComponentConfig,
    FLILL0: FlashInvalidInstructions,
    FLILL1: FlashInvalidInstructions,
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

    pub fn range(self) -> (usize, usize) {
        (self.ba(), self.la() + 1)
    }
}

impl Display for FlashRegion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.ba();
        let l = self.la();
        let u = if b > l { " (unused)" } else { "" };
        write!(f, "{b:08x} - {l:08x}{u}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Regions {
    pub flreg0: FlashRegion,
    pub flreg1: FlashRegion,
    pub flreg2: FlashRegion,
    pub flreg3: FlashRegion,
    pub flreg4: FlashRegion,
    pub flreg5: FlashRegion,
    pub flreg6: FlashRegion,
    pub flreg7: FlashRegion,
    pub flreg8: FlashRegion,
    pub flreg9: FlashRegion,
}

// NOTE: Regions have changed over processors generations.
impl Display for Regions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let r0 = format!("   Flash descriptor (IFD):   {}", self.flreg0);
        let r1 = format!("   BIOS (host) firmware:     {}", self.flreg1);
        let r2 = format!("   (CS)ME firmware:          {}", self.flreg2);
        let r3 = format!("   Gigabit ethernet data:    {}", self.flreg3);
        let r4 = format!("   Platform data:            {}", self.flreg4);
        let r5 = format!("   Embedded controller (EC): {}", self.flreg5);
        write!(f, "{r0}\n{r1}\n{r2}\n{r3}\n{r4}\n{r5}")
    }
}

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FlashMasterV1 {
    _r: u16,
    // read access
    read_fd: bool,
    read_bios: bool,
    read_me: bool,
    read_gbe: bool,
    read_pd: bool,
    #[bits(3)]
    _r: u8,
    // write access
    write_fd: bool,
    write_bios: bool,
    write_me: bool,
    write_gbe: bool,
    write_pd: bool,
    #[bits(3)]
    _r: u8,
}

fn cap_to_str(cap: bool) -> &'static str {
    if cap { "enabled" } else { "disabled" }
}

impl Display for FlashMasterV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = cap_to_str(self.write_pd());
        writeln!(f, "  Platform Data Region Write Access: {c}")?;
        let c = cap_to_str(self.write_gbe());
        writeln!(f, "  GbE Region Write Access:           {c}")?;
        let c = cap_to_str(self.write_me());
        writeln!(f, "  Intel ME Region Write Access:      {c}")?;
        let c = cap_to_str(self.write_bios());
        writeln!(f, "  Host CPU/BIOS Region Write Access: {c}")?;
        let c = cap_to_str(self.write_fd());
        writeln!(f, "  Flash Descriptor Write Access:     {c}")?;

        let c = cap_to_str(self.read_pd());
        writeln!(f, "  Platform Data Region Read Access:  {c}")?;
        let c = cap_to_str(self.read_gbe());
        writeln!(f, "  GbE Region Read Access:            {c}")?;
        let c = cap_to_str(self.read_me());
        writeln!(f, "  Intel ME Region Read Access:       {c}")?;
        let c = cap_to_str(self.read_bios());
        writeln!(f, "  Host CPU/BIOS Region Read Access:  {c}")?;
        let c = cap_to_str(self.read_fd());
        writeln!(f, "  Flash Descriptor Read Access:      {c}")?;
        write!(f, "")
    }
}

#[bitfield(u32)]
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize)]
pub struct FlashMasterV2 {
    // 0..7
    read_gbe10_1: bool,
    #[bits(3)]
    _r: u8,
    write_gbe10_1: bool,
    #[bits(3)]
    _r: u8,
    // 8..15
    read_fd: bool,
    read_bios: bool,
    read_me: bool,
    read_gbe: bool,
    read_pd: bool,
    #[bits(3)]
    _r: u8,
    // 16..23
    read_ec: bool,
    #[bits(2)]
    _r: u8,
    read_gbe10_0: bool,
    write_fd: bool,
    write_bios: bool,
    write_me: bool,
    write_gbe: bool,
    // 24..31
    write_pd: bool,
    #[bits(3)]
    _r: u8,
    write_ec: bool,
    #[bits(2)]
    _r: u8,
    write_gbe10_0: bool,
}

impl Display for FlashMasterV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let c = cap_to_str(self.write_ec());
        writeln!(f, "  EC Region Write Access:            {c}")?;
        let c = cap_to_str(self.write_pd());
        writeln!(f, "  Platform Data Region Write Access: {c}")?;
        let c = cap_to_str(self.write_gbe());
        writeln!(f, "  GbE Region Write Access:           {c}")?;
        let c = cap_to_str(self.write_me());
        writeln!(f, "  Intel ME Region Write Access:      {c}")?;
        let c = cap_to_str(self.write_bios());
        writeln!(f, "  Host CPU/BIOS Region Write Access: {c}")?;
        let c = cap_to_str(self.write_fd());
        writeln!(f, "  Flash Descriptor Write Access:     {c}")?;

        let c = cap_to_str(self.read_ec());
        writeln!(f, "  EC Region Read Access:             {c}")?;
        let c = cap_to_str(self.read_pd());
        writeln!(f, "  Platform Data Region Read Access:  {c}")?;
        let c = cap_to_str(self.read_gbe());
        writeln!(f, "  GbE Region Read Access:            {c}")?;
        let c = cap_to_str(self.read_me());
        writeln!(f, "  Intel ME Region Read Access:       {c}")?;
        let c = cap_to_str(self.read_bios());
        writeln!(f, "  Host CPU/BIOS Region Read Access:  {c}")?;
        let c = cap_to_str(self.read_fd());
        writeln!(f, "  Flash Descriptor Read Access:      {c}")?;
        write!(f, "")
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[repr(C)]
pub struct IFD {
    pub header: Header,
    pub components: Components,
    pub regions: Regions,
    pub masters: Vec<u32>,
    pub pch_straps: Vec<u32>,
    pub mch_straps: Vec<u32>,
}

impl Display for IFD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "== Configuration ==")?;
        writeln!(f, "{}", self.header.flmap0)?;
        writeln!(f, "{}", self.header.flmap1)?;
        writeln!(f, "{}", self.header.flmap2)?;
        writeln!(f, "== Components ==")?;
        writeln!(f, "{:#02x?}", self.components)?;
        writeln!(f, "== Regions ==")?;
        write!(f, "{}", self.regions)
    }
}

impl Debug for IFD {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{self}")?;
        writeln!(f, "== ICH/PCH Straps ==")?;
        for (i, s) in self.pch_straps.iter().enumerate() {
            writeln!(f, "  {i:2}: {s:08x}")?;
        }
        writeln!(f, "== MCH Straps ==")?;
        for (i, s) in self.mch_straps.iter().enumerate() {
            writeln!(f, "  {i:2}: {s:08x}")?;
        }
        write!(f, "")
    }
}

/// Extract a bit from a given byte, as bool.
fn extract_bit(straps: &Vec<u32>, byte: usize, bit: u32) -> bool {
    straps[byte] >> bit & 1 == 1
}

/// Set or clear a bit in a given strap, identified by its number.
fn set_bit(straps: &mut Vec<u32>, byte: usize, bit: u32, s: bool) {
    let b = straps[byte];
    straps[byte] = b & !(1 << bit) | ((s as u32) << bit);
}

// TODO: The straps changed over the generations of processors.
// Specifically the HAP bit on Skylake and later has moved, so we should not
// blindly assume it.
impl IFD {
    // TODO: What is CSE in debug mode?
    // https://review.coreboot.org/c/coreboot/+/88307/5

    /// Direct Connect Interface
    // from <https://review.coreboot.org/c/coreboot/+/82272>
    // <https://edc.intel.com/content/www/us/en/design/products-and-solutions/processors-and-chipsets/700-series-chipset-family-platform-controller-hub-datasheet-volume-1-of/004/intel-direct-connect-interface-dci/>
    pub fn dci(&self) -> bool {
        extract_bit(&self.pch_straps, 0, 17)
    }
    /// Direct Connect Interface
    pub fn set_dci(&mut self, s: bool) {
        set_bit(&mut self.pch_straps, 0, 17, s)
    }

    // TODO: there is a _different_ soft-disable feature
    // https://review.coreboot.org/c/coreboot/+/78911/comments/61e2b541_b4faa3e4
    // TODO: make ME version a parameter to infer platform
    /// High-Assurance Platform (ME soft-disable), ME Gen 3
    pub fn hap(&self) -> bool {
        extract_bit(&self.pch_straps, 0, 16)
    }
    /// High-Assurance Platform (ME soft-disable), ME Gen 3
    pub fn set_hap(&mut self, s: bool) {
        set_bit(&mut self.pch_straps, 0, 16, s)
    }

    /// I/O Controller Hub, ME Gen 1
    pub fn ich_me_disabled(&self) -> bool {
        extract_bit(&self.pch_straps, 0, 0)
    }
    /// I/O Controller Hub, ME Gen 1
    pub fn set_ich_me_disabled(&mut self, s: bool) {
        set_bit(&mut self.pch_straps, 0, 0, s)
    }

    /// Memory Controller Hub, ME Gen 1
    pub fn mch_me_disabled(&self) -> bool {
        extract_bit(&self.mch_straps, 0, 0)
    }
    /// Memory Controller Hub, ME Gen 1
    pub fn set_mch_me_disabled(&mut self, s: bool) {
        set_bit(&mut self.mch_straps, 0, 0, s)
    }

    /// Memory Controller Hub (alternative), ME Gen 1
    pub fn mch_alt_me_disabled(&self) -> bool {
        extract_bit(&self.mch_straps, 0, 7)
    }
    /// Memory Controller Hub (alternative), ME Gen 1
    pub fn set_mch_alt_me_disabled(&mut self, s: bool) {
        set_bit(&mut self.mch_straps, 0, 7, s)
    }

    /// Disable ME (alternative), ME Gen 2
    pub fn alt_me_disabled(&self) -> bool {
        extract_bit(&self.pch_straps, 10, 7)
    }
    /// Disable ME (alternative), ME Gen 2
    pub fn set_alt_me_disabled(&mut self, s: bool) {
        set_bit(&mut self.pch_straps, 10, 7, s)
    }

    /// Disable ME for ME generation 3.
    /// Requires passing the ME version in order to infer the platform.
    pub fn disable_me_gen3(&mut self, me_ver: &Option<Version>) -> Result<(), String> {
        if let Some(ver) = me_ver {
            match ver.major {
                11 => self.set_hap(true),
                _ => return Err("ME > 11 not yet supported".into()),
            }
        }
        Ok(())
    }

    /// Disable the ME using any mechanism given the ME generation and version.
    pub fn disable_me(
        &mut self,
        me_gen: &Generation,
        me_ver: &Option<Version>,
    ) -> Result<(), String> {
        match me_gen {
            Generation::Gen1 => {
                self.set_ich_me_disabled(true);
                self.set_mch_me_disabled(true);
                // NOTE: coreboot ifdtool would also set the AltMeDisable bit here.
                // TODO: self.set_alt_me_disabled(true);
                Ok(())
            }
            Generation::Gen2 => {
                self.set_alt_me_disabled(true);
                Ok(())
            }
            Generation::Gen3 => self.disable_me_gen3(me_ver),
            _ => Err("Unknown ME generation/version".into()),
        }
    }
}

impl IFD {
    pub fn to_vec(self) -> Vec<u8> {
        let mut res = vec![EMPTY; SIZE];
        let components_offset = self.header.flmap0.fcba();
        let regions_offset = self.header.flmap0.frba();
        let masters_offset = self.header.flmap1.fmba();
        let pch_straps_offset = self.header.flmap1.fisba();
        let mch_straps_offset = self.header.flmap2.fmsba();

        for (o, b) in self.header.as_bytes().iter().enumerate() {
            res[OFFSET + o] = *b;
        }

        for (o, b) in self.components.as_bytes().iter().enumerate() {
            res[components_offset + o] = *b;
        }
        for (o, b) in self.regions.as_bytes().iter().enumerate() {
            res[regions_offset + o] = *b;
        }
        for (o, b) in self.masters.as_bytes().iter().enumerate() {
            res[masters_offset + o] = *b;
        }

        for (o, b) in self.pch_straps.as_bytes().iter().enumerate() {
            res[pch_straps_offset + o] = *b;
        }
        for (o, b) in self.mch_straps.as_bytes().iter().enumerate() {
            res[mch_straps_offset + o] = *b;
        }

        res
    }
}

const OFFSET: usize = 16;

// NOTE: We cannot use NM here (number of "masters").
// It is not what it suggests, or found to be not matching the actual
// count on real firmware images. What is the real number?
// Do we have to infer it from non-all-ff u32's up to the regions?
// Or should we adjust it after looking at the regions?
// Their count is not clear either.
const REGION_COUNT: usize = 8;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum IfdError {
    NoIfd(String),
}

fn u8_slice_to_u32(slice: &[u8]) -> Vec<u32> {
    slice
        .chunks(4)
        .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
        .collect()
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

        let components_offset = header.flmap0.fcba();
        let regions_offset = header.flmap0.frba();
        let masters_offset = header.flmap1.fmba();
        let pch_straps_offset = header.flmap1.fisba();
        let mch_straps_offset = header.flmap2.fmsba();

        let (components, _) = Components::read_from_prefix(&data[components_offset..]).unwrap();

        let (regions, _) = Regions::read_from_prefix(&data[regions_offset..]).unwrap();

        let slice = &data[masters_offset..masters_offset + REGION_COUNT * 4];
        let masters = u8_slice_to_u32(slice);

        let slice = &data[pch_straps_offset..pch_straps_offset + header.flmap1.isl() * 4];
        let pch_straps = u8_slice_to_u32(slice);

        let slice = &data[mch_straps_offset..mch_straps_offset + header.flmap2.msl() * 4];
        let mch_straps = u8_slice_to_u32(slice);

        Ok(Self {
            header,
            components,
            regions,
            masters,
            mch_straps,
            pch_straps,
        })
    }
}

#[cfg(test)]
static IFD_DATA_GEN2: &[u8] = include_bytes!("../tests/me8.ifd");

#[cfg(test)]
static IFD_DATA_GEN3: &[u8] = include_bytes!("../tests/me11.ifd");

#[test]
/// We should be able to write back the original data 1:1.
fn to_vec_gen2() {
    let ifd = IFD::parse(IFD_DATA_GEN2).unwrap();
    assert_eq!(ifd.to_vec(), IFD_DATA_GEN2);
}
#[test]
/// We should be able to write back the original data 1:1.
fn to_vec_gen3() {
    let ifd = IFD::parse(IFD_DATA_GEN3).unwrap();
    assert_eq!(ifd.to_vec(), IFD_DATA_GEN3);
}
