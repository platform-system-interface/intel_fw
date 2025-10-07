use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy::{FromBytes, IntoBytes};
use zerocopy_derive::{FromBytes, IntoBytes};

use crate::ver::Version;

const VENDOR_INTEL: u32 = 0x8086;
const MANIFEST2_MAGIC: &[u8] = b"$MN2";

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
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

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
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

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
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
#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
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

const HEADER_SIZE: usize = core::mem::size_of::<Header>();
const KEY_SIZE: usize = 0x100;

#[derive(Serialize, Deserialize, IntoBytes, FromBytes, Clone, Copy, Debug)]
#[repr(C)]
pub struct Manifest {
    pub header: Header,
    #[serde(with = "serde_bytes")]
    pub rsa_pub_key: [u8; KEY_SIZE],
    pub rsa_pub_exp: u32,
    #[serde(with = "serde_bytes")]
    pub rsa_sig: [u8; KEY_SIZE],
}

pub const MANIFEST_SIZE: usize = core::mem::size_of::<Manifest>();

impl<'a> Manifest {
    pub fn new(data: &'a [u8]) -> Result<Self, String> {
        let (header, _) = Header::read_from_prefix(data).unwrap();

        if header.magic != *MANIFEST2_MAGIC {
            let err = format!("manifest magic not found, got: {:02x?}", header.magic);
            return Err(err);
        }

        let o = HEADER_SIZE;
        let rsa_pub_key: [u8; KEY_SIZE] = data[o..o + KEY_SIZE].try_into().unwrap();
        let o = o + KEY_SIZE;
        let (rsa_pub_exp, _) = u32::read_from_prefix(&data[o..o + 4]).unwrap();
        let o = o + 4;
        let rsa_sig: [u8; KEY_SIZE] = data[o..o + KEY_SIZE].try_into().unwrap();

        let m = Self {
            header,
            rsa_pub_key,
            rsa_pub_exp,
            rsa_sig,
        };

        Ok(m)
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
