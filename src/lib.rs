#![doc = include_str!("../README.md")]

use log::{info, warn};
use serde::{Deserialize, Serialize};

pub mod dir;
pub mod fit;
pub mod fpt;
pub mod ifd;
pub mod me;
pub mod meta;
pub mod ver;

use fit::{Fit, FitError};
use ifd::{IFD, IfdError};
use me::ME;

// An empty byte in a NOR flash is all-1's.
pub const EMPTY: u8 = 0xff;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Firmware {
    pub ifd: Result<IFD, IfdError>,
    pub me: Option<Result<ME, String>>,
    pub fit: Result<Fit, FitError>,
}

impl Firmware {
    pub fn parse(data: &[u8], debug: bool) -> Self {
        let ifd = IFD::parse(&data);
        let me = match &ifd {
            Ok(ifd) => {
                let me_region = ifd.regions.flreg2.range();
                let (b, l) = me_region;
                info!("ME region start @ {b:08x}");
                ME::parse(&data[b..l], b, debug)
            }
            Err(e) => {
                warn!("Not a full image: {e:?}");
                ME::parse(data, 0, debug)
            }
        };
        let fit = Fit::new(data);
        Self { ifd, me, fit }
    }

    pub fn scan(data: &[u8], debug: bool) -> Self {
        let ifd = IFD::parse(&data);
        let me = ME::scan(&data, debug);
        let fit = Fit::new(&data);
        Self { ifd, me, fit }
    }
}
