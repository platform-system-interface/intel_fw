// We compile to Wasm, so this is needed
// https://gist.github.com/JakeHartnell/2c1fa387f185f5dc46c9429470a2e2be
#[doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/docs/me_gen2_mfs.md"))]
pub mod gen2;
pub mod gen3;

pub fn parse(data: &[u8], extract_dir: Option<String>, verbose: bool) -> Result<bool, String> {
    if let Ok(res) = gen2::parse(data, extract_dir, verbose) {
        // TODO: library should not print; remove verbose flag eventually
        return Ok(res);
    }

    gen3::parse(data)
}
