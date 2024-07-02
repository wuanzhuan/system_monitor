use crate::utils::TimeDateStamp;
use anyhow::{anyhow, Context, Result};
use chumsky::container::Container;
use linked_hash_map::LinkedHashMap;
use once_cell::sync::Lazy;
use parking_lot::FairMutex;
use pdb::{FallibleIterator, StringRef, SymbolData, PDB};
use std::{
    collections::{BTreeMap, HashMap},
    fs::File,
    ops::Bound,
    path::{Path, PathBuf},
    sync::Arc,
};
use tracing::error;

#[derive(Debug)]
pub struct ProcedureInfo {
    pub name: String,
    pub rva: u32, // base by iamge start
    pub len: u32,
    #[allow(unused)]
    pub global: bool,
    pub line_map: BTreeMap<u32, LineInfo>,
}

#[derive(Debug)]
pub struct LineInfo {
    pub rva: u32, // base by iamge start
    pub length: Option<u32>,
    pub module_index: u32,
    pub file_name: StringRef,
    pub line_start: u32,
    #[allow(unused)]
    pub line_end: u32,
    #[allow(unused)]
    pub column_start: Option<u32>,
    #[allow(unused)]
    pub column_end: Option<u32>,
}

pub struct PdbInfo {
    file_name_map: HashMap<StringRef, String>,
    modules_files_vec: Vec<ModuleInfo>,
    functions_map: BTreeMap<u32, ProcedureInfo>,
}

pub struct FileInfo {
    #[allow(unused)]
    name: StringRef,
    // checksum
}

// corresponding a object file
pub struct ModuleInfo {
    module_name: String,
    #[allow(unused)]
    object_file_name: String,
    #[allow(unused)]
    files_map: HashMap<StringRef, FileInfo>, // todo: checksum
}

