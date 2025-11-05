use intel_fw::me::ME;

pub fn clean(me: &ME, data: &mut [u8]) -> Result<Vec<u8>, String> {
    let mut new_me = me.clone();
    new_me.fpt_area.clean();
    let cleaned = new_me.fpt_area.to_vec();
    let size = cleaned.len();
    data[me.base..me.base + size].copy_from_slice(&cleaned);
    Ok(data.to_vec())
}
