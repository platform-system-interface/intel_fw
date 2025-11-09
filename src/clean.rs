use intel_fw::{
    ifd::{IFD, IfdError},
    me::ME,
};
use log::warn;

pub struct Options {
    pub relocate: bool,
    pub disable_me: bool,
    pub disable_me_only: bool,
}

pub fn clean(
    ifd: &Result<IFD, IfdError>,
    me: &ME,
    data: &mut [u8],
    options: Options,
) -> Result<Vec<u8>, String> {
    if (options.disable_me || options.disable_me_only)
        && let Ok(ifd) = ifd
    {
        let mut new_ifd = ifd.clone();
        if let Err(e) = new_ifd.disable_me(&me.generation, &me.version) {
            let msg = format!("Could not disable ME: {e}");
            if options.disable_me_only {
                return Err(msg);
            }
            warn!("{msg}");
        } else {
            let new_ifd = new_ifd.to_vec();
            let size = new_ifd.len();
            data[..size].copy_from_slice(&new_ifd);
        }
        if options.disable_me_only {
            return Ok(data.to_vec());
        }
    }
    let mut new_me = me.clone();
    new_me.fpt_area.clean();
    if options.relocate {
        if let Err(e) = new_me.fpt_area.relocate_partitions() {
            warn!("Could not relocate: {e}")
        }
    }
    let cleaned = new_me.fpt_area.to_vec();
    let size = cleaned.len();
    data[me.base..me.base + size].copy_from_slice(&cleaned);
    Ok(data.to_vec())
}