impl PdbInfo {
    pub fn get_location_info_by_offset(
        &self,
        offset: u32,
    ) -> (
        /*function_location*/ String,
        /*line_location*/ String,
    ) {
        let cursor = self.functions_map.upper_bound(Bound::Included(&offset));
        if let Some((_, procedure_info)) = cursor.peek_prev() {
            if offset < procedure_info.rva + procedure_info.len {
                let cursor_line = procedure_info
                    .line_map
                    .upper_bound(Bound::Included(&offset));
                if let Some((_, line_info)) = cursor_line.peek_prev() {
                    if let Some(len) = line_info.length {
                        if offset >= line_info.rva + len {
                            return (
                                format!(
                                    "{}+{:#x}",
                                    procedure_info.name,
                                    offset - procedure_info.rva
                                ),
                                String::new(),
                            );
                        }
                    }
                    let file_name = self.file_name_map.get(&line_info.file_name).unwrap();
                    return (
                        format!("{}+{:#x}", procedure_info.name, offset - procedure_info.rva),
                        format!(
                            "{} {file_name}: {}",
                            self.modules_files_vec[line_info.module_index as usize].module_name,
                            line_info.line_start
                        ),
                    );
                } else {
                    return (
                        format!("{}+{:#x}", procedure_info.name, offset - procedure_info.rva),
                        String::new(),
                    );
                }
            }
        }
        (String::new(), String::new())
    }
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

static PDBS_LOADED: Lazy<FairMutex<LinkedHashMap<(PathBuf, u32), Arc<PdbInfo>>>> =
    Lazy::new(|| FairMutex::new(LinkedHashMap::new()));

pub fn get_location_info(
    module_name: &Path,
    module_time_date_stamp: u32,
    offset: u32,
) -> Result<(
    /*function_location*/ String,
    /*line_location*/ String,
)> {
    if let Some(pdb_info) = PDBS_LOADED
        .lock()
        .get(&(module_name.to_path_buf(), module_time_date_stamp))
    {
        return Ok(pdb_info.get_location_info_by_offset(offset));
    }

    let pdb_info =
        get_pdb_info_from_pdb_file(module_name, module_time_date_stamp).with_context(|| {
            format!(
                "Faile to get_pdb_info_from_pdb_file for {}-{module_time_date_stamp}",
                module_name.display()
            )
        })?;
    let _ = PDBS_LOADED.lock().insert(
        (module_name.to_path_buf(), module_time_date_stamp),
        pdb_info.clone(),
    );
    Ok(pdb_info.get_location_info_by_offset(offset))
}

// module_name: file name i.e. system_monitor.exe
fn get_pdb_info_from_pdb_file(
    module_name: &Path,
    module_time_date_stamp: u32,
) -> Result<Arc<PdbInfo>> {
    let module_file_name = if let Some(module_file_name) = module_name.file_name() {
        module_file_name
    } else {
        return Err(anyhow!("No file name for {}", module_name.display()));
    };
    let pdb_prefix = pdb_path_get();
    let pdb_path = Path::new(pdb_prefix.as_str())
        .join(module_file_name)
        .with_extension("pdb");

    let file = File::open(pdb_path.as_path())
        .with_context(|| format!("Failed to open {}", pdb_path.display()))?;
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

    let mut file_name_map = HashMap::with_capacity(128);
    let mut modules_files_vec = Vec::with_capacity(16);
    let mut functions_map = BTreeMap::with_capacity(32);
    while let Some(module) = modules.next()? {
        let module_info = match pdb.module_info(&module)? {
            Some(info) => info,
            None => {
                continue;
            }
        };
        let program = module_info.line_program()?;
        let mut symbols = module_info.symbols()?;
        let mut files_map = HashMap::new();
        let mut files = program.files();
        while let Some(file_info) = files.next()? {
            let file_name = match file_info.name.to_raw_string(&string_table) {
                Err(e) => {
                    let s = format!("{e}");
                    error!(s);
                    s
                }
                Ok(rs) => format!("{rs}"),
            };
            let _ = file_name_map.try_insert(file_info.name, file_name);
            files_map.insert(
                file_info.name,
                FileInfo {
                    name: file_info.name,
                },
            );
        }
        let module_index = modules_files_vec.len() as u32;
        modules_files_vec.push(ModuleInfo {
            module_name: format!("{}", module.module_name()),
            object_file_name: format!("{}", module.object_file_name()),
            files_map,
        });

        while let Some(symbol) = symbols.next()? {
            match symbol.parse() {
                Ok(data) => {
                    if let SymbolData::Procedure(proc) = data {
                        if let Some(proc_rva) = proc.offset.to_rva(&address_map) {
                            let mut line_map = BTreeMap::new();
                            let mut lines = program.lines_for_symbol(proc.offset);
                            while let Some(line_info) = lines.next()? {
                                if let Some(rva) = line_info.offset.to_rva(&address_map) {
                                    let file_info =
                                        match program.get_file_info(line_info.file_index) {
                                            Err(e) => {
                                                error!("{e}");
                                                continue;
                                            }
                                            Ok(file_info) => file_info,
                                        };
                                    line_map.insert(
                                        rva.0,
                                        LineInfo {
                                            rva: rva.0,
                                            length: line_info.length,
                                            module_index,
                                            file_name: file_info.name,
                                            line_start: line_info.line_start,
                                            line_end: line_info.line_end,
                                            column_start: line_info.column_start,
                                            column_end: line_info.column_end,
                                        },
                                    );
                                }
                            }
                            functions_map.insert(
                                proc_rva.0,
                                ProcedureInfo {
                                    name: format!("{}", proc.name),
                                    rva: proc_rva.0,
                                    len: proc.len,
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

    Ok(Arc::new(PdbInfo {
        file_name_map,
        modules_files_vec,
        functions_map,
    }))
}

#[cfg(test)]
mod tests {
    use crate::process_modules::get_image_info_from_file;
    use std::path::Path;

    #[test]
    fn get_pdb_info_from_pdb_file() {
        let out_dir = env!("CARGO_MANIFEST_DIR");
        let pkg_name = env!("CARGO_PKG_NAME");
        let module_info = get_image_info_from_file(Path::new(
            format!("{out_dir}\\target\\debug\\{pkg_name}.exe").as_str(),
        ))
        .unwrap();
        super::pdb_path_set(format!("{out_dir}\\target\\debug").as_str());
        let r = super::get_pdb_info_from_pdb_file(
            &Path::new(format!("{pkg_name}.exe").as_str()),
            module_info.1,
        )
        .unwrap();
        println!("{:?}", r.get_location_info_by_offset(0x2b6168));
    }

    #[test]
    fn get_pdb_info_for_module() {
        let out_dir = env!("CARGO_MANIFEST_DIR");
        let pkg_name = env!("CARGO_PKG_NAME");
        let module_info = get_image_info_from_file(Path::new(
            format!("{out_dir}\\target\\debug\\{pkg_name}.exe").as_str(),
        ))
        .unwrap();
        super::pdb_path_set(format!("{out_dir}\\target\\debug").as_str());
        let location_info = super::get_location_info(
            &Path::new(format!("{pkg_name}.exe").as_str()),
            module_info.1,
            0x2b6168,
        )
        .unwrap();
        println!("{location_info:?}");
    }
}
