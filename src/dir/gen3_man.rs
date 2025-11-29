use core::fmt::Display;

use asn1::SimpleAsn1Readable;
use bitfield_struct::bitfield;
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, Immutable, Ref, TryFromBytes};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes, TryFromBytes};

use crate::{dump48, ver::Version};

const DEBUG: bool = true;

/// Module names occur in multiple extensions.
const MOD_NAME_LEN: usize = 12;
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
struct ModName([u8; MOD_NAME_LEN]);

impl Display for ModName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = match std::str::from_utf8(&self.0) {
            Ok(n) => n.trim_end_matches('\0'),
            Err(_) => "???",
        };
        write!(f, "{n}")
    }
}

const DIR_NAME_LEN: usize = 4;
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct DirName([u8; DIR_NAME_LEN]);

impl Display for DirName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = std::str::from_utf8(&self.0).unwrap_or("????");
        write!(f, "{n:4}")
    }
}

/// Hash in reverse byte order ("little endian"), occurs multiple times
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Hash([u8; 32]);

impl Hash {
    pub fn get(&self) -> [u8; 32] {
        let mut r = self.0;
        r.reverse();
        r
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let short_hash = &self.get()[..8];
        write!(f, "{short_hash:02x?}")
    }
}

/// Parse a header followed by entries
fn parse_header_and_entries<H, E>(data: &[u8]) -> Result<(H, Vec<E>), String>
where
    H: FromBytes,
    E: FromBytes + Immutable + Clone,
{
    let (header, rest) = match H::try_read_from_prefix(data) {
        Ok(r) => r,
        Err(e) => return Err(format!("cannot parse header: {e:?}")),
    };
    let l = rest.len();
    let e = size_of::<E>();
    let s = e * (l / e);
    let slice = &rest[..s];
    let entries = match Ref::<_, [E]>::from_bytes(slice) {
        Ok(r) => r,
        Err(e) => return Err(format!("cannot parse entries: {e}")),
    };
    let rem = &rest[s..];
    if !rem.is_empty() {
        println!("Remaining: {rem:02x?}");
    }
    let entries = entries.to_vec();
    Ok((header, entries))
}

/// Every extension must implement this trait.
trait Parseable {
    fn parse(data: &[u8]) -> Result<Box<Self>, String>;
}

/// Init Script Extension, ME Gen 3 Version 11
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InitScriptExtV11 {
    header: InitScriptHeader,
    entries: Vec<InitScriptEntryV11>,
}

/// Init Script Extension, ME Gen 3 Version 12
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct InitScriptExtV12 {
    header: InitScriptHeader,
    entries: Vec<InitScriptEntryV12>,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct InitScriptHeader {
    _r: u32,
    entries: u32,
}

#[bitfield(u32)]
#[derive(Immutable, FromBytes, IntoBytes, Serialize, Deserialize)]
pub struct InitFlags {
    pub ibl: bool,
    _x: bool,
    pub init_immediately: bool,
    #[bits(13)]
    _r: u16,
    pub cm0_u: bool,
    #[bits(15)]
    _r: u16,
}

#[bitfield(u32)]
#[derive(Immutable, FromBytes, IntoBytes, Serialize, Deserialize)]
pub struct BootType {
    pub normal: bool,
    #[bits(3)]
    _r: u16,
    pub recovery: bool,
    #[bits(27)]
    _r: u32,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct InitScriptEntryV11 {
    dir: DirName,
    name: ModName,
    init_flags: InitFlags,
    boot_type: BootType, // mostly 0001 0001, bit 4 appears to mean FTPR
}

impl Display for InitScriptEntryV11 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.dir;
        let n = format!("{:12}", self.name);
        let ini = self.init_flags;
        let ibl = if ini.ibl() { "IBL" } else { "" };
        let ii = if ini.init_immediately() {
            "immediately"
        } else {
            ""
        };
        let c = if ini.cm0_u() { "CM0 U" } else { "" };
        let bt = self.boot_type;
        let no = if bt.normal() { "normal" } else { "" };
        let rec = if bt.recovery() { "recovery" } else { "" };
        write!(
            f,
            "{d}: {n:12}  |  {ibl:3}  {ii:12}  {c:5}  |  {no:6}  {rec:8}"
        )
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct InitScriptEntryV12 {
    dir: DirName,
    name: ModName,
    init_flags: InitFlags,
    boot_type: BootType, // mostly 0001 0001, bit 4 appears to mean FTPR
    unk: u32,
}

impl Display for InitScriptEntryV12 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.dir;
        let n = format!("{:12}", self.name);
        let ini = self.init_flags;
        let ibl = if ini.ibl() { "IBL" } else { "" };
        let ii = if ini.init_immediately() {
            "immediately"
        } else {
            ""
        };
        let c = if ini.cm0_u() { "CM0 U" } else { "" };
        let bt = self.boot_type;
        let no = if bt.normal() { "normal" } else { "" };
        let rec = if bt.recovery() { "recovery" } else { "" };
        let u = self.unk;
        write!(
            f,
            "{d}: {n:12}  |  {ibl:3}  {ii:12}  {c:5}  |  {no:6}  {rec:8}  | {u:x}"
        )
    }
}

