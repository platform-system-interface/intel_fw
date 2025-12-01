//! Common struct for version information
//!
//! This kind of version information occurs in multiple places.

use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes};

#[derive(Immutable, IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
    pub build: u16,
}

impl Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Version {
            major,
            minor,
            patch,
            build,
        } = self;
        write!(f, "{major}.{minor}.{patch}.{build}")
    }
}
