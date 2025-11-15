use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, IntoBytes};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

use crate::ver::Version;

const VENDOR_INTEL: u32 = 0x8086;
const MANIFEST2_MAGIC: &str = "$MN2";
const MANIFEST2_MAGIC_BYTES: &[u8] = MANIFEST2_MAGIC.as_bytes();

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Date {
    day: u8,
    month: u8,
    year: u16,
}

impl Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Date { year, month, day } = self;
        write!(f, "{year:04x}-{month:02x}-{day:02x}")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Vendor(u32);

impl Display for Vendor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = self.0;
        let v = match id {
            VENDOR_INTEL => "Intel",
            _ => "unknown",
        };
        write!(f, "{v} ({id:04x})")
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct HeaderVersion {
    minor: u16,
    major: u16,
}

impl Display for HeaderVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let HeaderVersion { major, minor } = self;
        write!(f, "{major}.{minor}")
    }
}

// https://github.com/skochinsky/me-tools me_unpack.py MeManifestHeader
#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Header {
    pub mod_type: u16,
    pub mod_subtype: u16,
    pub header_len: u32, // in dwords, usually 0xa1, i.e., 0x284 bytes
    pub header_ver: HeaderVersion,
    pub flags: u32,
    pub vendor: Vendor,
    pub date: Date,
    pub size: u32, // in dwords, dword size is 32bit
    pub magic: [u8; 4],
    // NOTE: only for Gen 2 ME firmware
    pub entries: u32,
    pub version: Version,
    xx0: u32, // e.g. 0x0000_0001
    _30: u32, // e.g. all zero
    xxx: u32, // e.g. 0x0000_0003
    #[serde(with = "serde_bytes")]
    _38: [u8; 0x40], // e.g. all zero
    pub key_size: u32, // in dwords
    pub scratch_size: u32,
}

impl Display for Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hver = self.header_ver;
        let hlen = self.header_len;
        let ver = self.version;
        let date = self.date;
        let ven = self.vendor;
        let e = self.entries;
        write!(
            f,
            "{hver} {hlen:04x} vendor {ven}, version {ver} {date}, {e} entries"
        )
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Manifest {
    pub header: Header,
    #[serde(with = "serde_bytes")]
    pub rsa_pub_key: [u8; 0x100],
    pub rsa_pub_exp: u32,
    #[serde(with = "serde_bytes")]
    pub rsa_sig: [u8; 0x100],
}

pub const MANIFEST_SIZE: usize = core::mem::size_of::<Manifest>();

impl<'a> Manifest {
    pub fn new(data: &'a [u8]) -> Result<Self, String> {
        let (manifest, _) = match Self::read_from_prefix(data) {
            Ok(r) => r,
            Err(e) => {
                let err = format!("Manifest cannot be parsed: {e:?}");
                return Err(err);
            }
        };

        if manifest.header.magic != MANIFEST2_MAGIC_BYTES {
            let err = format!(
                "Manifest magic not found: wanted {MANIFEST2_MAGIC_BYTES:02x?} ({MANIFEST2_MAGIC}), got {:02x?}",
                manifest.header.magic
            );
            return Err(err);
        }

        Ok(manifest)
    }

    /// Get the header length
    pub fn header_len(&self) -> usize {
        self.header.header_len as usize * 4
    }

    /// Get the size of the manifest and its data
    pub fn size(&self) -> usize {
        self.header.size as usize * 4
    }

    /// Get the length of the data after the manifest
    pub fn data_len(&self) -> usize {
        let mlen = self.size();
        let hlen = self.header_len();
        mlen - hlen
    }

    /// Get the MD5 hash over the RSA public key and exponent.
    pub fn hash_key(self: Self) -> Vec<u8> {
        let k = self.rsa_pub_key.as_bytes();
        let e = self.rsa_pub_exp;
        let ke = [k, &e.to_le_bytes()].concat();
        md5::compute(ke).to_vec()
    }
}

impl Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let h = self.header;
        let exp = self.rsa_pub_exp;
        write!(f, "{h}, RSA exp {exp}")
    }
}
