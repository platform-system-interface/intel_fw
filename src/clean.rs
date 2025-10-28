use log::info;

use intel_fw::EMPTY;
use intel_fw::Removables;
use intel_fw::dir::gen2::{self, Directory as Gen2Directory};
use intel_fw::dir::gen3::{self, CodePartitionDirectory};
use intel_fw::fpt::{FTPR, REMOVABLE_PARTS};
use intel_fw::me::{Directories, ME};

fn fpt_clean(me: &ME, data: &mut [u8]) {
    let mut clean_fpt = me.fpt.clone();
    clean_fpt.clear();
    for (i, v) in clean_fpt.to_vec().iter().enumerate() {
        data[me.base + i] = *v;
    }
}

// Clear out removable ranges in the FTPR directory
fn dir_clean(
    dir: &dyn Removables,
    retention_list: &Vec<String>,
    dir_offset: usize,
    data: &mut [u8],
) {
    info!("FTPR @ {dir_offset:08x}");
    for r in dir.removables(&retention_list) {
        let offset = dir_offset + r.start;
        let size = r.end - r.start;
        info!("Freeing {size:8} bytes @ {offset:08x}");
        for o in r {
            data[dir_offset + o] = EMPTY;
        }
    }
}

fn gen2clean(me: &ME, dirs: &Vec<Gen2Directory>, data: &mut [u8]) {
    // TODO: Extend with user-provided list
    let retention_list = Vec::from(gen2::ALWAYS_RETAIN)
        .iter()
        .map(|s| String::from(*s))
        .collect();
    if let Some(d) = dirs.iter().find(|d| d.name == FTPR) {
        let dir_offset = me.base + d.offset;
        dir_clean(d, &retention_list, dir_offset, data);
    }

    // Clear out fully removable partitions
    for e in &me.fpt.entries {
        match e {
            // We dealt with the removable ranges in FTPR above.
            e if e.name().as_str() == FTPR => {
                info!("Retain {e}");
            }
            e if e.offset == 0xffff_ffff => {
                info!("Ignore {e} due to Invalid offset");
            }
            e => {
                let offset = me.base + e.offset as usize;
                let size = e.size as usize;
                let end = offset + size;
                info!("Remove {e}");
                for o in offset..end {
                    data[o] = EMPTY;
                }
            }
        }
    }
}

fn gen3clean(me: &ME, dirs: &Vec<CodePartitionDirectory>, data: &mut [u8]) {
    // TODO: Extend with user-provided list
    let retention_list = Vec::from(gen3::ALWAYS_RETAIN)
        .iter()
        .map(|s| String::from(*s))
        .collect();
    if let Some(d) = dirs.iter().find(|d| d.name == FTPR) {
        let dir_offset = me.base + d.offset;
        dir_clean(d, &retention_list, dir_offset, data);
    }

    // Clear out fully removable partitions
    for e in &me.fpt.entries {
        match e.name().as_str() {
            n if REMOVABLE_PARTS.contains(&n) => {
                let offset = me.base + e.offset as usize;
                let end = offset + e.size as usize;
                for o in offset..end {
                    data[o] = EMPTY;
                }
            }
            _ => {} //
        }
    }
}

pub fn clean(me: &ME, data: &mut [u8]) -> Result<Vec<u8>, ()> {
    fpt_clean(me, data);
    match &me.dirs {
        Directories::Gen2(dirs) => gen2clean(&me, &dirs, data),
        Directories::Gen3(dirs) => gen3clean(&me, &dirs, data),
        _ => return Err(()),
    };
    Ok(data.to_vec())
}