impl InitScriptExtV11 {
    pub fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, rest) = match InitScriptHeader::try_read_from_prefix(data) {
            Ok(r) => r,
            Err(e) => return Err(format!("cannot parse System Info header: {e:?}")),
        };
        let count = header.entries as usize;
        let entries = match Ref::<_, [InitScriptEntryV11]>::from_prefix_with_elems(rest, count) {
            Ok((entries, _)) => entries,
            Err(e) => {
                return Err(format!("cannot parse System Info entries: {e}"));
            }
        };
        let entries = entries.to_vec();
        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for InitScriptExtV11 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.header.entries;
        writeln!(
            f,
            "{count} init entries     |        init flags          |      boot type"
        )?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

impl InitScriptExtV12 {
    pub fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, rest) = match InitScriptHeader::try_read_from_prefix(data) {
            Ok(r) => r,
            Err(e) => return Err(format!("cannot parse System Info header: {e:?}")),
        };
        let count = header.entries as usize;
        let entries = match Ref::<_, [InitScriptEntryV12]>::from_prefix_with_elems(rest, count) {
            Ok((entries, _)) => entries,
            Err(e) => {
                return Err(format!("cannot parse System Info entries: {e}"));
            }
        };
        let entries = entries.to_vec();
        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for InitScriptExtV12 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let count = self.header.entries;
        writeln!(
            f,
            "{count} init entries     |        init flags          |      boot type"
        )?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

/// Partition Info Extension describes per-file additional metadata.
/// There is one metadata file per CPD entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PartitionInfoExt {
    header: PartitionInfoHeader,
    entries: Vec<PartitionInfoEntry>,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PartitionInfoHeader {
    dir: DirName,
    size: u32,
    hash: Hash,
    vcn: u32, // version control number
    part_ver: u32,
    data_ver: u32,
    inst_id: u32,
    flags: u32,
    _r: [u8; 16],
    _u: u32,
}

impl Display for PartitionInfoHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dn = self.dir;
        let s = self.size;
        let h = self.hash;
        let v = self.vcn;
        write!(f, "{dn}:   v{v},  {s:08x} bytes,  {h}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PartitionInfoEntry {
    name: ModName,
    y1: u32,
    size: u32,
    hash: Hash, // SHA256 of respective metadata file
}

impl Display for PartitionInfoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.name();
        let y1 = self.y1;
        let s = self.size;
        let hash = &self.hash;
        write!(f, "{n:12}  {y1:04x} {s:08x}  {hash}")
    }
}

impl PartitionInfoEntry {
    pub fn name(&self) -> String {
        format!("{}.met", self.name)
    }
}

impl PartitionInfoExt {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, entries) =
            match parse_header_and_entries::<PartitionInfoHeader, PartitionInfoEntry>(data) {
                Ok(r) => r,
                Err(e) => return Err(format!("Partition Info: {e}")),
            };
        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for PartitionInfoExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", &self.header)?;
        let count = &self.entries.len();
        writeln!(f, "{count:2} entries      ???      size         hash")?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

