use std::path::{Path, PathBuf};

pub fn nearest_project_root(path: &Path) -> Option<PathBuf> {
    path.ancestors()
        .find(|candidate| candidate.join(".git").exists())
        .map(Path::to_path_buf)
}
