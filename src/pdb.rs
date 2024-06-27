use crate::utils::TimeDateStamp;
use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use parking_lot::FairMutex;
use pdb::{FallibleIterator, SymbolData, PDB};
use std::{collections::BTreeMap, fs::File, path::{Path, PathBuf}, sync::Arc};
use tracing::error;
use linked_hash_map::LinkedHashMap;

#[derive(Debug)]
pub struct ProcedureInfo {
    pub name: String,
    pub offset: u32,
    #[allow(unused)]
    pub global: bool,
    pub line_map: BTreeMap<u32, String>,
}

static PDB_PATH: Lazy<FairMutex<String>> = Lazy::new(|| FairMutex::new(String::new()));

pub fn pdb_path_set(path: &str) {
    let path = if path.ends_with("\\") {
        &path[..path.len() - 1]
    } else {
        path
    };
    *PDB_PATH.lock() = path.to_string();
}

pub fn pdb_path_get() -> String {
    PDB_PATH.lock().clone()
}

static PDBS_LOADED: Lazy<FairMutex<LinkedHashMap<(PathBuf, u32), Option<Arc<BTreeMap<u32, ProcedureInfo>>>>>> = Lazy::new(|| {
    FairMutex::new(LinkedHashMap::new())
});

pub fn get_pdb_info_for_module(module_name: &Path, module_time_date_stamp: u32) -> Option<Arc<BTreeMap<u32, ProcedureInfo>>> {
    let mut lock = PDBS_LOADED.lock();
    if let Some(pdb_info) = lock.get(&(module_name.to_path_buf(), module_time_date_stamp)) {
        if pdb_info.is_some() {
            return pdb_info.clone();
        }
    } else {
        lock.insert((module_name.to_path_buf(), module_time_date_stamp), None);
    }
    drop(lock);

    match get_pdb_info_from_pdb_file(module_name, module_time_date_stamp) {
        Err(e) => {
            error!("Faile to get_pdb_info_from_pdb_file for {}-{module_time_date_stamp} {e}", module_name.display());
            None
        }
        Ok(pdb_info) => {
            let arc = Arc::new(pdb_info);
            let mut lock = PDBS_LOADED.lock();
            let _ = lock.get_mut(&(module_name.to_path_buf(), module_time_date_stamp)).unwrap().insert(arc.clone());
        
            Some(arc)
        }
    }
}

// module_name: file name i.e. system_monitor.exe
fn get_pdb_info_from_pdb_file(
    module_name: &Path,
    module_time_date_stamp: u32
) -> Result<BTreeMap<u32, ProcedureInfo>> {
    let module_file_name = if let Some(module_file_name) = module_name.file_name() {
        module_file_name
    } else {
        return Err(anyhow!("No file name for {}", module_name.display()));
    };
    let pdb_prefix = pdb_path_get();
    let pdb_path = Path::new(pdb_prefix.as_str())
        .join(module_file_name)
        .with_extension("pdb");

    let file = File::open(pdb_path.as_path()).with_context(|| {
        format!("Failed to open {}", pdb_path.display())
    })?;
    let mut pdb = PDB::open(file)?;
    let pdb_info = pdb.pdb_information()?;

    const TIME_DATE_STAMP_DIFF: u32 = 5;
    if pdb_info.signature.abs_diff(module_time_date_stamp) > TIME_DATE_STAMP_DIFF {
        return Err(anyhow!(
            "Unmatched TimeDateStamp (> {TIME_DATE_STAMP_DIFF}), module is {} pdb is {}",
            TimeDateStamp(module_time_date_stamp).to_string_detail(),
            TimeDateStamp(pdb_info.signature).to_string_detail()
        ));
    }

    let address_map = pdb.address_map()?;
    let string_table = pdb.string_table()?;
    let dbi = pdb.debug_information()?;
    let mut modules = dbi.modules()?;

    let mut map = BTreeMap::new();
    while let Some(module) = modules.next()? {
        let module_info = match pdb.module_info(&module)? {
            Some(info) => info,
            None => {
                continue;
            }
        };

        let program = module_info.line_program()?;
        let mut symbols = module_info.symbols()?;

        while let Some(symbol) = symbols.next()? {
            match symbol.parse() {
                Ok(data) => {
                    if let SymbolData::Procedure(proc) = data {
                        if let Some(proc_rva) = proc.offset.to_rva(&address_map) {
                            let mut line_map = BTreeMap::new();
                            let mut lines = program.lines_for_symbol(proc.offset);
                            while let Some(line_info) = lines.next()? {
                                if let Some(rva) = line_info.offset.to_rva(&address_map) {
                                    let file_info = program.get_file_info(line_info.file_index)?;
                                    let file_name =
                                        file_info.name.to_string_lossy(&string_table)?;
                                    line_map.insert(
                                        rva.0,
                                        format!("{file_name}: {}", line_info.line_start),
                                    );
                                }
                            }
                            map.insert(
                                proc_rva.0,
                                ProcedureInfo {
                                    name: format!("{}", proc.name),
                                    offset: proc_rva.0,
                                    global: proc.global,
                                    line_map: line_map,
                                },
                            );
                        }
                    }
                }
                Err(_e) => {
                    //warn!("{e} in file: {}", pdb_path.display());
                }
            }
        }
    }

    Ok(map)
}

#[cfg(test)]
mod tests {
    use std::{ops::Bound, path::Path};
    use crate::process_modules::get_image_info_from_file;

    #[test]
    fn get_pdb_info_from_pdb_file() {
        let out_dir = env!("CARGO_MANIFEST_DIR");
        let pkg_name = env!("CARGO_PKG_NAME");
        let module_info = get_image_info_from_file(Path::new(format!("{out_dir}\\target\\debug\\{pkg_name}.exe").as_str())).unwrap();
        super::pdb_path_set(format!("{out_dir}\\target\\debug\\pdb").as_str());
        let r = super::get_pdb_info_from_pdb_file(&Path::new(format!("{pkg_name}.exe").as_str()), module_info.1);
        match r {
            Ok(map) => {
                let address: u32 = 0x2b6168;
                let cursor = map.upper_bound(Bound::Included(&address));
                if let Some(info) = cursor.value() {
                    let cursor_line = info.line_map.upper_bound(Bound::Included(&address));
                    if let Some(line) = cursor_line.value() {
                        println!("{}: {line}", info.name);
                    }
                }
            }
            Err(e) => {
                println!("{e}");
            }
        }
    }

    #[test]
    fn get_pdb_info_for_module() {
        let out_dir = env!("CARGO_MANIFEST_DIR");
        let pkg_name = env!("CARGO_PKG_NAME");
        let module_info = get_image_info_from_file(Path::new(format!("{out_dir}\\target\\debug\\{pkg_name}.exe").as_str())).unwrap();
        super::pdb_path_set(format!("{out_dir}\\target\\debug\\pdb").as_str());
        if let Some(arc) = super::get_pdb_info_for_module(&Path::new(format!("{pkg_name}.exe").as_str()), module_info.1) {
            let address: u32 = 0x2b6168;
            let cursor = arc.upper_bound(Bound::Included(&address));
            if let Some(info) = cursor.value() {
                let cursor_line = info.line_map.upper_bound(Bound::Included(&address));
                if let Some(line) = cursor_line.value() {
                    println!("{}: {line}", info.name);
                }
            }
        }
    }
}