/// Feature Permission Extension
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FeaturePermissionsExt {
    header: FeaturePermissionsHeader,
    entries: Vec<FeaturePermissionsEntry>,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FeaturePermissionsHeader {
    count: u32,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct FeaturePermissionsEntry {
    uid: u16,
    _r: u16,
}

impl Parseable for FeaturePermissionsExt {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, entries) =
            match parse_header_and_entries::<FeaturePermissionsHeader, FeaturePermissionsEntry>(
                data,
            ) {
                Ok(r) => r,
                Err(e) => return Err(format!("Feature Permissions: {e}")),
            };

        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for FeaturePermissionsExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = &self.header;
        let c = h.count;
        writeln!(f, "{c:2} feature permissions")?;
        for e in &self.entries {
            writeln!(f, "{e:?}")?;
        }
        write!(f, "")
    }
}

/// Client System Info Extension
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ClientSystemInfoExt {
    header: ClientSysInfoHeader,
    entries: Vec<ClientSysInfoEntry>,
}

// TODO: bitfields
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ClientSysInfoHeader {
    fw_sku_caps: u32,
    _r: [u8; 28],
    bf_fw_sku_attrs: u64,
}

impl Display for ClientSysInfoHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let caps = self.fw_sku_caps;
        let c = format!("firmware SKU capabilities: {caps:08x}");
        let attrs = self.bf_fw_sku_attrs;
        let a = format!("BF firmware SKU attributes: {attrs:08x}");
        write!(f, "{c}, {a}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ClientSysInfoEntry {
    z1: u32,
    z2: u32,
    dir: DirName,
    size: u32,
    hash: Hash,
}

impl Display for ClientSysInfoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let z1 = self.z1;
        let z2 = self.z2;
        let dn = self.dir;
        let s = self.size;
        let hash = &self.hash;
        let xx = format!("{z1:08x} {z2:08x}");
        write!(f, "{dn}:  0x{s:08x} bytes, {hash} / {xx}")
    }
}

impl Parseable for ClientSystemInfoExt {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, entries) =
            match parse_header_and_entries::<ClientSysInfoHeader, ClientSysInfoEntry>(data) {
                Ok(r) => r,
                Err(e) => return Err(format!("Client System Info: {e}")),
            };
        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for ClientSystemInfoExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = &self.header;
        writeln!(f, "{h}")?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

/// System Info Extension
/// This describes per-file additional metadata.
/// There is one metadata file per CPD entry.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemInfoExtV11 {
    header: SystemInfoHeaderV11,
    entries: Vec<SystemInfoEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SystemInfoExtV15 {
    header: SystemInfoHeaderV15,
    entries: Vec<SystemInfoEntry>,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct SystemInfoHeaderV11 {
    uma_size: u32,
    chipset_ver: u32,
    hash: Hash,
    pageable_uma_size: u32,
    _r0: u64,
    _r1: u32,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct SystemInfoHeaderV15 {
    uma_size: u32,
    chipset_ver: u32,
    hash: Hash,
    pageable_uma_size: u32,
    _r0: u64,
    _r1: u32,
    _r2: u32,
    _r3: u32,
    _r4: u32,
    _r5: u32,
}

impl Display for SystemInfoHeaderV11 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let us = self.uma_size;
        let cv = self.chipset_ver;
        let ps = self.pageable_uma_size;
        let h = &self.hash;
        write!(
            f,
            "System Info v{cv:08x}, UMA: {us:08x}, pageable: {ps:08x}  {h}"
        )
    }
}

impl Display for SystemInfoHeaderV15 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let us = self.uma_size;
        let cv = self.chipset_ver;
        let ps = self.pageable_uma_size;
        let h = &self.hash;
        write!(
            f,
            "System Info v{cv:08x}, UMA: {us:08x}, pageable: {ps:08x}  {h}"
        )
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct SystemInfoEntry {
    dir: DirName,
    d1: u32,
    d2: u32,
}

