//! Adapted from me_cleaner, a map of key hashes => metadata (variant + version).

use std::fmt::Display;

use phf::phf_map;

/// Firmware variant:
/// - regular (ME) <https://www.intel.com/content/www/us/en/support/articles/000030079/software/chipset-software.html>
/// - Trusted Execution Engine (TXE) <https://www.intel.com/content/www/us/en/support/articles/000030081/software/chipset-software.html>
/// - Server Platform Services (SPS) <https://designintools.intel.com/intel-server-platform-services-sps-manageability-engine-me-firmware-tools.html>
#[derive(Clone, Copy, Debug)]
pub enum Variant {
    ME,
    TXE,
    SPS,
}

#[derive(Clone, Copy, Debug)]
pub struct Meta {
    pub variant: Variant,
    pub version: &'static [&'static str],
}

impl Display for Meta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let va = format!("firmware variant: {:?}", self.variant);
        let ve = format!("version range: {}", self.version.join("/"));
        write!(f, "{va}, {ve}")
    }
}

static KEY_TO_META: phf::Map<&'static str, Meta> = phf_map! {
    // (CS)ME
    "8431285d43b0f2a2f520d7cab3d34178" => Meta {
        variant: Variant::ME,
        version: &["2.0.x.x", "2.1.x.x", "2.2.x.x"],
    },
    "4c00dd06c28119b5c1e5bb8eb6f30596" => Meta {
        variant: Variant::ME,
        version: &["2.5.x.x", "2.6.x.x"],
    },
    "9c24077a7f7490967855e9c4c16c6b9e" => Meta {
        variant: Variant::ME,
        version: &["3.x.x.x"],
    },
    "bf41464be736f5520d80c67f6789020e" => Meta {
        variant: Variant::ME,
        version: &["4.x.x.x"],
    },
    "5c7169b7e7065323fb7b3b5657b4d57a" => Meta {
        variant: Variant::ME,
        version: &["5.x.x.x"],
    },
    "763e59ebe235e45a197a5b1a378dfa04" => Meta {
        variant: Variant::ME,
        version: &["6.x.x.x"],
    },
    "3a98c847d609c253e145bd36512629cb" => Meta {
        variant: Variant::ME,
        version: &["6.0.50.x"],
    },
    "0903fc25b0f6bed8c4ed724aca02124c" => Meta {
        variant: Variant::ME,
        version: &["7.x.x.x", "8.x.x.x"],
    },
    "2011ae6df87c40fba09e3f20459b1ce0" => Meta {
        variant: Variant::ME,
        version: &["9.0.x.x", "9.1.x.x"],
    },
    "e8427c5691cf8b56bc5cdd82746957ed" => Meta {
        variant: Variant::ME,
        version: &["9.5.x.x", "10.x.x.x"],
    },
    "986a78e481f185f7d54e4af06eb413f6" => Meta {
        variant: Variant::ME,
        version: &["11.x.x.x"],
    },
    "3efc26920b4bee901b624771c742887b" => Meta {
        variant: Variant::ME,
        version: &["12.x.x.x"],
    },
    "8e4f834644da2bef03039d69d41ecf02" => Meta {
        variant: Variant::ME,
        version: &["14.x.x.x"],
    },
    "b29411f89bf20ed177d411c46e8ec185" => Meta {
        variant: Variant::ME,
        version: &["15.x.x.x"],
     },
    "5887caf9b677601ffb257cc98a13d2a9" => Meta {
        variant: Variant::ME,
        version: &["16.x.x.x"]
    },
    // TXE
    "bda0b6bb8ca0bf0cac55ac4c4d55e0f2" => Meta {
        variant: Variant::TXE,
        version: &["1.x.x.x"],
    },
    "b726a2ab9cd59d4e62fe2bead7cf6997" => Meta {
        variant: Variant::TXE,
        version: &["1.x.x.x"],
    },
    "0633d7f951a3e7968ae7460861be9cfb" => Meta {
        variant: Variant::TXE,
        version: &["2.x.x.x"],
    },
    "1d0a36e9f5881540d8e4b382c6612ed8" => Meta {
        variant: Variant::TXE,
        version: &["3.x.x.x"],
    },
    // SPS
    "be900fef868f770d266b1fc67e887e69" => Meta {
        variant: Variant::SPS,
        version: &["2.x.x.x"],
    },
    "4622e3f2cb212a89c90a4de3336d88d2" => Meta {
        variant: Variant::SPS,
        version: &["3.x.x.x"],
    },
    "31ef3d950eac99d18e187375c0764ca4" => Meta {
        variant: Variant::SPS,
        version: &["4.x.x.x"],
    },
};

/// Get metadata for a given manifest signing key (pub key with exponent).
///
/// * `key_hash` - MD5 hash (hex str) of the key
pub fn get_meta_for_key(key_hash: &str) -> Option<&Meta> {
    KEY_TO_META.get(key_hash)
}
