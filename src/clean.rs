use log::info;

use intel_fw::dir::gen3::{self, CodePartitionDirectory};
use intel_fw::me::{Directories, ME};
use intel_fw::trans::Trans;

const EMPTY: u8 = 0xff;

fn gen3clean(me: &ME, dirs: &Vec<CodePartitionDirectory>, data: &mut [u8]) {
    let mut clean_fpt = me.fpt.clone();
    clean_fpt.clean();
    let mut clean_fpt = clean_fpt.to_vec();
    // TODO: This is arbitrary. We should save the original size when parsing.
    clean_fpt.resize(0x400, EMPTY);

    let o = me.base + me.fpt.offset;
    for (i, v) in clean_fpt.iter().enumerate() {
        data[o + i] = *v;
    }

    let blocklist = Vec::from(gen3::ALWAYS_RETAIN)
        .iter()
        .map(|s| String::from(*s))
        .collect();

    for d in dirs {
        if d.name == "FTPR" {
            let dir_offset = me.base + d.offset;
            info!("FPTR @ {dir_offset:08x}");
            let removables = d.clone().removable_entries(&blocklist);
            for (mod_offset, size) in removables {
                info!("freeing {size:6} bytes @ {mod_offset:08x}");
                for o in 0..size {
                    data[dir_offset + mod_offset + o] = EMPTY;
                }
            }

            let r = d.remainder();
            let e = me.fpt.entries.iter().find(|e| e.name() == "FTPR").unwrap();
            let end = dir_offset + e.size as usize;
            info!("Remaining space in FPTR: {r:08x}..{end:08x}");
            for o in r..end {
                data[o] = EMPTY;
            }
        }
    }

    for e in &me.fpt.entries {
        match e.name().as_str() {
            "FLOG" | "FTUP" | "IVBP" | "MFS" | "NFTP" | "PSVN" | "UTOK" => {
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
    match &me.dirs {
        Directories::Gen2(_) => todo!(),
        Directories::Gen3(dirs) => gen3clean(&me, &dirs, data),
        _ => return Err(()),
    };
    Ok(data.to_vec())
}