impl Display for SystemInfoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let dn = self.dir;
        let d1 = self.d1;
        let d2 = self.d2;
        write!(f, "{dn}:       0x{d1:08x} 0x{d2:08x}")
    }
}

impl Parseable for SystemInfoExtV11 {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, entries) =
            match parse_header_and_entries::<SystemInfoHeaderV11, SystemInfoEntry>(data) {
                Ok(r) => r,
                Err(e) => return Err(format!("System Info: {e}")),
            };
        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for SystemInfoExtV11 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", &self.header)?;
        let count = &self.entries.len();
        writeln!(f, "{count:2} entries      ??         ??")?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

impl Parseable for SystemInfoExtV15 {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let (header, entries) =
            match parse_header_and_entries::<SystemInfoHeaderV15, SystemInfoEntry>(data) {
                Ok(r) => r,
                Err(e) => return Err(format!("System Info: {e}")),
            };
        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for SystemInfoExtV15 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", &self.header)?;
        let count = &self.entries.len();
        writeln!(f, "{count:2} entries      ??         ??")?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

/// xxx
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PackageInfoExt {
    header: PackageInfoHeader,
    entries: Vec<PackageInfoEntry>,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PackageInfoHeader {
    dir: DirName,
    vcn: u32,
    usage: [u8; 16],
    svn: u32,
    r0: u32,
    _r: [u8; 12],
}

impl Display for PackageInfoHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.dir;
        let v = self.vcn;
        let u = self.usage;
        let s = self.svn;
        let r = self.r0;
        write!(f, "{d}: {v:08x} {u:02x?} {s:08x} ({r:08x})")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct PackageInfoEntry {
    name: ModName,
    pkg_type: u8,
    hash_algo: u8,
    hash_size: u16,
    metadata_size: u32,
    metadata_hash: Hash,
}

impl Display for PackageInfoEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.name;
        let n = format!("{n}");
        let pt = self.pkg_type;
        let ha = self.hash_algo;
        let hs = self.hash_size;
        let ms = self.metadata_size;
        let mh = self.metadata_hash;
        write!(f, "{n:12}: {pt} {ha}  {hs}; {ms:08x}, {mh}")
    }
}

impl Parseable for PackageInfoExt {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let Ok((header, rest)) = PackageInfoHeader::try_read_from_prefix(data) else {
            return Err("cannot parse Package Info header".into());
        };
        let l = rest.len();
        let es = size_of::<PackageInfoEntry>();
        // FIXME: This is just a workaround.
        // We have encountered samples with a single entry followed by another
        // 16 high-entropy bytes in the RBEP CPD.
        let s = (l / es) * es;
        let slice = &rest[..s];
        let entries = match Ref::<_, [PackageInfoEntry]>::from_bytes(slice) {
            Ok(r) => r,
            Err(e) => {
                return Err(format!(
                    "cannot parse Package Info entries of size {es} each from {l} remaining bytes: {e}"
                ));
            }
        };
        if false {
            let rem = &rest[s..];
            println!("Remainder: {rem:02x?}");
        }
        let entries = entries.to_vec();

        Ok(Box::new(Self { header, entries }))
    }
}

impl Display for PackageInfoExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Package Info {}", &self.header)?;
        let count = &self.entries.len();
        writeln!(f, "{count:2} entries      ??         ??")?;
        for e in &self.entries {
            writeln!(f, "{e}")?;
        }
        write!(f, "")
    }
}

/// Unknown 22 Extension
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Unk22Ext {
    data: Unk22Data,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Unk22Data {
    dir: DirName,
    xx: [u8; 24],
    hash: Hash,
    _r: [u8; 20],
}

impl Display for Unk22Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.dir;
        let h = self.hash;
        let x = self.xx;
        write!(f, "{d}: {h}    {x:02x?}")
    }
}

impl Parseable for Unk22Ext {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let Ok((data, _)) = Unk22Data::try_read_from_prefix(data) else {
            return Err("cannot parse Unk 22 data".into());
        };
        Ok(Box::new(Self { data }))
    }
}

