use intel_fw::me::ME;
use log::warn;

pub struct Options {
    pub relocate: bool,
}

pub fn clean(me: &ME, data: &mut [u8], options: Options) -> Result<Vec<u8>, String> {
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
