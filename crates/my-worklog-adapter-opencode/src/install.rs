use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use thiserror::Error;

use crate::plugin_template::plugin_template;
use crate::tool_templates::tool_templates;

#[derive(Debug, Clone)]
pub struct InstallOptions {
    pub target_dir: PathBuf,
    pub worklog_home: PathBuf,
    pub dry_run: bool,
    pub force: bool,
    pub include_tools: bool,
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub files: Vec<PlannedFile>,
}

#[derive(Debug, Clone)]
pub struct InstallReport {
    pub files: Vec<PathBuf>,
    pub backups: Vec<PathBuf>,
    pub dry_run: bool,
}

#[derive(Debug, Error)]
pub enum OpenCodeInstallError {
    #[error("refusing to overwrite existing file without --force: {0}")]
    ExistingFile(PathBuf),
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl InstallPlan {
    pub fn build(options: &InstallOptions) -> Self {
        let mut files = vec![PlannedFile {
            path: options.target_dir.join("plugins").join("my-worklog.ts"),
            contents: plugin_template(&options.worklog_home),
        }];
        if options.include_tools {
            files.extend(tool_templates(&options.target_dir));
        }
        Self { files }
    }

    pub fn apply(&self, options: &InstallOptions) -> Result<InstallReport, OpenCodeInstallError> {
        if options.dry_run {
            return Ok(InstallReport {
                files: self.files.iter().map(|file| file.path.clone()).collect(),
                backups: Vec::new(),
                dry_run: true,
            });
        }
        let mut written = Vec::with_capacity(self.files.len());
        let mut backups = Vec::new();
        for file in &self.files {
            if file.path.exists() && !options.force {
                return Err(OpenCodeInstallError::ExistingFile(file.path.clone()));
            }
            if let Some(parent) = file.path.parent() {
                fs::create_dir_all(parent).map_err(|source| OpenCodeInstallError::Io {
                    path: parent.to_path_buf(),
                    source,
                })?;
            }
            if file.path.exists() {
                let backup = backup_path(&file.path);
                fs::copy(&file.path, &backup).map_err(|source| OpenCodeInstallError::Io {
                    path: backup.clone(),
                    source,
                })?;
                backups.push(backup);
            }
            fs::write(&file.path, &file.contents).map_err(|source| OpenCodeInstallError::Io {
                path: file.path.clone(),
                source,
            })?;
            written.push(file.path.clone());
        }
        Ok(InstallReport {
            files: written,
            backups,
            dry_run: false,
        })
    }
}

pub fn default_project_target(project: &Path) -> PathBuf {
    project.join(".opencode")
}

fn backup_path(path: &Path) -> PathBuf {
    let stamp = Utc::now().format("%Y%m%d%H%M%S");
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("my-worklog.ts");
    path.with_file_name(format!("{name}.bak.{stamp}"))
}
