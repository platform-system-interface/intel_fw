use core::fmt::{self, Display};
use serde::{Deserialize, Serialize};
use zerocopy_derive::{FromBytes, IntoBytes};

#[derive(IntoBytes, FromBytes, Serialize, Deserialize, Clone, Copy, Debug)]
#[repr(C)]
pub struct Version {
    major: u16,
    minor: u16,
    patch: u16,
    build: u16,
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
