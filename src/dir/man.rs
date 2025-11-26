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
    pub manifest_len: u32, // in dwords, dword size is 32bit
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

impl Header {
    /// Get the header length including signature
    pub fn header_len(&self) -> usize {
        self.header_len as usize * 4
    }

    /// Get the length of the manifest including its data
    pub fn manifest_len(&self) -> usize {
        self.manifest_len as usize * 4
    }

    /// Get the length of the data after the header
    pub fn data_len(&self) -> usize {
        let mlen = self.manifest_len();
        let hlen = self.header_len();
        mlen - hlen
    }
}

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Signature {
    #[serde(with = "serde_bytes")]
    pub rsa_pub_key: [u8; 0x100],
    pub rsa_pub_exp: u32,
    #[serde(with = "serde_bytes")]
    pub rsa_sig: [u8; 0x100],
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[repr(C)]
pub struct Manifest {
    pub header: Header,
    pub signature: Signature,
    pub mdata: Vec<u8>,
}

pub const MANIFEST_SIZE: usize = core::mem::size_of::<Manifest>();

impl<'a> Manifest {
    pub fn new(data: &'a [u8]) -> Result<Self, String> {
        let (header, slice) = match Header::read_from_prefix(data) {
            Ok(r) => r,
            Err(e) => {
                let err = format!("Manifest cannot be parsed: {e:?}");
                return Err(err);
            }
        };

        if header.magic != MANIFEST2_MAGIC_BYTES {
            let err = format!(
                "Manifest magic not found: wanted {MANIFEST2_MAGIC_BYTES:02x?} ({MANIFEST2_MAGIC}), got {:02x?}",
                header.magic
            );
            return Err(err);
        }

        let (signature, _) = match Signature::read_from_prefix(slice) {
            Ok(r) => r,
            Err(e) => {
                let err = format!("Signature cannot be parsed: {e:?}");
                return Err(err);
            }
        };

        // The manifest carries additional data after its header and signature.
        // Note that header_len includes the signature.
        let header_len = header.header_len();
        let size = header.manifest_len();
        let mdata = &data[header_len..size];

        Ok(Self {
            header,
            signature,
            mdata: mdata.to_vec(),
        })
    }

    /// Get the MD5 hash over the RSA public key and exponent.
    pub fn hash_key(&self) -> Vec<u8> {
        let k = self.signature.rsa_pub_key.as_bytes();
        let e = self.signature.rsa_pub_exp;
        let ke = [k, &e.to_le_bytes()].concat();
        md5::compute(ke).to_vec()
    }

    /// Verify the manifest.
    pub fn verify(&self) -> bool {
        use num_bigint::BigUint;
        use sha2::{Digest, Sha256};

        let modulus = BigUint::from_bytes_le(&self.signature.rsa_pub_key);
        let exponent = BigUint::from(self.signature.rsa_pub_exp);
        let signature = BigUint::from_bytes_le(&self.signature.rsa_sig);
        let sb = signature.modpow(&exponent, &modulus).to_bytes_be();

        let header = self.header.as_bytes();
        let mut hasher = Sha256::new();
        hasher.update(header);
        hasher.update(&self.mdata);
        let hash = hasher.finalize();
        let hb = hash.as_bytes();

        let sl = sb.len();
        sb[sl - hb.len()..sl].eq(hb)
    }
}

impl Display for Manifest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let h = self.header;
        let exp = self.signature.rsa_pub_exp;
        write!(f, "{h}, RSA exp {exp}")
    }
}
