use parking_lot::FairMutex;
use once_cell::sync::Lazy;


static PDB_PATH: Lazy<FairMutex<String>> = Lazy::new(|| {
    FairMutex::new(String::new())
});

pub fn pdb_path_set(path: &str) {
    *PDB_PATH.lock() = path.to_string();
}