impl Display for Unk22Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = &self.data;
        write!(f, "{h}")
    }
}

/// Certiticate Revocation Extension, found in CSME v15 RBEP CPDs
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct CertRevocationExt {
    header: CertRevocationHeader,
    data: CertRevocationData,
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct CertRevocationHeader {
    size: u32,
}

impl Display for CertRevocationHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self.size;
        write!(f, "{s} bytes")
    }
}

#[derive(asn1::Asn1Read, asn1::Asn1Write, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct CertRevocationData {
    r: u32,
    s: u32,
}

impl Parseable for CertRevocationExt {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let Ok((header, rest)) = CertRevocationHeader::try_read_from_prefix(data) else {
            return Err("cannot parse Certificate Revocation header".into());
        };
        let s = header.size as usize;
        let slice = &rest[..s];
        if false {
            let data: CertRevocationData = match serde_asn1_der::from_bytes(slice) {
                Ok(d) => d,
                Err(e) => return Err(format!("cannot parse Certificate Revocation data: {e}")),
            };
        }
        let data = match CertRevocationData::parse_data(slice) {
            Ok(d) => d,
            Err(e) => return Err(format!("cannot parse Certificate Revocation data: {e}")),
        };
        Ok(Box::new(Self { header, data }))
    }
}

impl Display for CertRevocationExt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = &self.data;
        write!(f, "{d:?}")
    }
}

/// Unknown 31 Extension
#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
pub struct Unk31Ext {
    data: Unk31Data,
}

// TODO
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct Unk31Data {
    dir: DirName,
    xx: [u8; 24],
    hash: Hash,
    _r: [u8; 20],
}

impl Display for Unk31Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let d = self.dir;
        let h = self.hash;
        let x = self.xx;
        write!(f, "{d}: {h}    {x:02x?}")
    }
}

impl Parseable for Unk31Ext {
    fn parse(data: &[u8]) -> Result<Box<Self>, String> {
        let Ok((data, _)) = Unk31Data::try_read_from_prefix(data) else {
            return Err("cannot parse Unk 31 data".into());
        };
        Ok(Box::new(Self { data }))
    }
}

impl Display for Unk31Ext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let h = &self.data;
        write!(f, "{h}")
    }
}

/// Extension tags describe extensions. Use this to discriminate.
#[derive(Immutable, IntoBytes, TryFromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(u32)]
pub enum ExtTag {
    SystemInfo = 0,
    InitScript = 1,
    FeaturePermissions = 2,
    PartitionInfo = 3,
    SharedLib = 4,
    ManProcess = 5,
    Threads = 6,
    DeviceIds = 7,
    MmioRanges = 8,
    SpecialFileProducer = 9,
    ModAttr = 10,
    LockedRanges = 11,
    ClientSystemInfo = 12,
    UserInfo = 13,
    None = 14,
    PackageInfo = 15,
    Unk16 = 16,
    Unk18 = 18,
    Unk22 = 22,
    CertRevocation = 30,
    Unk31 = 31,
    Unk50 = 50,
}

#[derive(Immutable, IntoBytes, TryFromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C, packed)]
pub struct ExtDescriptor {
    tag: ExtTag,
    size: u32,
}

const EXT_DESCRIPTOR_SIZE: usize = size_of::<ExtDescriptor>();

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ManExtension {
    SystemInfoV11(Result<Box<SystemInfoExtV11>, String>),
    SystemInfoV15(Result<Box<SystemInfoExtV15>, String>),
    InitScriptV11(Result<Box<InitScriptExtV11>, String>),
    InitScriptV12(Result<Box<InitScriptExtV12>, String>),
    FeaturePermissions(Result<Box<FeaturePermissionsExt>, String>),
    PartitionInfo(Result<Box<PartitionInfoExt>, String>),
    ClientSystemInfo(Result<Box<ClientSystemInfoExt>, String>),
    PackageInfo(Result<Box<PackageInfoExt>, String>),
    Unk22(Result<Box<Unk22Ext>, String>),
    CertRevocation(Result<Box<CertRevocationExt>, String>),
    Unk31(Result<Box<Unk31Ext>, String>),
}

