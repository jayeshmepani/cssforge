use crate::{engine::unified_diff, model::OutputMode};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct OutputOptions {
    pub mode: OutputMode,
    pub root: PathBuf,
    pub out_dir: Option<PathBuf>,
    pub suffix: String,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self {
            mode: OutputMode::DryRun,
            root: PathBuf::from("."),
            out_dir: None,
            suffix: ".modern.css".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub mode: OutputMode,
    pub source: PathBuf,
    pub written: Option<PathBuf>,
    pub backup: Option<PathBuf>,
    pub stdout: Option<String>,
    pub message: String,
}

pub fn write_result(
    path: &Path,
    original: &str,
    transformed: &str,
    options: &OutputOptions,
) -> Result<WriteResult> {
    match options.mode {
        OutputMode::DryRun => Ok(WriteResult {
            mode: options.mode,
            source: path.to_path_buf(),
            written: None,
            backup: None,
            stdout: None,
            message: "dry run: no file written".into(),
        }),
        OutputMode::NewFile => {
            let target = modern_path(path, &options.suffix);
            write_atomic(&target, transformed)?;
            Ok(written(
                options.mode,
                path,
                target,
                None,
                "modernized file written",
            ))
        }
        OutputMode::OutDir => {
            let base = options
                .out_dir
                .clone()
                .unwrap_or_else(|| options.root.join("cssforge-out"));
            let relative = path
                .strip_prefix(&options.root)
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    path.file_name()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("styles.css"))
                });
            let target = base.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            write_atomic(&target, transformed)?;
            Ok(written(
                options.mode,
                path,
                target,
                None,
                "file written to output directory",
            ))
        }
        OutputMode::OverwriteWithBackup => {
            let backup = backup_path(path);
            fs::copy(path, &backup)
                .with_context(|| format!("failed to create backup {}", backup.display()))?;
            write_atomic(path, transformed)?;
            Ok(written(
                options.mode,
                path,
                path.to_path_buf(),
                Some(backup),
                "source overwritten after backup",
            ))
        }
        OutputMode::Overwrite => {
            write_atomic(path, transformed)?;
            Ok(written(
                options.mode,
                path,
                path.to_path_buf(),
                None,
                "source overwritten",
            ))
        }
        OutputMode::Patch => {
            let target = patch_path(path);
            let diff = unified_diff(
                original,
                transformed,
                &path.display().to_string(),
                &format!("{}.modern", path.display()),
            );
            write_atomic(&target, &diff)?;
            Ok(written(
                options.mode,
                path,
                target,
                None,
                "unified patch written",
            ))
        }
        OutputMode::Stdout => Ok(WriteResult {
            mode: options.mode,
            source: path.to_path_buf(),
            written: None,
            backup: None,
            stdout: Some(transformed.to_string()),
            message: "transformed CSS returned for stdout".into(),
        }),
    }
}

fn written(
    mode: OutputMode,
    source: &Path,
    target: PathBuf,
    backup: Option<PathBuf>,
    message: &str,
) -> WriteResult {
    WriteResult {
        mode,
        source: source.to_path_buf(),
        written: Some(target),
        backup,
        stdout: None,
        message: message.into(),
    }
}

fn modern_path(path: &Path, suffix: &str) -> PathBuf {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("styles");
    path.with_file_name(format!("{stem}{suffix}"))
}

fn backup_path(path: &Path) -> PathBuf {
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("styles.css");
    path.with_file_name(format!("{file}.bak"))
}

fn patch_path(path: &Path) -> PathBuf {
    let file = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("styles.css");
    path.with_file_name(format!("{file}.patch"))
}

fn write_atomic(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("output.css");
    let tmp = path.with_file_name(format!(".{name}.cssforge-tmp-{}", std::process::id()));
    {
        let mut file = fs::File::create(&tmp)
            .with_context(|| format!("failed to create temporary file {}", tmp.display()))?;
        file.write_all(content.as_bytes())
            .with_context(|| format!("failed to write temporary file {}", tmp.display()))?;
        file.sync_all()
            .with_context(|| format!("failed to sync {}", tmp.display()))?;
    }

    #[cfg(windows)]
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("failed to replace {}", path.display()))?;
    }

    fs::rename(&tmp, path)
        .with_context(|| format!("failed to move {} to {}", tmp.display(), path.display()))?;
    Ok(())
}
