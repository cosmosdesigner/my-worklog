use std::env;
use std::path::PathBuf;

use directories::BaseDirs;

pub fn config_dir_from_env() -> Option<PathBuf> {
    env::var_os("OPENCODE_CONFIG_DIR").map(PathBuf::from)
}

pub fn default_data_dir() -> Option<PathBuf> {
    let dirs = BaseDirs::new()?;
    let xdg_dir = dirs.home_dir().join(".local/share/opencode");
    if xdg_dir.exists() {
        return Some(xdg_dir);
    }
    Some(dirs.data_dir().join("opencode"))
}

pub fn default_db_path() -> Option<PathBuf> {
    env::var_os("OPENCODE_DB")
        .map(PathBuf::from)
        .or_else(|| default_data_dir().map(|dir| dir.join("opencode.db")))
}