pub struct ManExtensions {
    pub exts: Vec<ManExtension>,
}

fn read_u32(data: &[u8]) -> u32 {
    u32::from_le_bytes([data[0], data[1], data[2], data[3]])
}

impl ManExtensions {
    pub fn parse(data: &[u8], me_ver: Version) -> Result<Self, String> {
        let mut exts = vec![];

        let mut o = 0;
        let l = data.len();
        if DEBUG {
            println!("Manifest: {l:08x} bytes of extensions");
        }
        while o + size_of::<ExtDescriptor>() < l {
            // Every extension starts with a descriptor.
            let (ext, rest) = match ExtDescriptor::try_read_from_prefix(&data[o..]) {
                Ok(r) => r,
                Err(e) => {
                    // Manually obtain what we encountered for debugging.
                    let tag = read_u32(&data[o..]);
                    let size = read_u32(&data[o + 4..]);
                    return Err(format!(
                        "cannot parse extension tag {tag}, {size} bytes @ {o:08x}: {e:?}"
                    ));
                }
            };
            if DEBUG {
                println!("- {o:08x}: {ext:04x?}");
            }

            // Get the actualy data slice and check bounds.
            let ext_size = ext.size as usize;
            let data_size = ext_size - EXT_DESCRIPTOR_SIZE;
            let rlen = rest.len();
            if data_size > rlen {
                return Err(format!("only got {rlen} bytes left, need {data_size}"));
            }

            let slice = &rest[..data_size];
            match ext.tag {
                ExtTag::SystemInfo => match (me_ver.major, me_ver.minor) {
                    (15, _) => {
                        let r = SystemInfoExtV15::parse(slice);
                        exts.push(ManExtension::SystemInfoV15(r));
                    }
                    (_, _) => {
                        let r = SystemInfoExtV11::parse(slice);
                        exts.push(ManExtension::SystemInfoV11(r));
                    }
                },
                ExtTag::InitScript => match (me_ver.major, me_ver.minor) {
                    (11, _) => {
                        let r = InitScriptExtV11::parse(slice);
                        exts.push(ManExtension::InitScriptV11(r));
                    }
                    (15, 40) => {
                        let r = InitScriptExtV12::parse(slice);
                        exts.push(ManExtension::InitScriptV12(r));
                    }
                    (12..=15, _) => {
                        let r = InitScriptExtV12::parse(slice);
                        exts.push(ManExtension::InitScriptV12(r));
                    }
                    (ma, mi) => todo!("support ME version {ma} {mi}"),
                },
                ExtTag::FeaturePermissions => {
                    let r = FeaturePermissionsExt::parse(slice);
                    exts.push(ManExtension::FeaturePermissions(r));
                }
                ExtTag::PartitionInfo => {
                    let r = PartitionInfoExt::parse(slice);
                    exts.push(ManExtension::PartitionInfo(r));
                }
                ExtTag::ClientSystemInfo => {
                    let r = ClientSystemInfoExt::parse(slice);
                    exts.push(ManExtension::ClientSystemInfo(r));
                }
                ExtTag::PackageInfo => {
                    let r = PackageInfoExt::parse(slice);
                    exts.push(ManExtension::PackageInfo(r));
                }
                ExtTag::Unk22 => {
                    let r = Unk22Ext::parse(slice);
                    exts.push(ManExtension::Unk22(r));
                }
                ExtTag::CertRevocation => {
                    let r = CertRevocationExt::parse(slice);
                    exts.push(ManExtension::CertRevocation(r));
                }
                ExtTag::Unk31 => {
                    let r = Unk31Ext::parse(slice);
                    exts.push(ManExtension::Unk31(r));
                }
                tag => {
                    if DEBUG {
                        println!("  cannot yet parse {tag:?} @ {o:08x}");
                        dump48(&data[o..]);
                        dump48(&data[o + 48..]);
                        dump48(&data[o + 96..]);
                    }
                }
            }

            o += ext_size;
        }

        Ok(Self { exts })
    }
}
